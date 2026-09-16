use std::sync::Arc;

use serde_json::Value;

use crate::core::runtime::{ChatRequest, Role};
use crate::core::tools::preview::ToolPreview;
use crate::core::tools::registry::ToolRegistry;
use crate::runtime::tool::{Tool, ToolContext, ToolError};

/// The only dispatch boundary exposed to the AI runtime.
pub struct ToolManager {
    registry: Arc<ToolRegistry>,
}

#[allow(dead_code)]
impl ToolManager {
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }

    pub(crate) fn from_registry(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }

    pub fn schemas(&self) -> Vec<Value> {
        self.registry.schemas()
    }

    pub fn schemas_arc(&self) -> std::sync::Arc<[Value]> {
        self.registry.schemas_arc()
    }

    /// Model-facing schemas for one request, derived from its mode.
    ///
    /// Defense in depth with `ChatService` tool-set selection:
    /// - Image mode for `session_id` → only `generate_image` (also set via
    ///   `tools.image_mode()` when the turn starts).
    /// - Plan mode active for `session_id` → read-only tools plus the
    ///   planning/interaction tools plan mode allows.
    /// - Question-only request (Answer/explain/review, Diagnose) → read-only
    ///   tools plus `ask_user` / `update_tasks`.
    /// - Otherwise → the full toolset.
    pub fn schemas_for_request(&self, request: &ChatRequest, session_id: &str) -> Arc<[Value]> {
        if crate::core::tools::image_mode::is_image_mode(session_id) {
            return self.registry.filter_for_image_mode().schemas_arc();
        }
        if crate::core::tools::plan_mode::shared_plan_mode_store().is_active(session_id) {
            return self.registry.filter_for_plan_mode().schemas_arc();
        }
        if is_question_only_request(request) {
            return self.ask_mode().schemas_arc();
        }
        self.registry.filter_without_save_plan().schemas_arc()
    }

    /// Keep a stable small core; extensions are loaded by search_tools and stay
    /// visible for the remainder of this turn. Mode filtering always runs first.
    pub fn focused_schemas(
        &self,
        request: &ChatRequest,
        session_id: &str,
        loaded: &std::collections::HashSet<String>,
    ) -> Arc<[Value]> {
        let schemas = self.schemas_for_request(request, session_id);
        if schemas.len() <= 32
            || !schemas
                .iter()
                .any(|s| s["function"]["name"] == "search_tools")
        {
            return schemas;
        }
        const CORE: &[&str] = &[
            "search_tools",
            "read_file",
            "list_folder",
            "find_files",
            "search_files",
            "write_file",
            "replace_in_file",
            "apply_patch",
            "run_shell",
            "read_shell_output",
            "wait_for_shell",
            "stop_shell",
            "ask_user",
            "update_tasks",
            "save_plan",
            "request_plan_mode",
            "generate_image",
            "load_skill",
        ];
        schemas
            .iter()
            .filter(|s| {
                s["function"]["name"]
                    .as_str()
                    .is_some_and(|name| CORE.contains(&name) || loaded.contains(name))
            })
            .cloned()
            .collect::<Vec<_>>()
            .into()
    }

    pub fn preview(
        &self,
        context: &ToolContext,
        name: &str,
        arguments: &Value,
    ) -> Option<ToolPreview> {
        self.registry
            .get(name)
            .and_then(|tool| tool.preview(context, arguments).ok().flatten())
    }

    pub fn dispatch(
        &self,
        context: &ToolContext,
        name: &str,
        arguments: Value,
    ) -> Result<String, ToolError> {
        self.registry.execute(context, name, arguments)
    }

    /// Prefer this from the agent loop: async tools avoid nested `block_on`,
    /// sync tools still run on the blocking pool (including approval waits).
    pub async fn dispatch_async(
        &self,
        context: &ToolContext,
        name: &str,
        arguments: Value,
    ) -> Result<String, ToolError> {
        context.ensure_not_cancelled()?;
        if crate::core::tools::agent::is_async_runtime_tool(name) {
            let registry = Arc::clone(&self.registry);
            let auth_ctx = context.clone();
            let auth_name = name.to_string();
            let auth_args = arguments.clone();
            let tool_name = tauri::async_runtime::spawn_blocking(move || {
                let tool = registry.prepare_execution(&auth_ctx, &auth_name, &auth_args)?;
                Ok::<String, ToolError>(tool.name().to_string())
            })
            .await
            .unwrap_or_else(|error| Err(ToolError::new(format!("tool task failed: {error}"))))?;
            let result =
                crate::core::tools::agent::execute_async_tool(&tool_name, context, arguments).await;
            context.ensure_not_cancelled()?;
            return result;
        }

        let registry = Arc::clone(&self.registry);
        let context = context.clone();
        let cancellation = Arc::clone(&context.cancelled);
        let name = name.to_string();
        let result = tauri::async_runtime::spawn_blocking(move || {
            registry.execute(&context, &name, arguments)
        })
        .await
        .unwrap_or_else(|error| Err(ToolError::new(format!("tool task failed: {error}"))));
        if cancellation.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(ToolError::cancelled());
        }
        result
    }

    /// MCP and other runtime adapters register tools through this method.
    pub fn register_dynamic(&self, tool: Arc<dyn Tool>) {
        self.registry.register_dynamic(tool);
    }

    pub fn names(&self) -> Vec<String> {
        self.registry.names()
    }

    pub fn read_only(&self) -> Self {
        Self::new(self.registry.filter_read_only())
    }

    pub fn ask_mode(&self) -> Self {
        Self::new(self.registry.filter_for_ask_mode())
    }

    pub fn image_mode(&self) -> Self {
        Self::new(self.registry.filter_for_image_mode())
    }

    /// Unknown or missing tools are treated as non-read-only so the agent stays serial.
    pub fn is_read_only(&self, name: &str) -> bool {
        self.registry
            .get(name)
            .map(|tool| tool.read_only())
            .unwrap_or(false)
    }

    pub fn registry(&self) -> Arc<ToolRegistry> {
        Arc::clone(&self.registry)
    }
}

/// True when the latest user message looks like a pure question
/// (Answer/explain/review or Diagnose): no change intent, and either
/// question-shaped or very short.
pub(crate) fn is_question_only_request(request: &ChatRequest) -> bool {
    if let Some(state) = request
        .messages
        .iter()
        .rev()
        .find(|message| message.id == "agent-state-current")
    {
        if let Some(intent) = state
            .content
            .lines()
            .nth(1)
            .and_then(|json| serde_json::from_str::<Value>(json).ok())
            .and_then(|value| value["question_only"].as_bool())
        {
            return intent;
        }
    }
    request
        .messages
        .iter()
        .rev()
        .filter(|message| message.role == Role::User && !is_internal_feedback(&message.id))
        .map(|message| message.content.trim())
        .find(|text| !matches!(*text, "继续" | "接着做" | "continue" | "Continue" | "go on"))
        .is_some_and(is_question_only_text)
}

pub(crate) fn is_internal_feedback(id: &str) -> bool {
    id.starts_with("agent-feedback-")
        || id.starts_with("agent-state-")
        || id.starts_with("compact-")
}

pub(crate) fn is_question_only_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let text = lower.trim();
    if text.is_empty() {
        return false;
    }
    // Polite analysis requests remain analysis even when they mention edits.
    let mut lead = text;
    for prefix in ["can you ", "could you ", "please ", "帮我", "为我", "请"] {
        if let Some(rest) = lead.strip_prefix(prefix) {
            lead = rest.trim();
        }
    }
    if [
        "explain",
        "review",
        "analyze",
        "diagnose",
        "compare",
        "summarize",
        "解释",
        "分析",
        "评估",
        "评审",
        "比较",
        "总结",
        "排查",
    ]
    .iter()
    .any(|prefix| lead.starts_with(prefix))
    {
        return true;
    }
    let explicit_action = [
        "帮我",
        "为我",
        "请直接",
        "并实现",
        "并修改",
        "please ",
        "can you ",
        "could you ",
    ]
    .iter()
    .any(|prefix| text.contains(prefix));
    let advice = ["有什么可以", "有什么能", "有哪些可以", "是否需要"]
        .iter()
        .any(|part| text.contains(part))
        || ["如何", "怎么", "为什么", "what ", "why ", "how ", "should "]
            .iter()
            .any(|prefix| text.starts_with(prefix));
    if advice && !explicit_action {
        return true;
    }
    const CHANGE_MARKERS: &[&str] = &[
        "fix",
        "change",
        "update",
        "create",
        "build",
        "implement",
        "add",
        "remove",
        "delete",
        "write",
        "edit",
        "refactor",
        "complete",
        "finish",
        "install",
        "deploy",
        "run",
        "test",
        "merge",
        "commit",
        "rename",
        "move",
        "optimize",
        "migrate",
        "generate",
        "checkout",
        "pull",
        "push",
        "rebase",
        "patch",
        "adjust",
        "tweak",
        "improve",
        "rewrite",
        "修复",
        "修改",
        "实现",
        "完成",
        "添加",
        "删除",
        "写入",
        "编辑",
        "构建",
        "优化",
        "重构",
        "拆分",
        "合并",
        "更新",
        "创建",
        "生成",
        "运行",
        "执行",
        "测试",
        "提交",
        "部署",
        "改成",
        "改一下",
        "改下",
        "帮我改",
        "帮我做",
        "调整",
        "换成",
        "加上",
        "去掉",
        "弄成",
        "做成",
        "实现一下",
        "继续改",
        "接着改",
    ];
    if CHANGE_MARKERS.iter().any(|marker| {
        if marker.is_ascii() {
            text.split(|c: char| !c.is_ascii_alphabetic())
                .any(|word| word == *marker)
        } else {
            text.contains(marker)
        }
    }) {
        return false;
    }
    const QUESTION_MARKERS: &[&str] = &[
        "explain",
        "review",
        "analyze",
        "summarize",
        "describe",
        "compare",
        "diagnose",
        "inspect",
        "what",
        "why",
        "how",
        "which",
        "who",
        "when",
        "where",
        "解释",
        "分析",
        "总结",
        "描述",
        "比较",
        "诊断",
        "排查",
        "介绍",
        "什么",
        "为什么",
        "如何",
        "哪些",
        "怎么",
        "原因",
        "是否",
        "吗？",
        "吗?",
    ];
    // Require an explicit question shape. Do not treat short imperative
    // messages as question-only — that stripped ask_user / update_tasks from
    // Agent turns and made the model fall back to plain text questions.
    let has_question_mark = text.contains('?') || text.contains('？');
    QUESTION_MARKERS.iter().any(|marker| text.contains(marker)) || has_question_mark
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use crate::core::chat::conversation_manager::ConversationManager;
    use crate::core::event::{BusEvent, EventBus};
    use crate::core::runtime::RequestContext;
    use crate::core::tools::context::{AskStore, PathPermissionStore};
    use crate::core::tools::error::ToolError;

    struct NullEventBus;
    impl EventBus for NullEventBus {
        fn emit(&self, _event: BusEvent) {}
    }

    struct EchoTool;

    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echo a value."
        }
        fn parameters_schema(&self) -> Value {
            serde_json::json!({ "type": "object" })
        }
        fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
            Ok(args["value"].as_str().unwrap_or_default().to_string())
        }
    }

    #[test]
    fn dynamic_tools_appear_in_manager_schemas() {
        let manager = ToolManager::new(ToolRegistry::new());
        manager.register_dynamic(Arc::new(EchoTool));
        assert_eq!(manager.names(), vec!["echo"]);
        assert_eq!(manager.schemas()[0]["function"]["name"], "echo");

        let db_path = std::env::temp_dir().join(format!("peek-v3-{}.db", uuid::Uuid::new_v4()));
        let context = ToolContext {
            workspace_root: std::env::temp_dir(),
            request_context: RequestContext::default(),
            session_id: "test".into(),
            assistant_message_id: "assistant".into(),
            conversation: Arc::new(ConversationManager::new(db_path.clone())),
            event_bus: Arc::new(NullEventBus),
            tasks: Arc::new(Mutex::new(Vec::new())),
            ask_store: Arc::new(AskStore::new()),
            path_permission_store: Arc::new(PathPermissionStore::new()),
            registry: Some(manager.registry()),
            provider: None,
            subagent_depth: 0,
            max_subagent_depth: 0,
            subagent_id: None,
            parent_activity_id: None,
            app_handle: None,
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        assert_eq!(
            manager
                .dispatch(&context, "echo", serde_json::json!({ "value": "ok" }))
                .unwrap(),
            "ok"
        );
        drop(context);
        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn short_imperative_is_not_question_only() {
        assert!(!is_question_only_text("帮我改一下样式"));
        assert!(!is_question_only_text("继续"));
        assert!(!is_question_only_text("fix the bug"));
    }

    #[test]
    fn explicit_questions_are_question_only() {
        assert!(is_question_only_text("这段代码是怎么工作的？"));
        assert!(is_question_only_text("What does this function do?"));
        assert!(is_question_only_text("有什么可以优化的地方吗？"));
        assert!(is_question_only_text("这个项目有什么可以优化的地方吗？"));
        assert!(is_question_only_text("Can you explain how to fix this?"));
        assert!(is_question_only_text("帮我分析这个测试为什么失败"));
        assert!(is_question_only_text("How can I fix this test?"));
        assert!(is_question_only_text("What is the runtime?"));
        assert!(!is_question_only_text("Can you fix this test?"));
        assert!(!is_question_only_text("为我优化这些问题。"));
        assert!(!is_question_only_text("FIX the bug"));
    }

    #[test]
    fn discovery_loads_requested_tools_without_bypassing_plan_mode() {
        struct NamedTool(String);
        impl Tool for NamedTool {
            fn name(&self) -> &str {
                &self.0
            }
            fn description(&self) -> &str {
                "extension"
            }
            fn parameters_schema(&self) -> Value {
                serde_json::json!({"type":"object"})
            }
            fn read_only(&self) -> bool {
                self.0 != "write_file"
            }
            fn execute(&self, _ctx: &ToolContext, _args: Value) -> Result<String, ToolError> {
                Ok("ok".into())
            }
        }
        let mut registry = ToolRegistry::new();
        for index in 0..40 {
            registry.register(Arc::new(NamedTool(format!("extension_{index}"))));
        }
        registry.register(Arc::new(NamedTool("search_tools".into())));
        registry.register(Arc::new(NamedTool("write_file".into())));
        let manager = ToolManager::new(registry);
        let sid = format!("discovery-test-{}", uuid::Uuid::new_v4());
        let request = ChatRequest {
            request_id: "r".into(),
            session_id: sid.clone(),
            messages: vec![],
            context: RequestContext::default(),
            provider: None,
            stream: true,
            tools: Arc::from([]),
            temperature: None,
            max_tokens: None,
        };
        let empty = std::collections::HashSet::new();
        let core = manager.focused_schemas(&request, &sid, &empty);
        assert!(core.len() < 10);
        let loaded = std::collections::HashSet::from(["extension_12".into(), "write_file".into()]);
        assert!(manager
            .focused_schemas(&request, &sid, &loaded)
            .iter()
            .any(|schema| schema["function"]["name"] == "extension_12"));
        let plans = crate::core::tools::plan_mode::shared_plan_mode_store();
        plans.set_active(&sid, true);
        assert!(!manager
            .focused_schemas(&request, &sid, &loaded)
            .iter()
            .any(|schema| schema["function"]["name"] == "write_file"));
        plans.set_active(&sid, false);
    }
}
