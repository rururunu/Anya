use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde_json::Value;

use super::context::ToolContext;
use super::error::ToolError;
use super::Tool;

fn normalize_tool_name(name: &str) -> &str {
    match name {
        // File edits
        "write_to_file" | "create_file" | "Write" | "write" => "write_file",
        "replace_content" | "replace_file_content" | "edit_file" | "StrReplace" | "str_replace" => {
            "replace_in_file"
        }
        "Read" | "read" => "read_file",
        // Shell
        "run_command" | "execute_command" | "execute_shell" | "exec_command" | "Shell"
        | "shell" => "run_shell",
        // Search
        "Grep" | "grep" | "rg" => "search_files",
        "Glob" | "glob" => "find_files",
        // Task / ask aliases
        "todo_write" | "TodoWrite" | "todoWrite" | "update_task_list" => "update_tasks",
        "AskQuestion" | "ask_question" | "AskUser" => "ask_user",
        _ => name,
    }
}

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    dynamic: RwLock<HashMap<String, Arc<dyn Tool>>>,
    /// Cached model-facing schemas; invalidated on register/unregister.
    schema_cache: RwLock<Option<Vec<Value>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            dynamic: RwLock::new(HashMap::new()),
            schema_cache: RwLock::new(None),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
        self.invalidate_schema_cache();
    }

    pub fn register_dynamic(&self, tool: Arc<dyn Tool>) {
        if let Ok(mut guard) = self.dynamic.write() {
            guard.insert(tool.name().to_string(), tool);
        }
        self.invalidate_schema_cache();
    }

    pub fn unregister_dynamic_prefix(&self, prefix: &str) {
        if let Ok(mut guard) = self.dynamic.write() {
            guard.retain(|name, _| !name.starts_with(prefix));
        }
        self.invalidate_schema_cache();
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        let name = normalize_tool_name(name);
        if let Ok(guard) = self.dynamic.read() {
            if let Some(tool) = guard.get(name) {
                return Some(Arc::clone(tool));
            }
        }
        self.tools.get(name).cloned()
    }

    pub fn schemas(&self) -> Vec<Value> {
        if let Ok(guard) = self.schema_cache.read() {
            if let Some(cached) = guard.as_ref() {
                return cached.clone();
            }
        }
        let built = self.build_schemas();
        if let Ok(mut guard) = self.schema_cache.write() {
            *guard = Some(built.clone());
        }
        built
    }

    /// Shared-pointer view of schemas for hot agent loops (cheap to clone).
    pub fn schemas_arc(&self) -> Arc<[Value]> {
        Arc::from(self.schemas())
    }

    fn build_schemas(&self) -> Vec<Value> {
        let mut names: Vec<String> = self.tools.keys().cloned().collect();
        if let Ok(guard) = self.dynamic.read() {
            names.extend(guard.keys().cloned());
        }
        names.sort();
        names.dedup();
        names
            .into_iter()
            .filter_map(|name| self.get(&name))
            .filter(|tool| tool.available())
            .map(|tool| tool.schema())
            .collect()
    }

    fn invalidate_schema_cache(&self) {
        if let Ok(mut guard) = self.schema_cache.write() {
            *guard = None;
        }
    }

    pub fn execute(&self, ctx: &ToolContext, name: &str, args: Value) -> Result<String, ToolError> {
        let tool = self.prepare_execution(ctx, name, &args)?;
        tool.execute(ctx, args)
    }

    pub(crate) fn prepare_execution(
        &self,
        ctx: &ToolContext,
        name: &str,
        args: &Value,
    ) -> Result<Arc<dyn Tool>, ToolError> {
        let name = normalize_tool_name(name);
        crate::core::rules::RuleEngine::authorize_tool(name, args)?;
        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::new(format!("unknown tool: {name}")))?;
        crate::core::tools::plan_mode::shared_plan_mode_store().authorize(
            ctx.root_session_id(),
            tool.name(),
            tool.read_only(),
        )?;
        let preview = tool.preview(ctx, args)?;
        crate::core::tools::tool_approval::shared_tool_approval_store().authorize(
            ctx,
            tool.as_ref(),
            args,
            preview.clone(),
        )?;
        let current_preview = tool.preview(ctx, args)?;
        if current_preview != preview {
            return Err(ToolError::new(
                "file changed after the edit preview was created; inspect the latest content and retry",
            ));
        }
        if let Some(preview) = &current_preview {
            crate::core::checkpoint::shared_checkpoint_store().snapshot_preview(
                ctx.root_session_id(),
                &ctx.workspace_root,
                preview,
            )?;
        }
        Ok(tool)
    }

    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.tools.keys().cloned().collect();
        if let Ok(guard) = self.dynamic.read() {
            names.extend(guard.keys().cloned());
        }
        names.sort();
        names.dedup();
        names
    }

    pub fn filter_read_only(&self) -> ToolRegistry {
        let mut filtered = ToolRegistry::new();
        for name in self.names() {
            if let Some(tool) = self.get(&name) {
                if tool.read_only() {
                    filtered.register(tool);
                }
            }
        }
        filtered
    }

    /// Ask / question-only mode: read-only tools plus interaction/orchestration
    /// tools (`ask_user`, `update_tasks`) so the model can still clarify and
    /// maintain a task list without writing files.
    pub fn filter_for_ask_mode(&self) -> ToolRegistry {
        let mut filtered = ToolRegistry::new();
        for name in self.names() {
            if let Some(tool) = self.get(&name) {
                if tool.read_only()
                    || matches!(name.as_str(), "update_tasks" | "ask_user" | "todo_write")
                {
                    filtered.register(tool);
                }
            }
        }
        filtered
    }

    /// Child agents receive execution tools but never delegation tools.
    pub fn filter_for_subagent(&self, read_only: bool) -> ToolRegistry {
        let mut filtered = ToolRegistry::new();
        for name in self.names() {
            if crate::core::tools::agent::is_async_runtime_tool(&name) {
                continue;
            }
            if let Some(tool) = self.get(&name) {
                if !read_only || tool.read_only() {
                    filtered.register(tool);
                }
            }
        }
        filtered
    }

    /// Tools exposed while plan mode is active: read-only tools plus the
    /// planning / interaction tools plan mode explicitly allows.
    pub fn filter_for_plan_mode(&self) -> ToolRegistry {
        let mut filtered = ToolRegistry::new();
        for name in self.names() {
            if let Some(tool) = self.get(&name) {
                if tool.read_only()
                    || matches!(
                        name.as_str(),
                        "update_tasks" | "ask_user" | "complete_plan_step" | "todo_write"
                    )
                {
                    filtered.register(tool);
                }
            }
        }
        filtered
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubTool {
        name: &'static str,
        read_only: bool,
    }

    impl Tool for StubTool {
        fn name(&self) -> &str {
            self.name
        }

        fn description(&self) -> &str {
            "test tool"
        }

        fn parameters_schema(&self) -> Value {
            serde_json::json!({ "type": "object" })
        }

        fn read_only(&self) -> bool {
            self.read_only
        }

        fn execute(&self, _ctx: &ToolContext, _args: Value) -> Result<String, ToolError> {
            Ok(String::new())
        }
    }

    #[test]
    fn subagent_registry_excludes_all_delegation_tools() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(StubTool {
            name: "read_file",
            read_only: true,
        }));
        registry.register(Arc::new(StubTool {
            name: "write_file",
            read_only: false,
        }));
        for name in [
            "run_subagent",
            "run_parallel_subagents",
            "run_skill",
            "explore_codebase",
            "review_code",
        ] {
            registry.register(Arc::new(StubTool {
                name,
                read_only: true,
            }));
        }

        let writable = registry.filter_for_subagent(false).names();
        assert_eq!(writable, vec!["read_file", "write_file"]);

        let read_only = registry.filter_for_subagent(true).names();
        assert_eq!(read_only, vec!["read_file"]);
    }

    #[test]
    fn filter_for_ask_mode_keeps_interaction_tools() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(StubTool {
            name: "read_file",
            read_only: true,
        }));
        registry.register(Arc::new(StubTool {
            name: "write_file",
            read_only: false,
        }));
        registry.register(Arc::new(StubTool {
            name: "ask_user",
            read_only: false,
        }));
        registry.register(Arc::new(StubTool {
            name: "update_tasks",
            read_only: false,
        }));

        let filtered = registry.filter_for_ask_mode();
        let names = filtered.names();
        assert!(names.contains(&"read_file".to_string()));
        assert!(names.contains(&"ask_user".to_string()));
        assert!(names.contains(&"update_tasks".to_string()));
        assert!(!names.contains(&"write_file".to_string()));
    }
}
