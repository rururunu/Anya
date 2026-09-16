//! Headless AgentRunner eval harness (real runtime, not a Python toy loop).
//!
//! Tasks live under `eval/tasks/*.json`. Ablation flags toggle challenge /
//! mid-turn compact so harness changes can be measured on the same signal.

use std::collections::VecDeque;

#[path = "eval_live.rs"]
mod live;
pub use live::{EvalMetrics, LiveEvalConfig};
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
    #[serde(default)]
    pub followups: Vec<EvalFollowup>,
    #[serde(default)]
    pub context_window: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalFollowup {
    pub after_tool_calls: usize,
    pub content: String,
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
    /// Deterministic acceptance check authored in the fixture, not by the model.
    CommandSucceeds { command: String },
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
    pub live: Option<LiveEvalConfig>,
    pub include_office: bool,
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
    pub metrics: EvalMetrics,
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
    pub provider: String,
    pub model: Option<String>,
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
        request: ChatRequest,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        // Compaction uses the same provider, but must not consume a scripted
        // task action. Otherwise a summary request could swallow its next edit.
        let turn = if request.request_id.starts_with("compact-") {
            ProviderTurn {
                content: request
                    .messages
                    .last()
                    .map(|m| super::limits::truncate_tool_output(&m.content, 3500))
                    .unwrap_or_default(),
                tool_calls: vec![],
            }
        } else {
            self.take_turn()
        };
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
        if task.id.is_empty()
            || !task
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err("Evaluation task IDs must contain only letters, digits, _ or -".into());
        }
        for file in &task.setup_files {
            if !safe_fixture_path(&file.path) {
                return Err(format!(
                    "Setup path must stay inside the evaluation workspace: {}",
                    file.path
                ));
            }
        }
        for assertion in &task.assertions {
            if let EvalAssertion::FileContains { path, .. }
            | EvalAssertion::FileEquals { path, .. }
            | EvalAssertion::FileMissing { path } = assertion
            {
                if !safe_fixture_path(path) {
                    return Err(format!(
                        "Assertion path must stay inside the evaluation workspace: {path}"
                    ));
                }
            }
        }
        tasks.push(task);
    }
    tasks.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(tasks)
}

fn safe_fixture_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(':')
        && !path.starts_with(['/', '\\'])
        && !path.replace('\\', "/").split('/').any(|part| part == "..")
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

    if tasks.is_empty() {
        return Err("No evaluation tasks matched".into());
    }
    if options.live.is_none() && tasks.iter().any(|task| task.script.is_empty()) {
        return Err("Tasks without scripted turns require --live".into());
    }

    let seeds = options.seeds.max(1);
    let mut results = Vec::new();
    for seed in 0..seeds {
        for task in &tasks {
            results.push(run_one_task(task, &options, seed).await);
        }
    }

    let passed = results.iter().filter(|r| r.passed && !r.skipped).count();
    let failed = results.iter().filter(|r| !r.passed && !r.skipped).count();
    let pass_rate = if passed + failed == 0 {
        0.0
    } else {
        passed as f64 / (passed + failed) as f64
    };

    let report = EvalReport {
        options: EvalOptionsReport {
            challenges: options.challenges,
            compact: options.compact,
            plan_mode: options.plan_mode,
            seeds,
            provider: if options.live.is_some() {
                "live".into()
            } else {
                "scripted".into()
            },
            model: options.live.as_ref().map(|c| c.model.clone()),
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
        if !options.include_office || !crate::core::office::office_app_available(app) {
            return TaskResult {
                id: task.id.clone(),
                passed: true,
                skipped: true,
                seed,
                answer: String::new(),
                finish_reason: None,
                statuses: Vec::new(),
                errors: vec![format!(
                    "skipped: {app} evaluation not enabled or unavailable"
                )],
                metrics: EvalMetrics::default(),
            };
        }
    }

    let started = std::time::Instant::now();
    let metrics = Arc::new(Mutex::new(EvalMetrics::default()));
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
    let cancelled = Arc::new(AtomicBool::new(false));
    let soft_queue = Arc::new(Mutex::new(VecDeque::new()));
    let event_bus: Arc<dyn EventBus> = Arc::new(ToolRecordingBus {
        tools_called: Arc::clone(&tools_called),
        metrics: Arc::clone(&metrics),
        fingerprints: Mutex::new(std::collections::HashSet::new()),
        soft_queue: Arc::clone(&soft_queue),
        followups: Mutex::new(task.followups.clone()),
        finished_calls: std::sync::atomic::AtomicUsize::new(0),
        cancelled: Arc::clone(&cancelled),
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
    let provider: Arc<dyn AIProvider> = if let Some(config) = &options.live {
        match live::LiveProvider::new(config.clone(), Arc::clone(&metrics)) {
            Ok(provider) => Arc::new(provider),
            Err(error) => {
                return TaskResult {
                    id: task.id.clone(),
                    passed: false,
                    skipped: false,
                    seed,
                    answer: String::new(),
                    finish_reason: None,
                    statuses: vec![],
                    errors: vec![error],
                    metrics: EvalMetrics::default(),
                }
            }
        }
    } else {
        Arc::new(ScriptedProvider::from_task(task))
    };
    let mut runner = AgentRunner::new(Arc::clone(&provider), Arc::clone(&tools)).with_max_steps(40);
    if let Some(window) = task.context_window {
        runner = runner.with_max_turn_tokens(window);
    }

    let session_id = format!("eval-{}-{}", task.id, uuid::Uuid::new_v4());
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
        provider: Some(provider),
        subagent_depth: 0,
        max_subagent_depth: 1,
        subagent_id: None,
        parent_activity_id: None,
        app_handle: None,
        cancelled: Arc::clone(&cancelled),
    };

    let mut request = ChatRequest {
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

    if options.live.is_some() {
        request.messages.insert(
            0,
            ChatMessage {
                id: "eval-system".into(),
                session_id: session_id.clone(),
                role: Role::System,
                content: super::prompts::SYSTEM_PROMPT.into(),
                reasoning: None,
                work_timeline: None,
                tool_activities: None,
                tool_calls: None,
                tool_call_id: None,
                name: None,
                status: MessageStatus::Done,
                timestamp: 0,
                estimated_tokens: None,
            },
        );
    }
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(64);
    let run_cancelled = Arc::clone(&cancelled);

    let mut run = tauri::async_runtime::spawn(async move {
        runner
            .run(request, tool_ctx, tx, run_cancelled, soft_queue)
            .await
    });

    let mut answer = String::new();
    let mut finish_reason = None;
    let mut statuses = Vec::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(300);
    let mut timed_out = false;
    loop {
        let event = match tokio::time::timeout_at(deadline, rx.recv()).await {
            Ok(Some(event)) => event,
            Ok(None) => break,
            Err(_) => {
                cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
                timed_out = true;
                break;
            }
        };
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
    let mut stopped = true;
    let run_err = match tokio::time::timeout(std::time::Duration::from_secs(10), &mut run).await {
        Ok(Ok(Ok(()))) => None,
        Ok(Ok(Err(error))) => Some(error.to_string()),
        Ok(Err(error)) => Some(error.to_string()),
        Err(_) => {
            run.abort();
            stopped = false;
            Some("Evaluation cancellation did not settle; workspace retained".into())
        }
    };

    let mut errors = Vec::new();
    if timed_out {
        errors.push("Evaluation exceeded its 300-second limit".into());
    }
    if let Some(err) = run_err {
        errors.push(err);
    }
    for assertion in task.assertions.iter().filter(|_| stopped && !timed_out) {
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

    if stopped {
        let _ = fs::remove_dir_all(&workspace);
    }

    TaskResult {
        id: task.id.clone(),
        passed: errors.is_empty(),
        skipped: false,
        seed,
        answer,
        finish_reason,
        statuses,
        errors,
        metrics: {
            let mut m = metrics.lock().unwrap().clone();
            m.duration_ms = started.elapsed().as_millis() as u64;
            m
        },
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
        EvalAssertion::CommandSucceeds { command } => {
            match crate::core::tools::shell_jobs::run_foreground(
                command,
                Some(workspace),
                &Arc::new(AtomicBool::new(false)),
                None,
            ) {
                Ok(result) if super::agent_loop::post_edit_verify::shell_exit_code_ok(&result) => {
                    None
                }
                Ok(result) => Some(format!(
                    "commandSucceeds failed: {command}\n{}",
                    super::limits::truncate_tool_output(&result, 2000)
                )),
                Err(error) => Some(format!("commandSucceeds failed: {command}: {error}")),
            }
        }
        EvalAssertion::FileContains { path, text } => {
            let content = match fs::read_to_string(workspace.join(path)) {
                Ok(content) => content,
                Err(error) => return Some(format!("fileContains could not read {path}: {error}")),
            };
            if content.contains(text) {
                None
            } else {
                Some(format!("fileContains failed: {path} missing `{text}`"))
            }
        }
        EvalAssertion::FileEquals { path, text } => {
            let content = match fs::read_to_string(workspace.join(path)) {
                Ok(content) => content,
                Err(error) => return Some(format!("fileEquals could not read {path}: {error}")),
            };
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
    metrics: Arc<Mutex<EvalMetrics>>,
    fingerprints: Mutex<std::collections::HashSet<String>>,
    soft_queue: Arc<Mutex<VecDeque<String>>>,
    followups: Mutex<Vec<EvalFollowup>>,
    finished_calls: std::sync::atomic::AtomicUsize,
    cancelled: Arc<AtomicBool>,
}

impl EventBus for ToolRecordingBus {
    fn emit(&self, event: BusEvent) {
        match event {
            BusEvent::ToolStarted {
                tool_name,
                arguments,
                ..
            } => {
                if let Ok(mut guard) = self.tools_called.lock() {
                    guard.push(tool_name.clone());
                }
                if let Ok(mut m) = self.metrics.lock() {
                    m.tool_calls += 1;
                    if let Ok(mut seen) = self.fingerprints.lock() {
                        if !seen.insert(format!("{tool_name}|{arguments}")) {
                            m.repeated_tool_calls += 1;
                        }
                    }
                }
            }
            BusEvent::ToolFinished { success, .. } => {
                if !success {
                    if let Ok(mut m) = self.metrics.lock() {
                        m.failed_tool_calls += 1;
                    }
                }
                let count = self
                    .finished_calls
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    + 1;
                if let (Ok(mut pending), Ok(mut queue)) =
                    (self.followups.lock(), self.soft_queue.lock())
                {
                    pending.retain(|followup| {
                        if followup.after_tool_calls <= count {
                            queue.push_back(followup.content.clone());
                            false
                        } else {
                            true
                        }
                    });
                }
            }
            BusEvent::AskUser { .. }
            | BusEvent::PathPermissionRequest { .. }
            | BusEvent::ToolApprovalRequest { .. } => {
                if let Ok(mut m) = self.metrics.lock() {
                    m.user_interventions += 1;
                }
                self.cancelled
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            _ => {}
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
            live: None,
            include_office: false,
        })
        .await
        .unwrap();

        assert_eq!(report.failed, 0, "{:?}", report.results);
        let _ = fs::remove_dir_all(dir);
    }
}
