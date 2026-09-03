//! Headless AgentRunner eval harness (real runtime, not a Python toy loop).
//!
//! Tasks live under `eval/tasks/*.json`. Ablation flags toggle challenge /
//! mid-turn compact so harness changes can be measured on the same signal.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::core::ai::provider::{AIProvider, ProviderError};
use crate::core::chat::agent::AgentRunner;
use crate::core::chat::agent_loop::challenge;
use crate::core::chat::agent_loop::mid_turn_compact;
use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::event::{BusEvent, EventBus};
use crate::core::runtime::{
    ChatMessage, ChatRequest, MessageStatus, Role, StreamEvent, ToolCallPayload,
};
use crate::core::tools::builtin;
use crate::core::tools::context::{AskStore, PathPermissionStore, ToolContext};
use crate::core::tools::plan_mode::shared_plan_mode_store;
use crate::core::tools::registry::ToolRegistry;
use crate::core::tools::tool_approval::shared_tool_approval_store;
use crate::models::settings::ToolApprovalMode;
use crate::runtime::ToolManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalTask {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub script: Vec<EvalScriptTurn>,
    #[serde(default)]
    pub assertions: Vec<EvalAssertion>,
    #[serde(default, alias = "setup_files")]
    pub setup_files: Vec<EvalSetupFile>,
    #[serde(default, alias = "skip_unless_office")]
    pub skip_unless_office: Option<String>,
    /// Force plan-mode gate for this task (independent of CLI `--plan-mode`).
    #[serde(default, alias = "plan_mode")]
    pub plan_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalScriptTurn {
    #[serde(default)]
    pub content: String,
    #[serde(default, alias = "toolCalls")]
    pub tool_calls: Vec<EvalToolCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalToolCall {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalSetupFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EvalAssertion {
    #[serde(rename = "fileContains")]
    FileContains { path: String, text: String },
    #[serde(rename = "fileEquals")]
    FileEquals { path: String, text: String },
    #[serde(rename = "fileMissing")]
    FileMissing { path: String },
    #[serde(rename = "answerContains")]
    AnswerContains { text: String },
    #[serde(rename = "answerNotContains")]
    AnswerNotContains { text: String },
    #[serde(rename = "finishReason")]
    FinishReason { reason: String },
    #[serde(rename = "statusSeen")]
    StatusSeen { kind: String },
    #[serde(rename = "toolCalled")]
    ToolCalled { name: String },
}

#[derive(Debug, Clone, Default)]
pub struct EvalOptions {
    pub challenges: bool,
    pub compact: bool,
    pub plan_mode: bool,
    pub tasks_dir: PathBuf,
    pub results_dir: PathBuf,
    pub filter: Option<String>,
    pub seeds: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResult {
    pub id: String,
    pub passed: bool,
    pub skipped: bool,
    pub seed: u32,
    pub answer: String,
    pub finish_reason: Option<String>,
    pub statuses: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalReport {
    pub options: EvalOptionsReport,
    pub results: Vec<TaskResult>,
    pub passed: usize,
    pub failed: usize,
    pub pass_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalOptionsReport {
    pub challenges: bool,
    pub compact: bool,
    pub plan_mode: bool,
    pub seeds: u32,
}

struct ScriptedProvider {
    scripts: Mutex<Vec<ProviderTurn>>,
}

struct ProviderTurn {
    content: String,
    tool_calls: Vec<ToolCallPayload>,
}

impl ScriptedProvider {
    fn from_task(task: &EvalTask) -> Self {
        let scripts = task
            .script
            .iter()
            .enumerate()
            .map(|(idx, turn)| ProviderTurn {
                content: turn.content.clone(),
                tool_calls: turn
                    .tool_calls
                    .iter()
                    .enumerate()
                    .map(|(call_idx, call)| ToolCallPayload {
                        id: format!("call-{idx}-{call_idx}"),
                        name: call.name.clone(),
                        arguments: call.arguments.to_string(),
                        thought_signature: None,
                    })
                    .collect(),
            })
            .collect();
        Self {
            scripts: Mutex::new(scripts),
        }
    }

    fn take_turn(&self) -> ProviderTurn {
        self.scripts
            .lock()
            .ok()
            .and_then(|mut scripts| {
                if scripts.is_empty() {
                    None
                } else {
                    Some(scripts.remove(0))
                }
            })
            .unwrap_or(ProviderTurn {
                content: "done".into(),
                tool_calls: vec![],
            })
    }
}

#[async_trait]
impl AIProvider for ScriptedProvider {
    fn id(&self) -> &'static str {
        "scripted-eval"
    }

    async fn stream(
        &self,
        _request: ChatRequest,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        let turn = self.take_turn();
        if !turn.content.is_empty() {
            let _ = tx.send(StreamEvent::Delta(turn.content.clone())).await;
        }
        for call in &turn.tool_calls {
            let _ = tx.send(StreamEvent::ToolCall(call.clone())).await;
        }
        let _ = tx
            .send(StreamEvent::TurnComplete {
                content: turn.content,
                reasoning: None,
                tool_calls: turn.tool_calls,
                finish_reason: Some("stop".into()),
            })
            .await;
        Ok(())
    }
}

pub fn load_tasks(dir: &Path) -> Result<Vec<EvalTask>, String> {
    let mut tasks = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| format!("read tasks dir: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let raw = raw.strip_prefix('\u{feff}').unwrap_or(&raw);
        let task: EvalTask =
            serde_json::from_str(raw).map_err(|e| format!("{}: {e}", path.display()))?;
        tasks.push(task);
    }
    tasks.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(tasks)
}

pub async fn run_eval(options: EvalOptions) -> Result<EvalReport, String> {
    challenge::set_challenges_enabled(options.challenges);
    mid_turn_compact::set_compact_enabled(options.compact);
    shared_tool_approval_store().configure(ToolApprovalMode::AlwaysAllow);

    let tasks = load_tasks(&options.tasks_dir)?;
    let tasks: Vec<_> = tasks
        .into_iter()
        .filter(|task| {
            options
                .filter
                .as_ref()
                .map(|f| task.id.contains(f))
                .unwrap_or(true)
        })
        .collect();

    let seeds = options.seeds.max(1);
    let mut results = Vec::new();
    for seed in 0..seeds {
        for task in &tasks {
            results.push(run_one_task(task, &options, seed).await);
        }
    }

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;
    let pass_rate = if results.is_empty() {
        0.0
    } else {
        passed as f64 / results.len() as f64
    };

    let report = EvalReport {
        options: EvalOptionsReport {
            challenges: options.challenges,
            compact: options.compact,
            plan_mode: options.plan_mode,
            seeds,
        },
        results,
        passed,
        failed,
        pass_rate,
    };

    let _ = fs::create_dir_all(&options.results_dir);
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let out = options.results_dir.join(format!("eval-{stamp}.json"));
    let raw = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    fs::write(&out, raw).map_err(|e| e.to_string())?;
    eprintln!("wrote {}", out.display());

    // Restore defaults for subsequent tests in-process.
    challenge::set_challenges_enabled(true);
    mid_turn_compact::set_compact_enabled(true);

    Ok(report)
}

async fn run_one_task(task: &EvalTask, options: &EvalOptions, seed: u32) -> TaskResult {
    if let Some(app) = task.skip_unless_office.as_deref() {
        if !crate::core::office::office_app_available(app) {
            return TaskResult {
                id: task.id.clone(),
                passed: true,
                skipped: true,
                seed,
                answer: String::new(),
                finish_reason: None,
                statuses: Vec::new(),
                errors: vec![format!("skipped: {app} unavailable")],
            };
        }
    }

    let workspace = std::env::temp_dir().join(format!(
        "anya-eval-{}-{}-{}",
        task.id,
        seed,
        uuid::Uuid::new_v4()
    ));
    let _ = fs::create_dir_all(&workspace);
    for file in &task.setup_files {
        let path = workspace.join(&file.path);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, &file.content);
    }

    let db_path = workspace.join("eval.db");
    let conversation = Arc::new(ConversationManager::new(db_path));
    let tools_called = Arc::new(Mutex::new(Vec::<String>::new()));
    let event_bus: Arc<dyn EventBus> = Arc::new(ToolRecordingBus {
        tools_called: Arc::clone(&tools_called),
    });
    let mut registry = ToolRegistry::new();
    builtin::register_all(
        &mut registry,
        Arc::clone(&conversation),
        Arc::clone(&event_bus),
    );
    crate::core::office::register_tools(&mut registry);
    let tools = Arc::new(ToolManager::new(registry));
    let registry = tools.registry();
    let provider = Arc::new(ScriptedProvider::from_task(task));
    let runner = AgentRunner::new(provider, Arc::clone(&tools)).with_max_steps(40);

    let session_id = format!("eval-{}", task.id);
    shared_plan_mode_store().set_active(&session_id, options.plan_mode || task.plan_mode);

    let tool_ctx = ToolContext {
        workspace_root: workspace.clone(),
        request_context: Default::default(),
        session_id: session_id.clone(),
        assistant_message_id: "assistant".into(),
        conversation,
        event_bus,
        tasks: Arc::new(Mutex::new(Vec::new())),
        ask_store: Arc::new(AskStore::new()),
        path_permission_store: Arc::new(PathPermissionStore::new()),
        registry: Some(registry),
        provider: None,
        subagent_depth: 0,
        max_subagent_depth: 1,
        subagent_id: None,
        parent_activity_id: None,
        app_handle: None,
        cancelled: Arc::new(AtomicBool::new(false)),
    };

    let request = ChatRequest {
        request_id: format!("req-{}", task.id),
        session_id: session_id.clone(),
        messages: vec![ChatMessage {
            id: "user-1".into(),
            session_id: session_id.clone(),
            role: Role::User,
            content: task.prompt.clone(),
            reasoning: None,
            work_timeline: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 1,
            estimated_tokens: None,
        }],
        context: Default::default(),
        provider: None,
        stream: true,
        tools: std::sync::Arc::from([]),
        temperature: None,
        max_tokens: None,
    };

    let (tx, mut rx) = mpsc::channel::<StreamEvent>(64);
    let soft_queue = Arc::new(Mutex::new(VecDeque::new()));
    let cancelled = Arc::clone(&tool_ctx.cancelled);

    let run = tauri::async_runtime::spawn(async move {
        runner
            .run(request, tool_ctx, tx, cancelled, soft_queue)
            .await
    });

    let mut answer = String::new();
    let mut finish_reason = None;
    let mut statuses = Vec::new();
    while let Some(event) = rx.recv().await {
        match event {
            StreamEvent::TurnComplete {
                content,
                finish_reason: reason,
                ..
            } => {
                answer = content;
                finish_reason = reason;
            }
            StreamEvent::Delta(delta) => answer.push_str(&delta),
            StreamEvent::Status { kind } => statuses.push(kind),
            _ => {}
        }
    }
    let run_err = run.await.err().map(|e| e.to_string());

    let mut errors = Vec::new();
    if let Some(err) = run_err {
        errors.push(err);
    }
    for assertion in &task.assertions {
        if let Some(err) = check_assertion(
            assertion,
            &workspace,
            &answer,
            &finish_reason,
            &statuses,
            &tools_called,
        ) {
            errors.push(err);
        }
    }

    let _ = fs::remove_dir_all(&workspace);

    TaskResult {
        id: task.id.clone(),
        passed: errors.is_empty(),
        skipped: false,
        seed,
        answer,
        finish_reason,
        statuses,
        errors,
    }
}

fn check_assertion(
    assertion: &EvalAssertion,
    workspace: &Path,
    answer: &str,
    finish_reason: &Option<String>,
    statuses: &[String],
    tools_called: &Arc<Mutex<Vec<String>>>,
) -> Option<String> {
    match assertion {
        EvalAssertion::FileContains { path, text } => {
            let content = fs::read_to_string(workspace.join(path)).unwrap_or_default();
            if content.contains(text) {
                None
            } else {
                Some(format!("fileContains failed: {path} missing `{text}`"))
            }
        }
        EvalAssertion::FileEquals { path, text } => {
            let content = fs::read_to_string(workspace.join(path)).unwrap_or_default();
            if content == *text {
                None
            } else {
                Some(format!("fileEquals failed: {path}"))
            }
        }
        EvalAssertion::FileMissing { path } => {
            if workspace.join(path).exists() {
                Some(format!("fileMissing failed: {path} exists"))
            } else {
                None
            }
        }
        EvalAssertion::AnswerContains { text } => {
            if answer.contains(text) {
                None
            } else {
                Some(format!("answerContains failed: `{text}`"))
            }
        }
        EvalAssertion::AnswerNotContains { text } => {
            if answer.contains(text) {
                Some(format!("answerNotContains failed: found `{text}`"))
            } else {
                None
            }
        }
        EvalAssertion::FinishReason { reason } => {
            if finish_reason.as_deref() == Some(reason.as_str()) {
                None
            } else {
                Some(format!(
                    "finishReason failed: expected `{reason}`, got {:?}",
                    finish_reason
                ))
            }
        }
        EvalAssertion::StatusSeen { kind } => {
            if statuses.iter().any(|s| s == kind || s.contains(kind)) {
                None
            } else {
                Some(format!("statusSeen failed: `{kind}` not in {statuses:?}"))
            }
        }
        EvalAssertion::ToolCalled { name } => {
            let called = tools_called
                .lock()
                .ok()
                .map(|guard| guard.clone())
                .unwrap_or_default();
            if called.iter().any(|tool| tool == name) {
                None
            } else {
                Some(format!("toolCalled failed: `{name}` not in {called:?}"))
            }
        }
    }
}

struct ToolRecordingBus {
    tools_called: Arc<Mutex<Vec<String>>>,
}

impl EventBus for ToolRecordingBus {
    fn emit(&self, event: BusEvent) {
        if let BusEvent::ToolStarted { tool_name, .. } = event {
            if let Ok(mut guard) = self.tools_called.lock() {
                guard.push(tool_name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_completion_is_rejected_when_challenges_on() {
        let dir = std::env::temp_dir().join(format!("anya-eval-tasks-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("empty_claim.json"),
            r#"{
              "id": "empty_claim",
              "prompt": "Please edit src/main.rs and finish the task.",
              "script": [
                { "content": "任务完成", "tool_calls": [] },
                { "content": "任务完成", "tool_calls": [] }
              ],
              "assertions": [
                { "type": "statusSeen", "kind": "reject_empty_completion" },
                { "type": "finishReason", "reason": "unverified_completion" }
              ]
            }"#,
        )
        .unwrap();

        let report = run_eval(EvalOptions {
            challenges: true,
            compact: true,
            plan_mode: false,
            tasks_dir: dir.clone(),
            results_dir: dir.join("results"),
            filter: None,
            seeds: 1,
        })
        .await
        .unwrap();

        assert_eq!(report.failed, 0, "{:?}", report.results);
        let _ = fs::remove_dir_all(dir);
    }
}
