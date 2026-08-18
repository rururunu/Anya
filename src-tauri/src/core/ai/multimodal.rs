//! Helpers for multimodal fallback when the primary chat model rejects image input.
//!
//! Split multimodal analysis is a **supplement for text-only models** (e.g. DeepSeek-R1).
//! Vision-capable primaries such as Gemini must receive images natively and must never
//! be routed through a separate multimodal describe step.

use super::provider::ProviderError;

/// Whether the primary chat model already supports image input natively.
///
/// When true, image failures must surface as-is — do not call the multimodal fallback.
pub fn primary_model_has_native_vision(model: &str) -> bool {
    let m = model.trim().to_ascii_lowercase();
    if m.is_empty() {
        return false;
    }
    // Gemini (Antigravity / Google) — always native vision.
    if m.starts_with("gemini") {
        return true;
    }
    // Common OpenAI-compatible vision chat models.
    if m.contains("gpt-4o")
        || m.contains("gpt-4.1")
        || m.contains("gpt-4-turbo")
        || m.contains("gpt-4-vision")
        || m.contains("gpt-5")
        || m.starts_with("o1")
        || m.starts_with("o3")
        || m.starts_with("o4")
        || m.contains("chatgpt-4o")
    {
        return true;
    }
    // Claude / other explicit vision labels.
    if m.contains("claude") || m.contains("vision") {
        return true;
    }
    if m.starts_with("grok") {
        return true;
    }
    false
}

/// Whether an API error indicates the primary model cannot accept image payloads.
pub fn is_vision_unsupported_error(error: &ProviderError) -> bool {
    match error {
        ProviderError::Cancelled => false,
        ProviderError::Message(message) => is_vision_unsupported_message(message),
    }
}

pub fn is_vision_unsupported_message(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();

    if lower.contains("401") || lower.contains("403") || lower.contains("429") {
        return false;
    }
    if lower.contains("invalid api key") || lower.contains("incorrect api key") {
        return false;
    }
    if lower.contains("model") && lower.contains("not found") {
        return false;
    }

    let client_error = lower.contains("400")
        || lower.contains("422")
        || lower.contains("bad request")
        || lower.contains("unprocessable");

    if !client_error {
        return false;
    }

    const VISION_HINTS: &[&str] = &[
        "image",
        "vision",
        "multimodal",
        "image_url",
        "content type",
        "unsupported type",
        "does not support",
        "do not support",
        "not support",
        "invalid type",
        "modality",
        "visual",
        "图片",
        "视觉",
        "多模态",
    ];

    VISION_HINTS.iter().any(|hint| lower.contains(hint))
        || (lower.contains("content")
            && (lower.contains("invalid") || lower.contains("unsupported")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_openai_style_vision_rejection() {
        assert!(is_vision_unsupported_message(
            r#"DeepSeek API 400 Bad Request: {"error":{"message":"Invalid content: image_url is only supported by vision models"}}"#
        ));
    }

    #[test]
    fn ignores_auth_and_rate_limit_errors() {
        assert!(!is_vision_unsupported_message(
            "DeepSeek API 401: invalid api key"
        ));
        assert!(!is_vision_unsupported_message(
            "DeepSeek API 429: rate limit"
        ));
    }

    #[test]
    fn ignores_missing_model_errors() {
        assert!(!is_vision_unsupported_message(
            "DeepSeek API 400: model `foo` not found"
        ));
    }

    #[test]
    fn gemini_and_gpt4o_have_native_vision() {
        assert!(primary_model_has_native_vision("gemini-3.5-flash-low"));
        assert!(primary_model_has_native_vision("gemini-3-flash"));
        assert!(primary_model_has_native_vision("gpt-4o"));
        assert!(!primary_model_has_native_vision("deepseek-reasoner"));
        assert!(!primary_model_has_native_vision("deepseek-chat"));
    }
}
