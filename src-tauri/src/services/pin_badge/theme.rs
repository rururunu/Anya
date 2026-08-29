//! Resolve the current Anya accent color for the pin badge.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::models::settings::AppSettings;

/// Packed 0x00RRGGBB accent color (sRGB).
static ACCENT_RGB: AtomicU32 = AtomicU32::new(0x00_11_11_11);
/// Bumped whenever accent changes so badges can repaint.
static ACCENT_GENERATION: AtomicU32 = AtomicU32::new(1);

const DARK_ACCENT: u32 = 0x00_F5_F5_F5; // #f5f5f5 — themes.css --peek-accent
const LIGHT_ACCENT: u32 = 0x00_11_11_11; // #111111 — themes.css --peek-accent

pub fn accent_generation() -> u32 {
    ACCENT_GENERATION.load(Ordering::Relaxed)
}

/// Returns (r, g, b) of the current theme accent.
pub fn accent_rgb() -> (u8, u8, u8) {
    let packed = ACCENT_RGB.load(Ordering::Relaxed);
    (
        ((packed >> 16) & 0xFF) as u8,
        ((packed >> 8) & 0xFF) as u8,
        (packed & 0xFF) as u8,
    )
}

pub fn configure_from_settings(settings: &AppSettings) {
    let packed = resolve_accent(settings);
    let prev = ACCENT_RGB.swap(packed, Ordering::Relaxed);
    if prev != packed {
        ACCENT_GENERATION.fetch_add(1, Ordering::Relaxed);
    }
}

fn resolve_accent(settings: &AppSettings) -> u32 {
    if settings.color_scheme.is_dark() {
        DARK_ACCENT
    } else {
        LIGHT_ACCENT
    }
}
