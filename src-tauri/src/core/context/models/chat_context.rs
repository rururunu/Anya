use std::path::PathBuf;

use super::window_info::WindowInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureSource {
    Clipboard,
    Explorer,
    #[allow(dead_code)]
    UiAutomation,
    Browser,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatContext {
    pub selected_text: Option<String>,
    pub selected_files: Vec<PathBuf>,
    pub selected_images: Vec<String>,
    pub source: Option<CaptureSource>,
    pub window: Option<WindowInfo>,
}

impl ChatContext {
    pub fn empty() -> Self {
        Self {
            selected_text: None,
            selected_files: Vec::new(),
            selected_images: Vec::new(),
            source: None,
            window: None,
        }
    }

    pub fn has_content(&self) -> bool {
        self.selected_text
            .as_ref()
            .is_some_and(|text| !text.trim().is_empty())
            || !self.selected_files.is_empty()
            || !self.selected_images.is_empty()
    }
}
