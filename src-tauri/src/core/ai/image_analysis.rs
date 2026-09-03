//! Persistable `<peek-image-analysis>` tags next to `![image](...)` markdown.

use regex::Regex;
use std::sync::OnceLock;

const CLOSE_TAG: &str = "</peek-image-analysis>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageAnalysisBlock {
    pub model: String,
    pub text: String,
}

fn image_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"!\[image\]\((.*?)\)").expect("image regex"))
}

fn analysis_open_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?s)^\s*<peek-image-analysis\s+model="([^"]*)">\s*(.*?)\s*</peek-image-analysis>"#,
        )
        .expect("analysis open regex")
    })
}

pub fn format_analysis_tag(model: &str, text: &str) -> String {
    let safe_text = text.replace(CLOSE_TAG, "");
    format!(
        "<peek-image-analysis model=\"{}\">\n{}\n</peek-image-analysis>",
        model.trim(),
        safe_text.trim()
    )
}

/// If `image_markdown` is followed (after whitespace) by an analysis tag, return it.
pub fn analysis_after_image(content: &str, image_markdown: &str) -> Option<ImageAnalysisBlock> {
    let start = content.find(image_markdown)?;
    let after = &content[start + image_markdown.len()..];
    let caps = analysis_open_regex().captures(after)?;
    Some(ImageAnalysisBlock {
        model: caps.get(1)?.as_str().to_string(),
        text: caps.get(2)?.as_str().to_string(),
    })
}

/// Failed analyses must not be treated as cached successes (they should be retried).
pub fn is_usable_analysis(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with("Analysis failed:")
        && !trimmed.starts_with("分析失败:")
}

/// Like [`analysis_after_image`], but ignores failed analysis placeholders.
pub fn usable_analysis_after_image(
    content: &str,
    image_markdown: &str,
) -> Option<ImageAnalysisBlock> {
    let block = analysis_after_image(content, image_markdown)?;
    if is_usable_analysis(&block.text) {
        Some(block)
    } else {
        None
    }
}

/// Remove the analysis tag immediately following `image_markdown`, if present.
pub fn remove_analysis_after_image(content: &str, image_markdown: &str) -> String {
    let Some(start) = content.find(image_markdown) else {
        return content.to_string();
    };
    let after_img = start + image_markdown.len();
    let after = &content[after_img..];
    let Some(matched) = analysis_open_regex().find(after) else {
        return content.to_string();
    };
    let mut out = String::with_capacity(content.len());
    out.push_str(&content[..after_img]);
    out.push_str(&after[matched.end()..]);
    out
}

/// Insert an analysis tag immediately after `image_markdown` if a usable one is not already present.
pub fn insert_analysis_after_image(
    content: &str,
    image_markdown: &str,
    model: &str,
    text: &str,
) -> String {
    let content = if usable_analysis_after_image(content, image_markdown).is_some() {
        return content.to_string();
    } else if analysis_after_image(content, image_markdown).is_some() {
        remove_analysis_after_image(content, image_markdown)
    } else {
        content.to_string()
    };
    let Some(start) = content.find(image_markdown) else {
        return content;
    };
    let end = start + image_markdown.len();
    let tag = format_analysis_tag(model, text);
    let mut out = String::with_capacity(content.len() + tag.len() + 2);
    out.push_str(&content[..end]);
    out.push_str("\n\n");
    out.push_str(&tag);
    out.push_str(&content[end..]);
    out
}

/// Replace each image (+ optional trailing analysis tag) with `[Image analysis:…]` for the main model.
pub fn replace_images_with_analysis_text(content: &str) -> String {
    let mut result = String::new();
    let mut last = 0usize;
    for caps in image_regex().captures_iter(content) {
        let full = caps.get(0).expect("full match");
        let image_markdown = full.as_str();
        result.push_str(&content[last..full.start()]);

        let (desc, consume_end) = if let Some(block) = analysis_after_image(content, image_markdown)
        {
            let after_img = full.end();
            let after_slice = &content[after_img..];
            let matched = analysis_open_regex()
                .find(after_slice)
                .expect("analysis present");
            let desc = if is_usable_analysis(&block.text) {
                block.text
            } else {
                "(Image analysis failed, please resend)".to_string()
            };
            (desc, after_img + matched.end())
        } else {
            ("(No analysis result)".to_string(), full.end())
        };

        result.push_str(&format!("\n[Image analysis:\n{}\n]\n", desc.trim()));
        last = consume_end;
    }
    result.push_str(&content[last..]);
    // Drop leftover orphan analysis tags if any
    strip_orphan_analysis_tags(&result)
}

fn strip_orphan_analysis_tags(content: &str) -> String {
    let re = Regex::new(
        r#"(?s)\s*<peek-image-analysis\s+model="[^"]*">\s*.*?\s*</peek-image-analysis>"#,
    )
    .expect("strip regex");
    re.replace_all(content, "").into_owned()
}

/// Remove persisted `<peek-image-analysis>` blocks before sending native image bytes upstream.
pub fn strip_image_analysis_tags(content: &str) -> String {
    strip_orphan_analysis_tags(content)
}

/// Segment of user message content for multimodal assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageContentSegment {
    Text(String),
    ImagePayload(String),
}

/// Split user content into alternating text and `![image](payload)` segments.
pub fn split_image_content(content: &str) -> Vec<ImageContentSegment> {
    let content = crate::core::ai::image_gen::strip_edit_region_images(content);
    let mut segments = Vec::new();
    if !content.contains("![image](") {
        let cleaned = strip_image_analysis_tags(&content);
        if !cleaned.trim().is_empty() {
            segments.push(ImageContentSegment::Text(cleaned));
        }
        return segments;
    }

    let mut last = 0usize;
    for caps in image_regex().captures_iter(&content) {
        let full = caps.get(0).expect("full match");
        let before = strip_image_analysis_tags(&content[last..full.start()]);
        if !before.trim().is_empty() {
            segments.push(ImageContentSegment::Text(before));
        }

        if let Some(payload) = caps.get(1) {
            segments.push(ImageContentSegment::ImagePayload(
                payload.as_str().to_string(),
            ));
        }

        let mut end = full.end();
        if let Some(matched) = analysis_open_regex().find(&content[end..]) {
            end += matched.end();
        }
        last = end;
    }

    let after = strip_image_analysis_tags(&content[last..]);
    if !after.trim().is_empty() {
        segments.push(ImageContentSegment::Text(after));
    }
    segments
}

/// Decode `![image](...)` payload into Gemini `inlineData` fields `(mime_type, base64)`.
pub fn decode_image_inline_payload(payload: &str) -> Result<(String, String), String> {
    let payload = payload.trim();
    if payload.is_empty() {
        return Err("Image path is empty".into());
    }

    if payload.starts_with("data:") {
        let rest = payload
            .strip_prefix("data:")
            .ok_or_else(|| "Invalid image data URL".to_string())?;
        let (meta, data) = rest
            .split_once(',')
            .ok_or_else(|| "Invalid image data URL: missing base64 payload".to_string())?;
        let mime_type = meta
            .split(';')
            .next()
            .unwrap_or("image/png")
            .trim()
            .to_string();
        if data.trim().is_empty() {
            return Err("Image data URL contains no base64 payload".into());
        }
        return Ok((mime_type, data.trim().to_string()));
    }

    let bytes = crate::core::ai::image_gen::decode_image_source(payload)
        .map_err(|error| format!("Failed to read image file {payload}: {error}"))?;
    if bytes.is_empty() {
        return Err("Image file is empty".into());
    }

    let path_for_ext = payload.strip_prefix("path:").unwrap_or(payload);
    let mime_type = if path_for_ext.ends_with(".jpg") || path_for_ext.ends_with(".jpeg") {
        "image/jpeg"
    } else if path_for_ext.ends_with(".gif") {
        "image/gif"
    } else if path_for_ext.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png"
    }
    .to_string();

    use base64::{engine::general_purpose, Engine as _};
    Ok((mime_type, general_purpose::STANDARD.encode(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_existing_tag_after_image() {
        let content = "hi\n![image](data:image/png;base64,abc)\n\n<peek-image-analysis model=\"gpt-4o\">\ndesc here\n</peek-image-analysis>\n";
        let block = analysis_after_image(content, "![image](data:image/png;base64,abc)").unwrap();
        assert_eq!(block.model, "gpt-4o");
        assert_eq!(block.text.trim(), "desc here");
    }

    #[test]
    fn insert_then_replace_for_api() {
        let content = "hi\n![image](data:image/png;base64,abc)\n";
        let stored = insert_analysis_after_image(
            content,
            "![image](data:image/png;base64,abc)",
            "gpt-4o",
            "a cat",
        );
        assert!(stored.contains("<peek-image-analysis model=\"gpt-4o\">"));
        assert!(stored.contains("![image](data:image/png;base64,abc)"));
        let api = replace_images_with_analysis_text(&stored);
        assert!(!api.contains("![image]("));
        assert!(!api.contains("peek-image-analysis"));
        assert!(api.contains("[Image analysis:\na cat\n]"));
    }

    #[test]
    fn missing_tag_returns_none() {
        let content = "![image](data:image/png;base64,abc)\n";
        assert!(analysis_after_image(content, "![image](data:image/png;base64,abc)").is_none());
    }

    #[test]
    fn failed_analysis_is_not_usable() {
        assert!(!is_usable_analysis(
            "Analysis failed: Multimodal API returned 502: Bad Gateway"
        ));
        assert!(!is_usable_analysis("分析失败: 502"));
        assert!(!is_usable_analysis(""));
        assert!(is_usable_analysis("a cat sitting on a mat"));
    }

    #[test]
    fn usable_analysis_after_image_skips_failures() {
        let img = "![image](data:image/png;base64,abc)";
        let content = format!(
            "{img}\n\n<peek-image-analysis model=\"gpt-4o\">\nAnalysis failed: 502\n</peek-image-analysis>\n"
        );
        assert!(usable_analysis_after_image(&content, img).is_none());
        assert!(analysis_after_image(&content, img).is_some());
    }

    #[test]
    fn remove_failed_analysis_allows_reinsert() {
        let img = "![image](data:image/png;base64,abc)";
        let content = format!(
            "hi\n{img}\n\n<peek-image-analysis model=\"gpt-4o\">\nAnalysis failed: 502\n</peek-image-analysis>\nmore"
        );
        let cleaned = remove_analysis_after_image(&content, img);
        assert!(!cleaned.contains("peek-image-analysis"));
        assert!(cleaned.contains(img));
        assert!(cleaned.contains("more"));
        let restored = insert_analysis_after_image(&cleaned, img, "gpt-4o", "ok desc");
        assert!(restored.contains("ok desc"));
    }

    #[test]
    fn split_image_content_skips_analysis_tags() {
        let img = "![image](data:image/png;base64,abc)";
        let content = format!(
            "描述它\n{img}\n\n<peek-image-analysis model=\"gpt-4o\">\nwrong desc\n</peek-image-analysis>"
        );
        let segments = split_image_content(&content);
        assert_eq!(
            segments,
            vec![
                ImageContentSegment::Text("描述它\n".to_string()),
                ImageContentSegment::ImagePayload("data:image/png;base64,abc".to_string()),
            ]
        );
    }

    #[test]
    fn decode_image_inline_payload_parses_data_url() {
        let (mime, data) =
            decode_image_inline_payload("data:image/jpeg;base64,/9j/abc").expect("decode");
        assert_eq!(mime, "image/jpeg");
        assert_eq!(data, "/9j/abc");
    }
}
