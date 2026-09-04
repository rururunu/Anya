use std::io::Write;
use std::path::Path;
use std::time::Instant;

use chrono::{SecondsFormat, Utc};
use tracing::{info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Cancelled,
    ProviderHttp,
    Transport,
    Tool,
    ContextLimit,
    UserFacing,
    Unknown,
}

impl ErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::ProviderHttp => "provider_http",
            Self::Transport => "transport",
            Self::Tool => "tool",
            Self::ContextLimit => "context_limit",
            Self::UserFacing => "user_facing",
            Self::Unknown => "unknown",
        }
    }

    pub fn classify(message: &str) -> Self {
        let lower = message.to_ascii_lowercase();
        if lower.contains("cancelled") || lower.contains("canceled") {
            return Self::Cancelled;
        }
        if lower.contains("context") && (lower.contains("limit") || lower.contains("length")) {
            return Self::ContextLimit;
        }
        if lower.contains("tool") {
            return Self::Tool;
        }
        if lower.contains("timeout")
            || lower.contains("connection")
            || lower.contains("dns")
            || lower.contains("tls")
            || lower.contains("proxy")
            || lower.contains("network")
        {
            return Self::Transport;
        }
        if lower.contains("http")
            || lower.contains("401")
            || lower.contains("403")
            || lower.contains("429")
            || lower.contains("502")
            || lower.contains("503")
            || lower.contains("api returned")
            || lower.contains("status")
        {
            return Self::ProviderHttp;
        }
        if !message.trim().is_empty() {
            return Self::UserFacing;
        }
        Self::Unknown
    }
}

pub struct TurnSpan {
    pub session_id: String,
    pub turn_id: String,
    pub message_id: String,
    pub provider: String,
    pub model: String,
    started: Instant,
    first_token_at: Option<Instant>,
    tool_count: u32,
}

impl TurnSpan {
    pub fn start(
        session_id: impl Into<String>,
        turn_id: impl Into<String>,
        message_id: impl Into<String>,
        provider: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let span = Self {
            session_id: session_id.into(),
            turn_id: turn_id.into(),
            message_id: message_id.into(),
            provider: provider.into(),
            model: model.into(),
            started: Instant::now(),
            first_token_at: None,
            tool_count: 0,
        };
        info!(
            target: "peek.turn",
            session_id = %span.session_id,
            turn_id = %span.turn_id,
            message_id = %span.message_id,
            provider = %span.provider,
            model = %span.model,
            "turn_start"
        );
        span
    }

    pub fn mark_first_token(&mut self) {
        if self.first_token_at.is_none() {
            self.first_token_at = Some(Instant::now());
        }
    }

    pub fn add_tools(&mut self, count: u32) {
        self.tool_count = self.tool_count.saturating_add(count);
    }

    pub fn soft_inject(&self, chars: usize) {
        info!(
            target: "peek.turn",
            session_id = %self.session_id,
            turn_id = %self.turn_id,
            message_id = %self.message_id,
            chars,
            "soft_inject"
        );
    }

    pub fn finish_ok(&self, finish_reason: Option<&str>) {
        let duration_ms = self.started.elapsed().as_millis() as u64;
        let ttft_ms = self
            .first_token_at
            .map(|at| at.duration_since(self.started).as_millis() as u64);
        info!(
            target: "peek.turn",
            session_id = %self.session_id,
            turn_id = %self.turn_id,
            message_id = %self.message_id,
            provider = %self.provider,
            model = %self.model,
            duration_ms,
            ttft_ms,
            tool_count = self.tool_count,
            finish_reason = finish_reason.unwrap_or("stop"),
            "turn_finish"
        );
    }

    pub fn finish_err(&self, error: &str) {
        let duration_ms = self.started.elapsed().as_millis() as u64;
        let kind = ErrorKind::classify(error);
        warn!(
            target: "peek.turn",
            session_id = %self.session_id,
            turn_id = %self.turn_id,
            message_id = %self.message_id,
            provider = %self.provider,
            model = %self.model,
            duration_ms,
            tool_count = self.tool_count,
            error_kind = kind.as_str(),
            error = %error,
            "turn_error"
        );
    }
}

pub fn init_logging(config_dir: &Path) {
    let logs_dir = config_dir.join("logs");
    let _ = std::fs::create_dir_all(&logs_dir);
    install_panic_hook(&logs_dir);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(
            "info,peek=debug,peek.turn=info,peek.agent=info,peek.tool=info,peek.provider=info",
        )
    });

    let file_appender = tracing_appender::rolling::daily(&logs_dir, "peek.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    // Leak the guard so the worker thread lives for the process lifetime.
    std::mem::forget(guard);

    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking)
        .with_target(true);

    let stdout_layer = fmt::layer().with_target(true).with_writer(std::io::stderr);

    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .try_init();
}

/// Crash 现场落盘：panic 走同步 append 写到当天日志文件，不依赖可能丢数据的异步 writer。
fn install_panic_hook(logs_dir: &Path) {
    let logs_dir = logs_dir.to_path_buf();
    std::panic::set_hook(Box::new(move |info| {
        let now = Utc::now();
        let message = format!(
            "{} PANIC {}{}",
            now.to_rfc3339_opts(SecondsFormat::Millis, true),
            panic_message(info),
            info.location()
                .map(|loc| format!(" at {}:{}", loc.file(), loc.line()))
                .unwrap_or_default()
        );
        // 保留默认行为：console/debug 构建在终端仍可见。
        eprintln!("{message}");
        let file_name = format!("peek.log.{}", now.format("%Y-%m-%d"));
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(logs_dir.join(file_name))
        {
            let _ = writeln!(file, "{message}");
        }
    }));
}

fn panic_message(info: &std::panic::PanicHookInfo<'_>) -> String {
    let payload = info.payload();
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        format!("{payload:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_errors() {
        assert_eq!(
            ErrorKind::classify("request cancelled"),
            ErrorKind::Cancelled
        );
        assert_eq!(
            ErrorKind::classify("connection reset by peer"),
            ErrorKind::Transport
        );
        assert_eq!(
            ErrorKind::classify("Multimodal API returned 502"),
            ErrorKind::ProviderHttp
        );
        assert_eq!(
            ErrorKind::classify("context length exceeded"),
            ErrorKind::ContextLimit
        );
    }
}
