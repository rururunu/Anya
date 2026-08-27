//! Markdown image helpers shared by Images API, chat multimodal, and vision.
//!
//! Kept separate from the HTTP Images adapter so chat/vision can strip or
//! extract refs without pulling request/serialization concerns.

/// Cap on reference images attached to one Images / edits call.
/// Keep in sync with `image_gen::MAX_IMAGES`.
pub const MAX_IMAGE_REFS: usize = 4;

pub fn extract_image_refs(markdown: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = markdown[search_from..].find("](") {
        let bracket = search_from + rel;
        let start = bracket + 2;
        let Some(end_rel) = markdown[start..].find(')') else {
            break;
        };
        let url = markdown[start..start + end_rel].trim();
        // Skip display-only paint previews (`![edit-region](...)`); only
        // full-brightness `![image](...)` (and other alts) feed the edits API.
        let alt = image_markdown_alt(&markdown[..bracket]);
        if alt.as_deref() != Some("edit-region")
            && is_image_ref(url)
            && !out.iter().any(|item| item == url)
        {
            out.push(url.to_string());
            if out.len() >= MAX_IMAGE_REFS {
                break;
            }
        }
        search_from = start + end_rel + 1;
    }
    out
}

/// Alt text immediately before `](` in `![alt](url)`, if any.
fn image_markdown_alt(before_close_bracket: &str) -> Option<String> {
    let bytes = before_close_bracket.as_bytes();
    let mut i = bytes.len();
    while i > 0 {
        i -= 1;
        if bytes[i] == b'[' {
            if i > 0 && bytes[i - 1] == b'!' {
                return Some(before_close_bracket[i + 1..].to_string());
            }
            return None;
        }
        // Stop if we walk past a newline — not an image token.
        if bytes[i] == b'\n' {
            return None;
        }
    }
    None
}

/// Drop display-only paint previews before content is sent to chat/vision APIs.
/// Stored messages keep `![edit-region](...)` so the UI can show the selection.
pub fn strip_edit_region_images(content: &str) -> String {
    if !content.contains("![edit-region](") {
        return content.to_string();
    }
    let Ok(re) = regex::Regex::new(r"!\[edit-region\]\([^)]*\)") else {
        return content.to_string();
    };
    re.replace_all(content, "")
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn merge_reference_sources(user_markdown: &str, example_image: Option<&str>) -> Vec<String> {
    let mut refs = extract_image_refs(user_markdown);
    if let Some(example) = example_image.map(str::trim).filter(|value| !value.is_empty()) {
        if is_image_ref(example) && !refs.iter().any(|item| item == example) {
            refs.push(example.to_string());
        }
    }
    refs.truncate(MAX_IMAGE_REFS);
    refs
}

fn is_image_ref(url: &str) -> bool {
    let value = url.trim();
    if value.starts_with("data:image/") || value.starts_with("path:") {
        return true;
    }
    if value.starts_with("http://") || value.starts_with("https://") {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".webp")
        || lower.ends_with(".gif")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_markdown_image_refs_skips_http_urls() {
        let refs = extract_image_refs(
            "see ![image](data:image/png;base64,abc) then ![cat](path:/tmp/a.png) and ![x](https://cdn.example/a.png)",
        );
        assert_eq!(
            refs,
            vec![
                "data:image/png;base64,abc".to_string(),
                "path:/tmp/a.png".to_string(),
            ]
        );
    }

    #[test]
    fn extract_skips_edit_region_display_previews() {
        let refs = extract_image_refs(
            "![edit-region](data:image/png;base64,dimmed) ![image](data:image/png;base64,bright)",
        );
        assert_eq!(refs, vec!["data:image/png;base64,bright".to_string()]);
    }

    #[test]
    fn strip_edit_region_keeps_edits_reference() {
        let cleaned = strip_edit_region_images(
            "![edit-region](data:image/png;base64,dimmed)\n![image](data:image/png;base64,bright)\nedit the painted area",
        );
        assert!(!cleaned.contains("edit-region"));
        assert!(!cleaned.contains("dimmed"));
        assert!(cleaned.contains("![image](data:image/png;base64,bright)"));
        assert!(cleaned.contains("edit the painted area"));
    }

    #[test]
    fn merge_keeps_attachments_ahead_of_example() {
        let refs = merge_reference_sources(
            "![image](path:/tmp/a.png)",
            Some("data:image/png;base64,xx"),
        );
        assert_eq!(
            refs,
            vec![
                "path:/tmp/a.png".to_string(),
                "data:image/png;base64,xx".to_string(),
            ]
        );
    }
}
