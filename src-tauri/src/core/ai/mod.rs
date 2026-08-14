pub mod antigravity;
pub mod deepseek;
pub mod embed;
pub mod image_analysis;
pub mod multimodal;
pub mod multimodal_response;
pub mod provider;
pub mod registry;

pub use provider::ProviderError;
pub use registry::resolve_provider;
pub(crate) use registry::resolve_provider_for_selection;
