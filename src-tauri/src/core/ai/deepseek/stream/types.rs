use serde::Deserialize;
use std::time::Duration;

pub(crate) const RETRY_BACKOFF: Duration = Duration::from_millis(500);

pub(super) const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
pub(super) const MAX_STREAM_ATTEMPTS: u32 = 5;

pub(crate) const USER_STREAM_INTERRUPTED: &str = "Connection interrupted, please retry";
pub(super) const USER_STREAM_STALLED: &str = "Response timed out, please retry";

#[derive(Debug, Deserialize)]
pub(super) struct ApiStreamResponse {
    #[serde(default)]
    pub(super) choices: Vec<ApiStreamChoice>,
    #[serde(default)]
    pub(super) usage: Option<ApiTokenUsage>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ApiTokenUsage {
    pub(super) prompt_tokens: usize,
    pub(super) completion_tokens: usize,
    /// First-party DeepSeek field. Prefer this when present.
    #[serde(default)]
    prompt_cache_hit_tokens: usize,
    #[serde(default)]
    prompt_tokens_details: ApiPromptTokensDetails,
    #[serde(default)]
    pub(super) completion_tokens_details: ApiCompletionTokensDetails,
}

impl ApiTokenUsage {
    pub(super) fn cache_read_tokens(&self) -> usize {
        self.prompt_cache_hit_tokens
            .max(self.prompt_tokens_details.cached_tokens)
    }
}

#[derive(Debug, Default, Deserialize)]
struct ApiPromptTokensDetails {
    #[serde(default)]
    cached_tokens: usize,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct ApiCompletionTokensDetails {
    #[serde(default)]
    pub(super) reasoning_tokens: usize,
}

#[derive(Debug, Deserialize)]
pub(super) struct ApiStreamChoice {
    pub(super) delta: ApiStreamDelta,
    #[serde(default)]
    pub(super) finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct ApiStreamDelta {
    #[serde(default)]
    pub(super) content: Option<String>,
    #[serde(default)]
    pub(super) reasoning_content: Option<String>,
    #[serde(default)]
    pub(super) reasoning: Option<String>,
    #[serde(default)]
    pub(super) tool_calls: Option<Vec<ApiToolCallDelta>>,
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct ApiToolCallDelta {
    pub(super) index: Option<usize>,
    pub(super) id: Option<String>,
    #[serde(default)]
    pub(super) function: Option<ApiToolCallFunction>,
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct ApiToolCallFunction {
    pub(super) name: Option<String>,
    pub(super) arguments: Option<String>,
}

#[derive(Debug, Default)]
pub(super) struct ToolCallBuilder {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) arguments: String,
}

impl ToolCallBuilder {
    /// Later SSE deltas often repeat `name: ""`. Never let that wipe a name
    /// we already collected. If the provider streams the name in growing
    /// prefixes, keep the longest value.
    pub(super) fn set_name(&mut self, name: impl AsRef<str>) {
        let name = name.as_ref().trim();
        if name.is_empty() {
            return;
        }
        if self.name.is_empty() || name.starts_with(&self.name) || !self.name.starts_with(name) {
            self.name = name.to_string();
        }
    }
}

#[cfg(test)]
mod tool_call_builder_tests {
    use super::ToolCallBuilder;

    #[test]
    fn empty_name_delta_does_not_wipe_existing_name() {
        let mut builder = ToolCallBuilder::default();
        builder.set_name("web_search");
        builder.set_name("");
        builder.set_name("   ");
        assert_eq!(builder.name, "web_search");
    }

    #[test]
    fn growing_name_prefix_is_kept() {
        let mut builder = ToolCallBuilder::default();
        builder.set_name("mcp__");
        builder.set_name("mcp__puppeteer__navigate");
        assert_eq!(builder.name, "mcp__puppeteer__navigate");
        builder.set_name("mcp__");
        assert_eq!(builder.name, "mcp__puppeteer__navigate");
    }
}

#[derive(Debug, Default)]
pub(crate) struct StreamReadOutcome {
    pub(crate) saw_done: bool,
    pub(crate) emitted: bool,
    pub(crate) finish_reason: Option<String>,
}

impl StreamReadOutcome {
    pub(crate) fn is_complete(&self) -> bool {
        self.saw_done || self.finish_reason.is_some()
    }
}

#[cfg(test)]
impl StreamReadOutcome {
    pub(crate) fn test_with(saw_done: bool, finish_reason: Option<String>) -> Self {
        Self {
            emitted: false,
            saw_done,
            finish_reason,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum SseKind {
    ChatCompletions { is_deepseek: bool },
    Responses { is_deepseek: bool },
    AnthropicMessages,
}
