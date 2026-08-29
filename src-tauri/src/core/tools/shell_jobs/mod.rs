//! Background and foreground shell job execution.

mod constants;
mod foreground;
mod output;
mod process;
mod types;

pub use foreground::run_foreground;
pub use output::background_allowed;
pub use process::terminate_process_tree;
pub use types::ShellJobStore;

#[cfg(test)]
mod tests {
    use super::constants::{
        IDLE_CHECK_GRACE, IDLE_CHECK_MIN_INTERVAL, IDLE_JUDGE_ROUNDS,
    };
    use super::foreground::{next_wait_action, run_foreground_with_policy, WaitAction, WaitPolicy, WaitSnapshot};
    use super::*;
    use crate::core::tools::context::ToolContext;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    fn test_context() -> (ToolContext, std::path::PathBuf) {
        let db_path =
            std::env::temp_dir().join(format!("peek-shell-job-{}.db", uuid::Uuid::new_v4()));
        struct NullBus;
        impl crate::core::event::EventBus for NullBus {
            fn emit(&self, _event: crate::core::event::BusEvent) {}
        }
        let context = ToolContext {
            workspace_root: std::env::temp_dir(),
            request_context: Default::default(),
            session_id: "test".into(),
            assistant_message_id: "assistant".into(),
            conversation: Arc::new(
                crate::core::chat::conversation_manager::ConversationManager::new(db_path.clone()),
            ),
            event_bus: Arc::new(NullBus),
            tasks: Arc::new(Mutex::new(Vec::new())),
            ask_store: Arc::new(crate::core::tools::context::AskStore::new()),
            path_permission_store: Arc::new(crate::core::tools::context::PathPermissionStore::new()),
            registry: None,
            provider: None,
            subagent_depth: 0,
            max_subagent_depth: 0,
            subagent_id: None,
            parent_activity_id: None,
            app_handle: None,
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        (context, db_path)
    }

    /// Fake provider for judge tests: always answers with a fixed verdict
    /// token, regardless of the prompt content.
    struct FixedVerdictProvider {
        token: &'static str,
    }

    #[async_trait::async_trait]
    impl crate::core::ai::provider::AIProvider for FixedVerdictProvider {
        fn id(&self) -> &'static str {
            "fixed-verdict-test-provider"
        }

        async fn stream(
            &self,
            _request: crate::core::runtime::ChatRequest,
            tx: tokio::sync::mpsc::Sender<crate::core::runtime::StreamEvent>,
        ) -> Result<(), crate::core::ai::provider::ProviderError> {
            let _ = tx
                .send(crate::core::runtime::StreamEvent::TurnComplete {
                    content: self.token.to_string(),
                    reasoning: None,
                    tool_calls: Vec::new(),
                    finish_reason: None,
                })
                .await;
            Ok(())
        }
    }

    fn context_with_verdict(token: &'static str) -> (ToolContext, std::path::PathBuf) {
        let (mut context, db_path) = test_context();
        context.provider = Some(Arc::new(FixedVerdictProvider { token }));
        (context, db_path)
    }

    #[test]
    fn background_job_is_registered_before_waiter_runs() {
        let store = ShellJobStore::new();
        let id = store
            .spawn_background(
                "Write-Output 'ok'".into(),
                None,
                Arc::new(AtomicBool::new(false)),
                None,
            )
            .expect("spawn");
        let (context, db_path) = test_context();
        let status = store.wait_job(&id, &context).expect("wait");
        assert!(
            status.contains("status: done") || status.contains("exit_code:"),
            "unexpected status: {status}"
        );
        drop(context);
        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn foreground_command_stops_when_cancelled() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&cancelled);
        let started = Instant::now();
        let canceller = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            signal.store(true, Ordering::Relaxed);
        });

        let error = run_foreground("Start-Sleep -Seconds 30", None, &cancelled, None).unwrap_err();
        canceller.join().unwrap();

        assert!(error.is_cancelled());
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn finite_commands_are_not_allowed_in_background() {
        assert!(!background_allowed("git status"));
        assert!(!background_allowed("pnpm build"));
        assert!(!background_allowed("docker compose ps"));
        assert!(!background_allowed("docker compose logs --tail 100"));
        assert!(background_allowed("docker compose logs -f --tail 100"));
        assert!(background_allowed("Get-Content -Wait -Tail 100 app.log"));
    }

    #[test]
    fn background_output_is_readable_before_process_exits() {
        let store = ShellJobStore::new();
        let id = store
            .spawn_background(
                "Write-Output 'first'; Start-Sleep -Milliseconds 800; Write-Output 'second'".into(),
                None,
                Arc::new(AtomicBool::new(false)),
                None,
            )
            .expect("spawn");

        let deadline = Instant::now() + Duration::from_secs(3);
        let running = loop {
            let status = store.read_output_limited(&id, None, None).expect("read");
            if status.contains("first") {
                break status;
            }
            assert!(Instant::now() < deadline, "first log line was not streamed");
            std::thread::sleep(Duration::from_millis(25));
        };
        assert!(running.contains("status: running"));

        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let status = store.read_output_limited(&id, None, None).expect("read");
            if status.contains("status: done") {
                assert!(status.contains("first"));
                assert!(status.contains("second"));
                break;
            }
            assert!(
                Instant::now() < deadline,
                "background command did not finish"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    /// Windows regression: a wrapper script's direct process exits while a
    /// background service it spawned keeps the output pipe open, so EOF
    /// never arrives. Foreground must still return promptly with the output
    /// collected so far.
    #[test]
    fn foreground_returns_when_descendant_holds_pipe() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let started = Instant::now();
        let command = concat!(
            "Write-Output 'start'; ",
            "cmd /c \"start /b powershell -NoProfile -Command Start-Sleep -Seconds 300\"; ",
            "Start-Sleep -Milliseconds 500; Write-Output 'done'"
        );
        let result = run_foreground(command, None, &cancelled, None).expect("run");
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "foreground command stuck on held pipe: {}s",
            started.elapsed().as_secs()
        );
        assert!(result.contains("done"), "missing final output: {result}");
        assert!(
            result.contains("note:"),
            "expected held-pipe note: {result}"
        );
    }

    /// A direct child that never exits on its own (e.g. a wrapper process
    /// whose real work is already done but which keeps running) must still
    /// be recognized as finished once the model judge says so — well before
    /// the hard `shell_timeout_secs` ceiling.
    #[test]
    fn foreground_recovers_when_lingering_process_is_judged_finished() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let started = Instant::now();
        let (context, db_path) = context_with_verdict("FINISHED");
        let result = run_foreground(
            "Write-Output 'build succeeded'; Start-Sleep -Seconds 300",
            None,
            &cancelled,
            Some(&context),
        )
        .expect("run");
        assert!(
            started.elapsed() < Duration::from_secs(45),
            "lingering process was not reclaimed promptly: {}s",
            started.elapsed().as_secs()
        );
        assert!(
            result.contains("build succeeded"),
            "missing output: {result}"
        );
        assert!(
            result.contains("note:"),
            "expected idle-completion note: {result}"
        );
        drop(context);
        let _ = std::fs::remove_file(db_path);
    }

    /// While the model judge keeps saying the task is still running, the
    /// process must not be killed early — it should keep waiting.
    #[test]
    fn foreground_keeps_waiting_while_judged_running() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let (context, db_path) = context_with_verdict("RUNNING");
        let started = Instant::now();
        let result = run_foreground(
            "Write-Output 'compiling'; Start-Sleep -Milliseconds 900",
            None,
            &cancelled,
            Some(&context),
        )
        .expect("run");
        assert!(started.elapsed() < Duration::from_secs(10));
        assert!(result.contains("compiling"));
        drop(context);
        let _ = std::fs::remove_file(db_path);
    }

    fn snapshot() -> WaitSnapshot {
        WaitSnapshot {
            elapsed: Duration::from_secs(1),
            since_progress: Duration::from_secs(1),
            since_judge: None,
            judge_interval: IDLE_CHECK_MIN_INTERVAL,
            judge_rounds_left: IDLE_JUDGE_ROUNDS,
            stall_unconfirmed: false,
            activity_measurable: true,
        }
    }

    fn policy() -> WaitPolicy {
        WaitPolicy {
            ceiling: Duration::from_secs(3600),
            stall: Duration::from_secs(120),
        }
    }

    #[test]
    fn a_command_making_progress_is_left_alone() {
        let snap = WaitSnapshot {
            elapsed: Duration::from_secs(900),
            since_progress: Duration::from_secs(2),
            since_judge: Some(Duration::from_secs(1)),
            ..snapshot()
        };
        assert_eq!(
            next_wait_action(&policy(), &snap),
            WaitAction::KeepWaiting,
            "a long command that keeps working must not be interrupted"
        );
    }

    #[test]
    fn the_ceiling_wins_over_everything_else() {
        let snap = WaitSnapshot {
            elapsed: Duration::from_secs(3600),
            since_progress: Duration::from_millis(10),
            ..snapshot()
        };
        assert_eq!(next_wait_action(&policy(), &snap), WaitAction::Timeout);
    }

    #[test]
    fn a_stall_asks_the_model_before_giving_up() {
        let stalled = WaitSnapshot {
            elapsed: Duration::from_secs(300),
            since_progress: Duration::from_secs(120),
            ..snapshot()
        };
        assert_eq!(next_wait_action(&policy(), &stalled), WaitAction::Judge);

        let unconfirmed = WaitSnapshot {
            stall_unconfirmed: true,
            ..stalled
        };
        assert_eq!(
            next_wait_action(&policy(), &unconfirmed),
            WaitAction::Stalled
        );

        let no_judge = WaitSnapshot {
            judge_rounds_left: 0,
            ..stalled
        };
        assert_eq!(next_wait_action(&policy(), &no_judge), WaitAction::Stalled);
    }

    /// Without a usable activity signal, "quiet" and "stuck" are the same
    /// thing from the outside — so nothing may be killed for being quiet.
    #[test]
    fn silence_alone_never_stops_a_command() {
        let snap = WaitSnapshot {
            elapsed: Duration::from_secs(600),
            since_progress: Duration::from_secs(590),
            since_judge: Some(Duration::from_secs(1)),
            stall_unconfirmed: true,
            activity_measurable: false,
            ..snapshot()
        };
        assert_eq!(next_wait_action(&policy(), &snap), WaitAction::KeepWaiting);
    }

    #[test]
    fn periodic_checks_respect_the_grace_period_and_backoff() {
        let too_early = WaitSnapshot {
            elapsed: IDLE_CHECK_GRACE - Duration::from_secs(1),
            ..snapshot()
        };
        assert_eq!(
            next_wait_action(&policy(), &too_early),
            WaitAction::KeepWaiting
        );

        let first_check = WaitSnapshot {
            elapsed: IDLE_CHECK_GRACE,
            ..snapshot()
        };
        assert_eq!(next_wait_action(&policy(), &first_check), WaitAction::Judge);

        let waiting_out_backoff = WaitSnapshot {
            elapsed: Duration::from_secs(60),
            since_judge: Some(IDLE_CHECK_MIN_INTERVAL - Duration::from_millis(1)),
            ..snapshot()
        };
        assert_eq!(
            next_wait_action(&policy(), &waiting_out_backoff),
            WaitAction::KeepWaiting
        );

        let backoff_elapsed = WaitSnapshot {
            elapsed: Duration::from_secs(60),
            since_judge: Some(IDLE_CHECK_MIN_INTERVAL),
            ..snapshot()
        };
        assert_eq!(
            next_wait_action(&policy(), &backoff_elapsed),
            WaitAction::Judge
        );
    }

    /// A command that burns CPU without printing anything (compiling,
    /// linking, packing) must be allowed to keep going even though its stall
    /// window has long since passed in output terms.
    #[test]
    fn silent_but_busy_commands_are_not_killed() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let policy = WaitPolicy {
            ceiling: Duration::from_secs(120),
            stall: Duration::from_secs(2),
        };
        let started = Instant::now();
        let result = run_foreground_with_policy(
            "$sw=[Diagnostics.Stopwatch]::StartNew(); $x=0; \
             while ($sw.Elapsed.TotalSeconds -lt 6) { $x++ }; Write-Output \"spun $x times\"",
            None,
            &cancelled,
            None,
            policy,
        )
        .expect("a busy command must not be reported as stuck");
        assert!(
            result.contains("spun"),
            "command was cut short before finishing: {result}"
        );
        assert!(started.elapsed() >= Duration::from_secs(5));
    }

    /// A process that produces nothing and burns no CPU is stuck, and must be
    /// reclaimed after the stall window instead of holding the turn until the
    /// absolute ceiling.
    #[test]
    fn stuck_commands_are_reclaimed_before_the_ceiling() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let policy = WaitPolicy {
            ceiling: Duration::from_secs(600),
            stall: Duration::from_secs(2),
        };
        let started = Instant::now();
        let error =
            run_foreground_with_policy("Start-Sleep -Seconds 300", None, &cancelled, None, policy)
                .expect_err("an idle process must be reported as stuck");
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "stuck command was not reclaimed promptly: {}s",
            started.elapsed().as_secs()
        );
        let message = error.to_string();
        assert!(
            message.contains("no progress"),
            "unexpected stall message: {message}"
        );
    }

    /// Same held-pipe scenario through the background path: the job must be
    /// published as done instead of hanging until the daemon exits.
    #[test]
    fn background_job_finishes_when_descendant_holds_pipe() {
        let store = ShellJobStore::new();
        let id = store
            .spawn_background(
                concat!(
                    "Write-Output 'start'; ",
                    "cmd /c \"start /b powershell -NoProfile -Command Start-Sleep -Seconds 300\"; ",
                    "Start-Sleep -Milliseconds 500; Write-Output 'done'"
                )
                .into(),
                None,
                Arc::new(AtomicBool::new(false)),
                None,
            )
            .expect("spawn");
        let (context, db_path) = test_context();
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let status = store.read_output_limited(&id, None, None).expect("read");
            if status.contains("status: done") {
                assert!(status.contains("done"), "missing final output: {status}");
                assert!(
                    status.contains("note:"),
                    "expected held-pipe note: {status}"
                );
                break;
            }
            assert!(
                Instant::now() < deadline,
                "background job stuck on held pipe: {status}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        drop(context);
        let _ = std::fs::remove_file(db_path);
    }
}
