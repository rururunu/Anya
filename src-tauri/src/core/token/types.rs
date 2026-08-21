use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TokenAccuracy {
    Exact,
    Mixed,
    #[default]
    Estimated,
}

impl TokenAccuracy {
    pub fn merge(self, other: Self) -> Self {
        if self == other {
            self
        } else {
            Self::Mixed
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenCount {
    pub tokens: usize,
    pub accuracy: TokenAccuracy,
    pub tokenizer: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCategory {
    Input,
    Output,
    System,
    Context,
    ToolCall,
    ToolResult,
    Memory,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub system_tokens: usize,
    pub context_tokens: usize,
    pub tool_call_tokens: usize,
    pub tool_result_tokens: usize,
    pub memory_tokens: usize,
    pub total_tokens: usize,
    pub accuracy: TokenAccuracy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Provider-reported prompt-cache reads (DeepSeek's `prompt_tokens` includes
    /// these; they are subtracted out of `input_tokens`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<usize>,
    /// Provider-reported reasoning tokens (`completion_tokens_details.reasoning_tokens`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<usize>,
}

impl TokenUsage {
    pub fn exact(input_tokens: usize, output_tokens: usize, source: impl Into<String>) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens.saturating_add(output_tokens),
            accuracy: TokenAccuracy::Exact,
            source: Some(source.into()),
            ..Self::default()
        }
    }

    pub fn exact_with_breakdown(
        input_tokens: usize,
        output_tokens: usize,
        source: impl Into<String>,
        cache_read_tokens: Option<usize>,
        reasoning_tokens: Option<usize>,
    ) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens.saturating_add(output_tokens),
            accuracy: TokenAccuracy::Exact,
            source: Some(source.into()),
            cache_read_tokens,
            reasoning_tokens,
            ..Self::default()
        }
    }

    pub fn add_category(&mut self, category: TokenCategory, tokens: usize) {
        match category {
            TokenCategory::Input => self.input_tokens += tokens,
            TokenCategory::Output => self.output_tokens += tokens,
            TokenCategory::System => self.system_tokens += tokens,
            TokenCategory::Context => self.context_tokens += tokens,
            TokenCategory::ToolCall => self.tool_call_tokens += tokens,
            TokenCategory::ToolResult => self.tool_result_tokens += tokens,
            TokenCategory::Memory => self.memory_tokens += tokens,
        }
        self.total_tokens = self.input_tokens.saturating_add(self.output_tokens);
    }

    pub fn accumulate(&mut self, delta: &Self) {
        self.input_tokens += delta.input_tokens;
        self.output_tokens += delta.output_tokens;
        self.system_tokens += delta.system_tokens;
        self.context_tokens += delta.context_tokens;
        self.tool_call_tokens += delta.tool_call_tokens;
        self.tool_result_tokens += delta.tool_result_tokens;
        self.memory_tokens += delta.memory_tokens;
        self.total_tokens = self.input_tokens.saturating_add(self.output_tokens);
        self.accuracy = if self.total_tokens == delta.total_tokens {
            delta.accuracy
        } else {
            self.accuracy.merge(delta.accuracy)
        };
        if self.source.is_none() {
            self.source.clone_from(&delta.source);
        } else if self.source != delta.source {
            self.source = Some("multiple".to_string());
        }
        if let Some(cache_read) = delta.cache_read_tokens {
            self.cache_read_tokens =
                Some(self.cache_read_tokens.unwrap_or(0).saturating_add(cache_read));
        }
        if let Some(reasoning) = delta.reasoning_tokens {
            self.reasoning_tokens =
                Some(self.reasoning_tokens.unwrap_or(0).saturating_add(reasoning));
        }
    }
}

/// Future cost trackers can consume this without coupling pricing to Agent Runtime.
#[allow(dead_code)]
pub trait TokenUsageObserver: Send + Sync {
    fn record(&self, model: &str, usage: &TokenUsage);
}

/// Future budget policies can inspect accounting snapshots without owning execution flow.
#[allow(dead_code)]
pub trait TokenBudgetPolicy: Send + Sync {
    fn allows(&self, model: &str, current: &TokenUsage, next_input_tokens: usize) -> bool;
}
