//! DeepSeek and OpenAI-compatible chat provider.

mod anthropic;
mod balance;
pub(crate) mod files;
mod image_fallback;
mod messages;
mod models;
mod multimodal;
mod provider;
mod stream;

pub(crate) use crate::core::ai::provider::ProviderError;
pub use balance::{get_user_balance, DeepSeekBalanceReport};
pub use files::{
    delete_file, list_files, retrieve_file, upload_path, DeepSeekFileDeleteResult,
    DeepSeekFileList, DeepSeekFileObject,
};
pub use models::list_models;
pub use models::{list_openai_compatible_models, normalize_models_url};
pub(crate) use models::{normalize_chat_completions_url, normalize_images_generations_url};
pub use provider::DeepSeekProvider;
pub(crate) use stream::RETRY_BACKOFF;

#[cfg(test)]
mod tests;
