use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    MissingPath,
    PatchConflict,
    InvalidArguments,
    Permission,
    Transient,
    Unknown,
}

impl ErrorCategory {
    pub fn classify(message: &str) -> Self {
        let text = message.to_lowercase();
        if [
            "permission denied",
            "access denied",
            "access is denied",
            "not allowed",
            "plan mode is active",
            "权限",
        ]
        .iter()
        .any(|s| text.contains(s))
        {
            Self::Permission
        } else if [
            "not found",
            "does not exist",
            "cannot find",
            "no such file",
            "找不到",
        ]
        .iter()
        .any(|s| text.contains(s))
        {
            Self::MissingPath
        } else if ["match", "patch", "old_string"]
            .iter()
            .any(|s| text.contains(s))
        {
            Self::PatchConflict
        } else if [
            "invalid argument",
            "missing field",
            "must be",
            "expected",
            "invalid type",
        ]
        .iter()
        .any(|s| text.contains(s))
        {
            Self::InvalidArguments
        } else if [
            "timeout",
            "timed out",
            "429",
            "502",
            "503",
            "connection reset",
            "temporarily unavailable",
        ]
        .iter()
        .any(|s| text.contains(s))
        {
            Self::Transient
        } else {
            Self::Unknown
        }
    }

    pub fn guidance(self) -> &'static str {
        match self {
            Self::MissingPath => "Locate the actual path with list_folder/find_files before retrying.",
            Self::PatchConflict => "Read the current target range and reconstruct the patch from exact text; do not overwrite the file.",
            Self::InvalidArguments => "Inspect the tool schema and correct the indicated fields before retrying.",
            Self::Permission => "Respect the denied scope. If the error mentions plan mode, stop writers and either finish the plan (save_plan / update_tasks) or wait until the user approves / leaves Plan. Do not bypass the gate.",
            Self::Transient => "A bounded retry may help for read-only operations. Check whether a write already took effect before retrying it.",
            Self::Unknown => "Inspect this error and choose a different approach instead of repeating the same call.",
        }
    }
}

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct ToolError {
    pub message: String,
    terminal: bool,
    cancelled: bool,
}

impl ToolError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            terminal: false,
            cancelled: false,
        }
    }

    pub fn user_denied(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            terminal: true,
            cancelled: false,
        }
    }

    pub fn cancelled() -> Self {
        Self {
            message: "tool execution cancelled".to_string(),
            terminal: false,
            cancelled: true,
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.terminal
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn category(&self) -> ErrorCategory {
        if self.terminal {
            ErrorCategory::Permission
        } else {
            ErrorCategory::classify(&self.message)
        }
    }
}

impl From<std::io::Error> for ToolError {
    fn from(value: std::io::Error) -> Self {
        Self::new(value.to_string())
    }
}

impl From<serde_json::Error> for ToolError {
    fn from(value: serde_json::Error) -> Self {
        Self::new(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn user_denial_is_a_terminal_tool_error() {
        assert!(ToolError::user_denied("denied").is_terminal());
        assert!(!ToolError::new("ordinary failure").is_terminal());
    }

    #[test]
    fn cancellation_is_distinct_from_user_denial() {
        let error = ToolError::cancelled();
        assert!(error.is_cancelled());
        assert!(!error.is_terminal());
    }
}
