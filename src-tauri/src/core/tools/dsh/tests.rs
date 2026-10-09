use super::*;
use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::event::EventBus;
use crate::core::tools::context::{AskStore, PathPermissionStore};
use std::sync::{atomic::AtomicBool, Mutex};

struct Events;
impl EventBus for Events {
    fn emit(&self, _: BusEvent) {}
}
fn context() -> ToolContext {
    let root = std::env::temp_dir().join(format!("anya-dsh-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let ctx = ToolContext {
        workspace_root: root.clone(),
        request_context: Default::default(),
        session_id: uuid::Uuid::new_v4().to_string(),
        assistant_message_id: "test".into(),
        conversation: Arc::new(ConversationManager::new(root.join("test.db"))),
        event_bus: Arc::new(Events),
        tasks: Arc::new(Mutex::new(vec![])),
        ask_store: Arc::new(AskStore::new()),
        path_permission_store: Arc::new(PathPermissionStore::new()),
        registry: None,
        provider: None,
        subagent_depth: 0,
        max_subagent_depth: 1,
        subagent_id: None,
        parent_activity_id: None,
        app_handle: None,
        cancelled: Arc::new(AtomicBool::new(false)),
    };
    crate::core::tools::tool_approval::shared_tool_approval_store().set_session_mode(
        &ctx.session_id,
        Some(crate::models::settings::ToolApprovalMode::Auto),
    );
    ctx
}

#[test]
fn existing_files_require_read_and_reject_external_changes() {
    let ctx = context();
    let path = ctx.workspace_root.join("sample.txt");
    std::fs::write(&path, "old\n").unwrap();
    let edit = json!({"file_path": "sample.txt", "old_string": "old", "new_string": "new"});
    assert!(files::mutate(&ctx, &edit, true)
        .unwrap_err()
        .to_string()
        .contains("Read"));
    assert!(files::read(&ctx, &json!({"file_path": "sample.txt"}))
        .unwrap()
        .contains("1: old"));
    std::fs::write(&path, "external\n").unwrap();
    assert!(files::mutate(&ctx, &edit, true)
        .unwrap_err()
        .to_string()
        .contains("changed"));
    assert_eq!(std::fs::read_to_string(path).unwrap(), "external\n");
}

#[test]
fn literal_edit_rejects_ambiguity_and_observes_own_writes() {
    let ctx = context();
    files::mutate(
        &ctx,
        &json!({"file_path": "new.txt", "content": "猫 猫"}),
        false,
    )
    .unwrap();
    let mut args = json!({"file_path": "new.txt", "old_string": "猫", "new_string": "犬"});
    assert!(files::mutate(&ctx, &args, true)
        .unwrap_err()
        .to_string()
        .contains("2 times"));
    args["replace_all"] = json!(true);
    files::mutate(&ctx, &args, true).unwrap();
    assert_eq!(
        std::fs::read_to_string(ctx.workspace_root.join("new.txt")).unwrap(),
        "犬 犬"
    );
    let mut other = context();
    other.workspace_root = ctx.workspace_root.clone();
    assert!(files::mutate(&other, &json!({"file_path":"new.txt", "content":""}), false).is_err());
    clear_session(&ctx.session_id);
    assert!(files::mutate(&ctx, &json!({"file_path":"new.txt", "content":""}), false).is_err());
}

#[test]
fn read_windows_empty_files_and_long_lines() {
    let ctx = context();
    std::fs::write(ctx.workspace_root.join("lines.txt"), "甲\r\n乙\r\n丙\r\n").unwrap();
    let result = files::read(
        &ctx,
        &json!({"file_path":"lines.txt", "offset":2, "limit":1}),
    )
    .unwrap();
    assert!(result.contains("2: 乙"));
    assert!(result.contains("of 3. Use offset=3"));
    assert!(!result.contains("1: 甲"));
    std::fs::write(ctx.workspace_root.join("long.txt"), "x".repeat(2_000_000)).unwrap();
    let result = files::read(&ctx, &json!({"file_path":"long.txt"})).unwrap();
    assert!(result.len() < 3000);
    assert!(result.contains("total 1 lines"));
    std::fs::write(ctx.workspace_root.join("empty.txt"), "").unwrap();
    assert!(files::read(&ctx, &json!({"file_path":"empty.txt"}))
        .unwrap()
        .contains("total 0 lines"));
    assert!(files::read(&ctx, &json!({"file_path":"empty.txt","offset":2})).is_err());
}

#[test]
fn schemas_reject_legacy_arguments_and_prompt_filters_platform() {
    let read = contracts()
        .iter()
        .find(|s| s["function"]["name"] == "read")
        .unwrap();
    assert!(validate(
        &read["function"]["parameters"],
        &json!({"path":"x"}),
        "args"
    )
    .is_err());
    // Official schemas intentionally allow extra properties when unspecified.
    assert!(validate(
        &read["function"]["parameters"],
        &json!({"file_path":"x", "unknown":true}),
        "args"
    )
    .is_ok());
    let prompt = tool_prompt(&[read.clone()]);
    assert!(prompt.contains("read tool"));
    assert!(!prompt.contains("pwsh"));
    assert!(!prompt.contains("write tool"));
}

struct BackTool(&'static str);
impl Tool for BackTool {
    fn name(&self) -> &str {
        self.0
    }
    fn description(&self) -> &str {
        "test capability"
    }
    fn parameters_schema(&self) -> Value {
        json!({"type":"object"})
    }
    fn execute(&self, _: &ToolContext, _: Value) -> Result<String, ToolError> {
        Ok("test".into())
    }
}
fn manager() -> Arc<ToolManager> {
    let mut registry = ToolRegistry::new();
    for name in [
        "read_file",
        "write_file",
        "run_shell",
        "update_tasks",
        "ask_user",
        "load_skill",
        "web_search",
        "browser_read",
        "run_subagent",
        "present",
    ] {
        registry.register(Arc::new(BackTool(name)));
    }
    Arc::new(ToolManager::new(registry))
}

#[test]
fn dsh_registry_is_separate_preserves_exact_schemas_and_plan_gate() {
    let old = manager();
    let dsh = old.dsh_contract();
    assert!(!old.is_dsh());
    assert!(dsh.is_dsh());
    assert!(old.registry().names().contains(&"run_shell".into()));
    assert!(!dsh.registry().names().contains(&"run_shell".into()));
    for schema in dsh.schemas() {
        assert!(
            contracts().contains(&schema),
            "schema differs from upstream: {schema}"
        );
    }
    let ctx = context();
    let store = crate::core::tools::plan_mode::shared_plan_mode_store();
    store.set_active(&ctx.session_id, true);
    let error = dsh
        .dispatch(
            &ctx,
            "write",
            json!({"file_path":"blocked.txt", "content":"x"}),
        )
        .unwrap_err();
    assert!(error.to_string().contains("exit_plan_mode"));
    assert!(!ctx.workspace_root.join("blocked.txt").exists());
    store.set_active(&ctx.session_id, false);
    assert!(dsh
        .ask_mode()
        .registry()
        .names()
        .contains(&"ask_user_question".into()));
    assert!(!dsh.ask_mode().registry().names().contains(&"write".into()));
}

#[test]
fn dsh_keeps_available_plugin_and_mcp_tools() {
    let backing = manager();
    backing.register_dynamic(Arc::new(BackTool("plugin_computer-use__see")));
    backing.register_dynamic(Arc::new(BackTool("mcp__example__search")));
    backing.register_dynamic(Arc::new(BackTool("manage_plugin")));
    backing.register_dynamic(Arc::new(BackTool("unrelated_dynamic_tool")));
    let dsh = backing.dsh_contract();
    let names = dsh.registry().names();
    assert!(names.contains(&"plugin_computer-use__see".into()));
    assert!(names.contains(&"mcp__example__search".into()));
    assert!(names.contains(&"manage_plugin".into()));
    assert!(!names.contains(&"unrelated_dynamic_tool".into()));
    assert!(dsh
        .schemas()
        .iter()
        .any(|schema| { schema["function"]["name"] == "plugin_computer-use__see" }));
    assert!(dsh.registry().get("mcp__example__search").is_some());
}

#[test]
fn dsh_keeps_generate_image_as_the_only_image_mode_tool() {
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(BackTool("generate_image")));
    registry.register(Arc::new(BackTool("run_shell")));
    let image_tools = Arc::new(ToolManager::new(registry).image_mode());
    let dsh = image_tools.dsh_contract();
    assert!(dsh.is_dsh());
    assert_eq!(dsh.registry().names(), vec!["generate_image"]);
    assert_eq!(dsh.schemas()[0]["function"]["name"], "generate_image");
}

#[test]
fn present_verifies_all_files_and_returns_replayable_original_references() {
    let ctx = context();
    std::fs::write(ctx.workspace_root.join("报告.pptx"), b"original").unwrap();
    let tools = manager().dsh_contract();
    assert!(tools
        .schemas()
        .iter()
        .any(|schema| schema["function"]["name"] == "present"));
    assert!(tool_prompt(&tools.schemas()).contains("Each presented file adds a card"));
    let result = tools
        .dispatch(
            &ctx,
            "present",
            json!({"files":[{"path":"报告.pptx","description":"演示文稿"}]}),
        )
        .unwrap();
    let value: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["files"][0]["name"], "报告.pptx");
    assert_eq!(value["files"][0]["size"], 8);
    assert!(
        std::path::Path::new(value["files"][0]["absolutePath"].as_str().unwrap()).is_absolute()
    );
    assert!(!value["files"][0]["absolutePath"]
        .as_str()
        .unwrap()
        .starts_with(r"\\?\"));
    assert_eq!(
        std::fs::read(ctx.workspace_root.join("报告.pptx")).unwrap(),
        b"original"
    );
    for args in [
        json!({"files":[]}),
        json!({"files":[{"path":"."}]}),
        json!({"files":[{"path":"报告.pptx"},{"path":"missing.pdf"}]}),
        json!({"files":[{"path":"报告.pptx","description":123}]}),
    ] {
        assert!(tools.dispatch(&ctx, "present", args).is_err());
    }
}

#[test]
fn shell_timeout_promotes_and_job_output_is_incremental_and_session_scoped() {
    let ctx = context();
    let (shell_name, command) = if cfg!(windows) {
        (
            "pwsh",
            "Write-Output first; Start-Sleep -Milliseconds 200; Write-Output second",
        )
    } else {
        ("bash", "printf 'first\\n'; sleep 0.2; printf 'second\\n'")
    };
    let result = shell::execute(
        &ctx,
        shell_name,
        &json!({"command":command, "description":"test background continuation", "timeoutMs":1}),
    )
    .unwrap();
    assert!(result.contains("moved to the background"));
    let id = result
        .split("[job_id: ")
        .nth(1)
        .unwrap()
        .split(']')
        .next()
        .unwrap();
    assert!(shell::execute(&context(), "job_output", &json!({"job_id":id})).is_err());
    let completed = shell::execute(
        &ctx,
        "job_output",
        &json!({"job_id":id,"wait":true,"timeout_ms":20000}),
    )
    .unwrap();
    assert!(completed.contains("completed"));
    let combined = format!("{result}{completed}");
    assert_eq!(combined.matches("first").count(), 1);
    assert_eq!(combined.matches("second").count(), 1);
    let again = shell::execute(&ctx, "job_output", &json!({"job_id":id})).unwrap();
    assert!(again.contains("no new output"));
    clear_session(&ctx.session_id);
    assert!(shell::execute(&ctx, "job_output", &json!({"job_id":id})).is_err());
}

#[test]
fn glob_and_grep_keep_unicode_names_and_literal_windows_paths() {
    let ctx = context();
    std::fs::write(ctx.workspace_root.join("空 格.txt"), "alpha\n包含 中文\n").unwrap();
    let glob = search::execute(&ctx, "glob", &json!({"pattern":"*.txt"})).unwrap();
    assert!(glob.contains("空 格.txt"));
    let grep = search::execute(
        &ctx,
        "grep",
        &json!({"pattern":"包含 中文", "include":"*.txt"}),
    )
    .unwrap();
    assert!(grep.contains("Line 2: 包含 中文"));
}

struct AnswerEvents {
    store: Arc<AskStore>,
    label: &'static str,
}
impl EventBus for AnswerEvents {
    fn emit(&self, event: BusEvent) {
        if let BusEvent::AskUser { request_id, .. } = event {
            self.store.complete(
                &request_id,
                json!({"answers":[{"selected":[self.label]}]}).to_string(),
            );
        }
    }
}
#[test]
fn plan_review_stays_gated_until_explicit_approval() {
    let mut ctx = context();
    let store = crate::core::tools::plan_mode::shared_plan_mode_store();
    let contract = contracts()
        .iter()
        .find(|s| s["function"]["name"] == "exit_plan_mode")
        .unwrap();
    let tool = DshTool {
        schema: contract.clone(),
        backing: manager(),
    };
    store.set_active(&ctx.session_id, true);
    ctx.event_bus = Arc::new(AnswerEvents {
        store: ctx.ask_store.clone(),
        label: "Keep planning",
    });
    let rejected = tool
        .execute(&ctx, json!({"plan":"# Plan\nInspect the source."}))
        .unwrap();
    assert!(rejected.contains("not approved"));
    assert!(store.is_active(&ctx.session_id));
    ctx.event_bus = Arc::new(AnswerEvents {
        store: ctx.ask_store.clone(),
        label: "Approve",
    });
    let approved = tool
        .execute(&ctx, json!({"plan":"# Plan\nInspect the source."}))
        .unwrap();
    assert!(approved.starts_with("Plan approved"));
    assert!(!store.is_active(&ctx.session_id));
    assert!(tool.execute(&ctx, json!({"plan":"# Plan"})).is_err());
}

#[test]
fn read_image_scales_large_inputs_and_returns_actual_image_reference() {
    let ctx = context();
    let image = image::RgbImage::new(3000, 2);
    image
        .save(ctx.workspace_root.join("图片(原图).png"))
        .unwrap();
    let result = files::read_image(&ctx, &json!({"file_path":"图片(原图).png"})).unwrap();
    assert!(result.contains("downscaled from 3000x2"));
    let reference = result
        .split("![image](")
        .nth(1)
        .unwrap()
        .trim_end_matches(')');
    let stored = image::open(reference).unwrap();
    assert_eq!(stored.width(), 2048);
    std::fs::write(ctx.workspace_root.join("bad.png"), "not an image").unwrap();
    assert!(files::read_image(&ctx, &json!({"file_path":"bad.png"})).is_err());
}

struct RecordingProvider {
    model: &'static str,
    request: Mutex<Option<crate::core::runtime::ChatRequest>>,
}
#[async_trait::async_trait]
impl crate::core::ai::provider::AIProvider for RecordingProvider {
    fn id(&self) -> &'static str {
        "deepseek"
    }
    fn uses_dsh_tools(&self) -> bool {
        self.model.starts_with("deepseek-")
    }
    async fn stream(
        &self,
        request: crate::core::runtime::ChatRequest,
        tx: tokio::sync::mpsc::Sender<crate::core::runtime::StreamEvent>,
    ) -> Result<(), crate::core::ai::provider::ProviderError> {
        *self.request.lock().unwrap() = Some(request);
        let _ = tx
            .send(crate::core::runtime::StreamEvent::TurnComplete {
                content: "Hello".into(),
                reasoning: None,
                tool_calls: vec![],
                finish_reason: Some("stop".into()),
            })
            .await;
        Ok(())
    }
}

#[tokio::test]
async fn agent_selects_native_tools_only_for_deepseek_models() {
    use crate::core::chat::prompt::{PromptBuildInput, PromptBuilder, PromptPreferences};
    for model in ["deepseek-v4-pro", "gpt-5"] {
        let provider = Arc::new(RecordingProvider {
            model,
            request: Mutex::new(None),
        });
        let tools = manager();
        let ctx = context();
        let preferences = PromptPreferences::default();
        let builder = if model.starts_with("deepseek-") {
            PromptBuilder::build_dsh
        } else {
            PromptBuilder::build
        };
        let request = builder(PromptBuildInput {
            request_id: "r",
            session_id: &ctx.session_id,
            history: &[],
            context: &ctx.request_context,
            project_rules: None,
            recalled_memories: None,
            preferred_resources: None,
            provider: Some("deepseek".into()),
            preferences: &preferences,
        });
        let runner = crate::core::chat::agent::AgentRunner::new(provider.clone(), tools.clone());
        let (tx, _rx) = tokio::sync::mpsc::channel(64);
        runner
            .run(
                request,
                ctx.clone(),
                tx,
                ctx.cancelled.clone(),
                Arc::new(Mutex::new(std::collections::VecDeque::new())),
            )
            .await
            .unwrap();
        let captured = provider.request.lock().unwrap().clone().unwrap();
        let names: Vec<_> = captured
            .tools
            .iter()
            .filter_map(|s| s["function"]["name"].as_str())
            .collect();
        if model.starts_with("deepseek-") {
            assert!(names.contains(&"read"));
            assert!(!names.contains(&"read_file"));
        } else {
            assert!(names.contains(&"read_file"));
            assert!(!names.contains(&"read"));
        }
        assert!(!tools.is_dsh());
    }
}
