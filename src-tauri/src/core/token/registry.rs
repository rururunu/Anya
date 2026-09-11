use std::collections::HashMap;
use std::sync::Arc;

use super::{
    DeepSeekTokenizer, DeepSeekV4Tokenizer, EstimatedTokenizer, OpenAiTokenizer, Tokenizer,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    Exact,
    Prefix,
    Provider,
    Fallback,
}

pub struct TokenizerSelection {
    pub tokenizer: Arc<dyn Tokenizer>,
    pub matched_by: MatchKind,
}

pub struct TokenizerRegistry {
    exact: HashMap<String, Arc<dyn Tokenizer>>,
    prefixes: Vec<(String, Arc<dyn Tokenizer>)>,
    providers: HashMap<String, Arc<dyn Tokenizer>>,
    fallback: Arc<dyn Tokenizer>,
}

impl Default for TokenizerRegistry {
    fn default() -> Self {
        let deepseek: Arc<dyn Tokenizer> = Arc::new(DeepSeekTokenizer);
        let deepseek_v4: Arc<dyn Tokenizer> = Arc::new(DeepSeekV4Tokenizer);
        let openai: Arc<dyn Tokenizer> = Arc::new(OpenAiTokenizer::new("gpt-4o"));
        let estimated: Arc<dyn Tokenizer> = Arc::new(EstimatedTokenizer);
        let mut registry = Self::new(Arc::clone(&estimated));

        registry.register_exact("deepseek-chat", Arc::clone(&deepseek));
        registry.register_exact("deepseek-reasoner", Arc::clone(&deepseek));
        registry.register_exact("deepseek-v4-flash", Arc::clone(&deepseek_v4));
        registry.register_exact("deepseek-v4-pro", Arc::clone(&deepseek_v4));
        registry.register_prefix("deepseek-v4-", Arc::clone(&deepseek_v4));
        registry.register_prefix("deepseek-ai/deepseek-v4-", deepseek_v4);
        registry.register_prefix("deepseek-", Arc::clone(&deepseek));
        registry.register_prefix("gpt-", Arc::clone(&openai));
        registry.register_prefix("chatgpt-", Arc::clone(&openai));
        registry.register_prefix("o1", Arc::clone(&openai));
        registry.register_prefix("o3", Arc::clone(&openai));
        registry.register_prefix("o4", Arc::clone(&openai));
        registry.register_provider("openai", Arc::clone(&openai));
        registry.register_provider("deepseek", deepseek);
        registry.register_prefix("gemini-", Arc::clone(&estimated));
        registry
    }
}

impl TokenizerRegistry {
    pub fn new(fallback: Arc<dyn Tokenizer>) -> Self {
        Self {
            exact: HashMap::new(),
            prefixes: Vec::new(),
            providers: HashMap::new(),
            fallback,
        }
    }

    pub fn register_exact(&mut self, model: impl Into<String>, tokenizer: Arc<dyn Tokenizer>) {
        self.exact.insert(normalize(&model.into()), tokenizer);
    }

    pub fn register_prefix(&mut self, prefix: impl Into<String>, tokenizer: Arc<dyn Tokenizer>) {
        self.prefixes.push((normalize(&prefix.into()), tokenizer));
        self.prefixes
            .sort_by_key(|(prefix, _)| std::cmp::Reverse(prefix.len()));
    }

    pub fn register_provider(
        &mut self,
        provider: impl Into<String>,
        tokenizer: Arc<dyn Tokenizer>,
    ) {
        self.providers
            .insert(normalize(&provider.into()), tokenizer);
    }

    pub fn resolve(&self, model: &str, provider: &str) -> TokenizerSelection {
        let model = normalize(model);
        if let Some(tokenizer) = self.exact.get(&model) {
            return selection(tokenizer, MatchKind::Exact);
        }
        if let Some((_, tokenizer)) = self
            .prefixes
            .iter()
            .find(|(prefix, _)| model.starts_with(prefix))
        {
            return selection(tokenizer, MatchKind::Prefix);
        }
        if let Some(tokenizer) = self.providers.get(&normalize(provider)) {
            return selection(tokenizer, MatchKind::Provider);
        }
        TokenizerSelection {
            tokenizer: Arc::clone(&self.fallback),
            matched_by: MatchKind::Fallback,
        }
    }
}

fn selection(tokenizer: &Arc<dyn Tokenizer>, matched_by: MatchKind) -> TokenizerSelection {
    TokenizerSelection {
        tokenizer: Arc::clone(tokenizer),
        matched_by,
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_uses_exact_prefix_provider_then_fallback() {
        let registry = TokenizerRegistry::default();
        assert_eq!(
            registry.resolve("deepseek-chat", "custom").matched_by,
            MatchKind::Exact
        );
        assert_eq!(
            registry.resolve("gpt-4.1-mini", "custom").matched_by,
            MatchKind::Prefix
        );
        assert_eq!(
            registry.resolve("vendor-model", "openai").matched_by,
            MatchKind::Provider
        );
        assert_eq!(
            registry.resolve("unknown", "unknown").matched_by,
            MatchKind::Fallback
        );
    }

    #[test]
    fn registry_routes_the_full_deepseek_v4_family_to_v4_tokenizer() {
        let registry = TokenizerRegistry::default();
        for model in [
            "deepseek-v4-flash",
            "deepseek-v4-pro",
            "deepseek-v4-flash-0731",
            "deepseek-v4-flash-dspark",
            "deepseek-v4-pro-dspark",
            "deepseek-v4-flash-base",
            "deepseek-v4-pro-base",
            "deepseek-ai/DeepSeek-V4-Flash",
            "deepseek-ai/DeepSeek-V4-Pro",
        ] {
            let selection = registry.resolve(model, "deepseek");
            assert_eq!(
                selection.tokenizer.name(),
                "deepseek-ai/DeepSeek-V4",
                "{model}"
            );
            assert_ne!(selection.matched_by, MatchKind::Fallback, "{model}");
        }
    }

    #[test]
    fn longest_prefix_wins() {
        let fallback: Arc<dyn Tokenizer> = Arc::new(EstimatedTokenizer);
        let mut registry = TokenizerRegistry::new(Arc::clone(&fallback));
        registry.register_prefix("model-", Arc::clone(&fallback));
        registry.register_prefix("model-specific-", Arc::clone(&fallback));
        assert_eq!(registry.prefixes[0].0, "model-specific-");
    }
}
