//! DeepSeek and OpenAI-compatible chat provider.

mod anthropic;
mod image_fallback;
mod messages;
mod models;
mod multimodal;
mod provider;
mod stream;

pub use provider::DeepSeekProvider;
pub(crate) use crate::core::ai::provider::ProviderError;
pub(crate) use stream::RETRY_BACKOFF;
pub use models::list_models;
pub(crate) use models::{normalize_chat_completions_url, normalize_images_generations_url};
pub use models::{list_openai_compatible_models, normalize_models_url};

#[cfg(test)]
mod tests;
