const SELECTION_MARKER: &str = "\n\n<peek-selection lines=\"";

fn strip_attached_files(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(start) = rest.find("<peek-attached-file") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        if let Some(end) = after.find("/>") {
            // Prefer the earliest closer so a self-closing tag doesn't skip a later body tag.
            let body_close = after.find("</peek-attached-file>");
            match body_close {
                Some(close) if close < end => {
                    rest = &after[close + "</peek-attached-file>".len()..];
                }
                _ => {
                    rest = &after[end + 2..];
                }
            }
            continue;
        }
        if let Some(close) = after.find("</peek-attached-file>") {
            rest = &after[close + "</peek-attached-file>".len()..];
            continue;
        }
        // Malformed — keep the rest verbatim.
        out.push_str(after);
        return out;
    }
    out.push_str(rest);
    out
}

pub fn visible_user_text(content: &str) -> String {
    let without_selection = content
        .split_once(SELECTION_MARKER)
        .map(|(message, _)| message)
        .unwrap_or(content);
    strip_attached_files(without_selection).trim().to_string()
}

/// Extracts informative user text for title generation, preserving file names and key snippets.
pub fn user_text_for_title_context(content: &str) -> String {
    let file_names = extract_attached_filenames(content);
    let selection_snippet = extract_selection_snippet(content);
    let base_text = visible_user_text(content);

    let mut parts = Vec::new();
    if !file_names.is_empty() {
        parts.push(format!("[文件: {}]", file_names.join(", ")));
    }
    if !base_text.is_empty() {
        parts.push(base_text.clone());
    }
    if base_text.chars().count() <= 30 {
        if let Some(snippet) = selection_snippet {
            parts.push(format!("[代码: {}]", snippet));
        }
    }
    parts.join(" ").trim().to_string()
}

fn extract_attached_filenames(content: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("<peek-attached-file") {
        let after = &rest[start..];
        let tag_end = match after.find('>') {
            Some(idx) => idx,
            None => break,
        };
        let tag_content = &after[..tag_end];
        if let Some(name_pos) = tag_content.find("name=\"") {
            let val_start = name_pos + "name=\"".len();
            if let Some(quote_end) = tag_content[val_start..].find('"') {
                let name = &tag_content[val_start..val_start + quote_end];
                if !name.trim().is_empty() && !names.contains(&name.to_string()) {
                    names.push(name.to_string());
                }
            }
        }
        rest = &after[tag_end + 1..];
    }
    names
}

fn extract_selection_snippet(content: &str) -> Option<String> {
    let (_, after) = content.split_once(SELECTION_MARKER)?;
    let body_start = after.find(">\n")? + 2;
    let body_and_rest = &after[body_start..];
    let body = body_and_rest.split_once("\n</peek-selection>")?.0;
    let first_line = body.lines().map(str::trim).find(|line| !line.is_empty())?;
    let snippet: String = first_line.chars().take(50).collect();
    if snippet.is_empty() {
        None
    } else {
        Some(snippet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_selection_attachment_from_rule_and_memory_text() {
        let content =
            "Explain this\n\n<peek-selection lines=\"2\">\nfirst\nsecond\n</peek-selection>";
        assert_eq!(visible_user_text(content), "Explain this");
    }

    #[test]
    fn hides_attached_file_payload_from_titles() {
        let content = "Please review\n\n<peek-attached-file name=\"README.md\" path=\"README.md\">\n# title\n</peek-attached-file>";
        assert_eq!(visible_user_text(content), "Please review");
    }

    #[test]
    fn preserves_regular_user_text() {
        assert_eq!(
            visible_user_text("  regular question  "),
            "regular question"
        );
    }

    #[test]
    fn user_text_for_title_context_preserves_filenames_and_snippet() {
        let content = "Please review\n\n<peek-attached-file name=\"README.md\" path=\"README.md\">\n# title\n</peek-attached-file>";
        assert_eq!(
            user_text_for_title_context(content),
            "[文件: README.md] Please review"
        );

        let selection = "Explain this\n\n<peek-selection lines=\"2\">\nfn compute() -> bool\nsecond\n</peek-selection>";
        assert_eq!(
            user_text_for_title_context(selection),
            "Explain this [代码: fn compute() -> bool]"
        );
    }
}
