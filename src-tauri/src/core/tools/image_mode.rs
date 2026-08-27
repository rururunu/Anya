//! Per-session image-mode toolbar values applied to `generate_image`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageModeOptions {
    pub size: String,
    pub quality: String,
    pub n: u8,
    pub style_prompt: String,
    pub reference_images: Vec<String>,
}

#[derive(Default)]
struct ImageModeStore {
    sessions: HashMap<String, ImageModeOptions>,
}

impl ImageModeStore {
    fn set(&mut self, session_id: &str, options: Option<ImageModeOptions>) {
        if session_id.is_empty() {
            return;
        }
        if let Some(options) = options {
            self.sessions.insert(session_id.to_string(), options);
        } else {
            self.sessions.remove(session_id);
        }
    }

    fn get(&self, session_id: &str) -> Option<ImageModeOptions> {
        self.sessions.get(session_id).cloned()
    }
}

fn shared_store() -> &'static Mutex<ImageModeStore> {
    static STORE: OnceLock<Mutex<ImageModeStore>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(ImageModeStore::default()))
}

pub fn set_image_mode(session_id: &str, options: Option<ImageModeOptions>) {
    if let Ok(mut store) = shared_store().lock() {
        store.set(session_id, options);
    }
}

pub fn image_mode_options(session_id: &str) -> Option<ImageModeOptions> {
    shared_store().lock().ok().and_then(|store| store.get(session_id))
}

pub fn is_image_mode(session_id: &str) -> bool {
    image_mode_options(session_id).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_clears_per_session() {
        let id = format!("img-mode-{}", uuid::Uuid::new_v4());
        set_image_mode(
            &id,
            Some(ImageModeOptions {
                size: "1024x1536".into(),
                quality: "high".into(),
                n: 2,
                style_prompt: "anime".into(),
                reference_images: vec!["path:/tmp/ref.png".into()],
            }),
        );
        let stored = image_mode_options(&id).expect("stored");
        assert_eq!(stored.size, "1024x1536");
        assert_eq!(stored.n, 2);
        assert_eq!(stored.reference_images.len(), 1);
        set_image_mode(&id, None);
        assert!(image_mode_options(&id).is_none());
    }
}
