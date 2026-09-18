pub mod active_file_provider;
mod capture_provider;
mod clipboard_provider;
pub mod environment_provider;
mod explorer_provider;

pub use capture_provider::{CaptureProvider, CaptureResult, PartialCapture};
pub use clipboard_provider::ClipboardProvider;
pub use explorer_provider::ExplorerProvider;
