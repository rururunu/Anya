//! Parse Codex apply_patch envelopes into hunks.
use std::path::PathBuf;

pub const BEGIN_PATCH_MARKER: &str = "*** Begin Patch";
pub const END_PATCH_MARKER: &str = "*** End Patch";
pub const ADD_FILE_MARKER: &str = "*** Add File: ";
pub const DELETE_FILE_MARKER: &str = "*** Delete File: ";
pub const UPDATE_FILE_MARKER: &str = "*** Update File: ";
pub const MOVE_TO_MARKER: &str = "*** Move to: ";
pub const EOF_MARKER: &str = "*** End of File";
pub const CHANGE_CONTEXT_MARKER: &str = "@@ ";
pub const EMPTY_CHANGE_CONTEXT_MARKER: &str = "@@";

const FORMAT_HINT: &str = "\
Expected Codex apply_patch format, for example:\n\
*** Begin Patch\n\
*** Update File: path/to/file.md\n\
@@\n\
-old line\n\
+new line\n\
*** End Patch\n\
Do not use unified-diff headers like `--- a/file` / `+++ b/file`. \
File ops must start with `*** ` (three asterisks + space), e.g. `*** Update File: README.md`.";

#[derive(Debug, PartialEq, Clone)]
pub enum ParseError {
    InvalidPatch(String),
    InvalidHunk { message: String, line_number: usize },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPatch(msg) => write!(f, "invalid patch: {msg}\n\n{FORMAT_HINT}"),
            Self::InvalidHunk {
                message,
                line_number,
            } => write!(
                f,
                "invalid hunk at line {line_number}, {message}\n\n{FORMAT_HINT}"
            ),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, PartialEq, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum Hunk {
    Add {
        path: PathBuf,
        contents: String,
    },
    Delete {
        path: PathBuf,
    },
    Update {
        path: PathBuf,
        move_path: Option<PathBuf>,
        chunks: Vec<UpdateChunk>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub struct UpdateChunk {
    pub change_context: Option<String>,
    pub old_lines: Vec<String>,
    pub new_lines: Vec<String>,
    pub is_end_of_file: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ApplyPatchArgs {
    pub patch: String,
    pub hunks: Vec<Hunk>,
}

/// Normalize common model mistakes into Codex V4A patch text, then parse.
pub fn parse_patch(patch: &str) -> Result<ApplyPatchArgs, ParseError> {
    let normalized = normalize_patch_text(patch);
    parse_patch_strict(&normalized)
}

fn parse_patch_strict(patch: &str) -> Result<ApplyPatchArgs, ParseError> {
    let lines: Vec<&str> = patch.trim().lines().collect();
    if lines.is_empty() {
        return Err(ParseError::InvalidPatch(
            "The first line of the patch must be '*** Begin Patch'".into(),
        ));
    }
    let first = lines[0].trim();
    let last = lines[lines.len() - 1].trim();
    if first != BEGIN_PATCH_MARKER {
        return Err(ParseError::InvalidPatch(
            "The first line of the patch must be '*** Begin Patch'".into(),
        ));
    }
    if last != END_PATCH_MARKER {
        return Err(ParseError::InvalidPatch(
            "The last line of the patch must be '*** End Patch'".into(),
        ));
    }

    let mut hunks = Vec::new();
    let mut i = 1usize;
    while i < lines.len() - 1 {
        let line = lines[i].trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        if let Some(path) = trimmed.strip_prefix(ADD_FILE_MARKER) {
            let path = PathBuf::from(path.trim());
            i += 1;
            let mut contents = String::new();
            while i < lines.len() - 1 {
                let body = lines[i];
                let body_trim = body.trim_end();
                if body_trim.starts_with("*** ") {
                    break;
                }
                if let Some(rest) = body.strip_prefix('+') {
                    contents.push_str(rest);
                    contents.push('\n');
                } else if !body_trim.is_empty() {
                    return Err(ParseError::InvalidHunk {
                        message: format!("Add File lines must start with '+', got '{}'", body_trim),
                        line_number: i + 1,
                    });
                }
                i += 1;
            }
            if contents.is_empty() {
                return Err(ParseError::InvalidHunk {
                    message: format!("Add file hunk for path '{}' is empty", path.display()),
                    line_number: i,
                });
            }
            hunks.push(Hunk::Add { path, contents });
            continue;
        }
        if let Some(path) = trimmed.strip_prefix(DELETE_FILE_MARKER) {
            hunks.push(Hunk::Delete {
                path: PathBuf::from(path.trim()),
            });
            i += 1;
            continue;
        }
        if let Some(path) = trimmed.strip_prefix(UPDATE_FILE_MARKER) {
            let path = PathBuf::from(path.trim());
            let hunk_line = i + 1;
            i += 1;
            let mut move_path = None;
            if i < lines.len() - 1 {
                if let Some(dest) = lines[i].trim().strip_prefix(MOVE_TO_MARKER) {
                    move_path = Some(PathBuf::from(dest.trim()));
                    i += 1;
                }
            }
            let mut chunks: Vec<UpdateChunk> = Vec::new();
            let mut current: Option<UpdateChunk> = None;
            while i < lines.len() - 1 {
                let raw = lines[i];
                let t = raw.trim_end();
                if t.starts_with("*** ") && !t.starts_with(EOF_MARKER) {
                    break;
                }
                if t == EOF_MARKER {
                    if let Some(chunk) = current.as_mut() {
                        chunk.is_end_of_file = true;
                    }
                    i += 1;
                    continue;
                }
                if t == EMPTY_CHANGE_CONTEXT_MARKER || t.starts_with(CHANGE_CONTEXT_MARKER) {
                    if let Some(chunk) = current.take() {
                        if chunk.old_lines.is_empty() && chunk.new_lines.is_empty() {
                            return Err(ParseError::InvalidHunk {
                                message: "Update hunk does not contain any lines".into(),
                                line_number: i + 1,
                            });
                        }
                        chunks.push(chunk);
                    }
                    let ctx = if t == EMPTY_CHANGE_CONTEXT_MARKER {
                        None
                    } else {
                        Some(t[CHANGE_CONTEXT_MARKER.len()..].to_string())
                    };
                    current = Some(UpdateChunk {
                        change_context: ctx,
                        old_lines: Vec::new(),
                        new_lines: Vec::new(),
                        is_end_of_file: false,
                    });
                    i += 1;
                    continue;
                }
                if current.is_none() {
                    // Implicit chunk without @@ — start one.
                    current = Some(UpdateChunk {
                        change_context: None,
                        old_lines: Vec::new(),
                        new_lines: Vec::new(),
                        is_end_of_file: false,
                    });
                }
                let chunk = current.as_mut().unwrap();
                if let Some(rest) = raw.strip_prefix('+') {
                    chunk.new_lines.push(rest.to_string());
                } else if let Some(rest) = raw.strip_prefix('-') {
                    chunk.old_lines.push(rest.to_string());
                } else if let Some(rest) = raw.strip_prefix(' ') {
                    chunk.old_lines.push(rest.to_string());
                    chunk.new_lines.push(rest.to_string());
                } else if !t.is_empty() {
                    return Err(ParseError::InvalidHunk {
                        message: format!(
                            "Unexpected line found in update hunk: '{t}'. Every line should start with ' ' (context line), '+' (added line), or '-' (removed line)"
                        ),
                        line_number: i + 1,
                    });
                }
                i += 1;
            }
            if let Some(chunk) = current.take() {
                if chunk.old_lines.is_empty() && chunk.new_lines.is_empty() {
                    return Err(ParseError::InvalidHunk {
                        message: "Update hunk does not contain any lines".into(),
                        line_number: i + 1,
                    });
                }
                chunks.push(chunk);
            }
            if chunks.is_empty() && move_path.is_none() {
                return Err(ParseError::InvalidHunk {
                    message: format!("Update file hunk for path '{}' is empty", path.display()),
                    line_number: hunk_line,
                });
            }
            hunks.push(Hunk::Update {
                path,
                move_path,
                chunks,
            });
            continue;
        }
        return Err(ParseError::InvalidHunk {
            message: format!("Unexpected line in patch: '{trimmed}'"),
            line_number: i + 1,
        });
    }

    Ok(ApplyPatchArgs {
        patch: lines.join("\n"),
        hunks,
    })
}

/// Repair common LLM mistakes before strict parsing.
fn normalize_patch_text(raw: &str) -> String {
    let text = strip_code_fences(raw.trim());

    // If this looks like a unified diff (with or without envelope), convert first.
    if looks_like_unified_diff(&text) {
        if let Some(converted) = unified_diff_to_v4a(&text) {
            return converted;
        }
    }

    let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();

    // Fix file-op markers missing the `*** ` prefix.
    for line in &mut lines {
        let trimmed = line.trim().to_string();
        if let Some(rest) = strip_optional_stars(&trimmed, "Add File:") {
            *line = format!("{ADD_FILE_MARKER}{}", rest.trim());
        } else if let Some(rest) = strip_optional_stars(&trimmed, "Delete File:") {
            *line = format!("{DELETE_FILE_MARKER}{}", rest.trim());
        } else if let Some(rest) = strip_optional_stars(&trimmed, "Update File:") {
            *line = format!("{UPDATE_FILE_MARKER}{}", rest.trim());
        } else if let Some(rest) = strip_optional_stars(&trimmed, "Move to:") {
            *line = format!("{MOVE_TO_MARKER}{}", rest.trim());
        } else if matches_marker(&trimmed, "Begin Patch") {
            *line = BEGIN_PATCH_MARKER.to_string();
        } else if matches_marker(&trimmed, "End Patch") {
            *line = END_PATCH_MARKER.to_string();
        } else if matches_marker(&trimmed, "End of File") {
            *line = EOF_MARKER.to_string();
        }
    }

    // Drop unified-diff path headers if they still remain inside a V4A envelope.
    lines.retain(|line| {
        let t = line.trim();
        !(t.starts_with("--- ") || t.starts_with("+++ ") || t.starts_with("diff --git "))
    });

    let body = lines.join("\n");
    ensure_envelope(&body)
}

fn strip_optional_stars<'a>(trimmed: &'a str, marker: &str) -> Option<&'a str> {
    for prefix in [
        format!("*** {marker}"),
        format!("** {marker}"),
        format!("* {marker}"),
        marker.to_string(),
    ] {
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            return Some(rest);
        }
    }
    None
}

fn matches_marker(trimmed: &str, marker: &str) -> bool {
    let stars = format!("*** {marker}");
    trimmed == marker
        || trimmed == stars
        || trimmed == format!("** {marker}")
        || trimmed == format!("* {marker}")
        || trimmed.eq_ignore_ascii_case(marker)
        || trimmed.eq_ignore_ascii_case(&stars)
}

fn strip_code_fences(raw: &str) -> String {
    let mut lines: Vec<&str> = raw.lines().collect();
    if lines
        .first()
        .is_some_and(|l| l.trim_start().starts_with("```"))
    {
        lines.remove(0);
    }
    if lines.last().is_some_and(|l| l.trim() == "```") {
        lines.pop();
    }
    lines.join("\n")
}

fn ensure_envelope(body: &str) -> String {
    let trimmed = body.trim();
    let has_begin = trimmed.lines().next().is_some_and(|l| {
        l.trim() == BEGIN_PATCH_MARKER || l.trim().eq_ignore_ascii_case("*** begin patch")
    });
    let has_end = trimmed.lines().next_back().is_some_and(|l| {
        l.trim() == END_PATCH_MARKER || l.trim().eq_ignore_ascii_case("*** end patch")
    });
    match (has_begin, has_end) {
        (true, true) => trimmed.to_string(),
        (true, false) => format!("{trimmed}\n{END_PATCH_MARKER}"),
        (false, true) => format!("{BEGIN_PATCH_MARKER}\n{trimmed}"),
        (false, false) => format!("{BEGIN_PATCH_MARKER}\n{trimmed}\n{END_PATCH_MARKER}"),
    }
}

fn looks_like_unified_diff(text: &str) -> bool {
    let mut has_minus = false;
    let mut has_plus = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with("diff --git ") {
            return true;
        }
        if t.starts_with("--- ") || t.starts_with("---\t") {
            has_minus = true;
        }
        if t.starts_with("+++ ") || t.starts_with("+++\t") {
            has_plus = true;
        }
        if has_minus && has_plus {
            return true;
        }
    }
    false
}

fn strip_diff_path(path: &str) -> String {
    let path = path.trim();
    let path = path
        .strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path);
    path.trim().to_string()
}

/// Parse a unified-diff body line. Tolerates indent before the `+`/`-`/` ` marker.
fn diff_body_line(line: &str) -> Option<(char, &str)> {
    let s = line.trim_end();
    let bytes = s.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() && bytes[idx] == b' ' {
        idx += 1;
    }
    if idx >= bytes.len() {
        return None;
    }
    match bytes[idx] {
        b'+' | b'-' => {
            let kind = bytes[idx] as char;
            Some((kind, &s[idx + 1..]))
        }
        // Indentation spaces were skipped and content doesn't begin with +/- —
        // treat as a context line (marker = space).
        _ if idx > 0 => Some((' ', &s[idx..])),
        _ => None,
    }
}

fn unified_diff_to_v4a(text: &str) -> Option<String> {
    // Work on the inner body if an envelope is already present.
    let mut lines: Vec<&str> = text.lines().collect();
    if lines
        .first()
        .is_some_and(|l| l.trim() == BEGIN_PATCH_MARKER)
    {
        lines.remove(0);
    }
    if lines.last().is_some_and(|l| l.trim() == END_PATCH_MARKER) {
        lines.pop();
    }

    let mut out = String::from(BEGIN_PATCH_MARKER);
    out.push('\n');
    let mut i = 0usize;
    let mut produced = false;

    while i < lines.len() {
        let line = lines[i].trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("diff --git ") || trimmed.starts_with("index ")
        {
            i += 1;
            continue;
        }

        // Already-correct V4A markers inside mixed content.
        if trimmed.starts_with("*** Add File:")
            || trimmed.starts_with("*** Update File:")
            || trimmed.starts_with("*** Delete File:")
        {
            // Keep remaining V4A content as-is.
            for rest in &lines[i..] {
                if rest.trim() == END_PATCH_MARKER {
                    break;
                }
                out.push_str(rest);
                out.push('\n');
            }
            produced = true;
            break;
        }

        if let Some(old_path) = trimmed.strip_prefix("--- ") {
            let old_path = strip_diff_path(old_path.split('\t').next().unwrap_or(old_path));
            i += 1;
            let new_line = lines.get(i).map(|l| l.trim()).unwrap_or("");
            let new_path = new_line
                .strip_prefix("+++ ")
                .map(|p| strip_diff_path(p.split('\t').next().unwrap_or(p)))
                .unwrap_or_default();
            if new_line.starts_with("+++ ") {
                i += 1;
            }

            if old_path == "/dev/null" || old_path == "nul" {
                // Add file
                out.push_str(ADD_FILE_MARKER);
                out.push_str(&new_path);
                out.push('\n');
                while i < lines.len() {
                    let t = lines[i].trim();
                    if t.starts_with("--- ") || t.starts_with("diff --git ") {
                        break;
                    }
                    if t.starts_with("@@") {
                        i += 1;
                        continue;
                    }
                    if let Some(('+', rest)) = diff_body_line(lines[i]) {
                        out.push('+');
                        out.push_str(rest);
                        out.push('\n');
                    }
                    i += 1;
                }
                produced = true;
                continue;
            }
            if new_path == "/dev/null" || new_path == "nul" {
                out.push_str(DELETE_FILE_MARKER);
                out.push_str(&old_path);
                out.push('\n');
                while i < lines.len() {
                    let t = lines[i].trim_end();
                    if t.starts_with("--- ") || t.starts_with("diff --git ") {
                        break;
                    }
                    i += 1;
                }
                produced = true;
                continue;
            }

            let path = if new_path.is_empty() {
                old_path
            } else {
                new_path
            };
            out.push_str(UPDATE_FILE_MARKER);
            out.push_str(&path);
            out.push('\n');
            // Emit each @@ as a fresh chunk with a bare @@ marker (drop hunk counts).
            while i < lines.len() {
                let l = lines[i];
                let t = l.trim();
                if t.starts_with("--- ") || t.starts_with("diff --git ") {
                    break;
                }
                if t.starts_with("@@") {
                    out.push_str("@@\n");
                    i += 1;
                    continue;
                }
                if let Some((kind, rest)) = diff_body_line(l) {
                    out.push(kind);
                    out.push_str(rest);
                    out.push('\n');
                    i += 1;
                    continue;
                }
                if t.is_empty() {
                    i += 1;
                    continue;
                }
                // Unknown — stop this file.
                break;
            }
            produced = true;
            continue;
        }

        i += 1;
    }

    if !produced {
        return None;
    }
    out.push_str(END_PATCH_MARKER);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_add_update_delete() {
        let args = parse_patch(
            "*** Begin Patch\n\
             *** Add File: path/add.py\n\
             +abc\n\
             +def\n\
             *** Delete File: path/delete.py\n\
             *** Update File: path/update.py\n\
             *** Move to: path/update2.py\n\
             @@ def f():\n\
             -    pass\n\
             +    return 123\n\
             *** End Patch",
        )
        .unwrap();
        assert_eq!(args.hunks.len(), 3);
        match &args.hunks[0] {
            Hunk::Add { path, contents } => {
                assert_eq!(path, PathBuf::from("path/add.py").as_path());
                assert_eq!(contents, "abc\ndef\n");
            }
            _ => panic!("expected add"),
        }
        assert!(matches!(args.hunks[1], Hunk::Delete { .. }));
        match &args.hunks[2] {
            Hunk::Update {
                path,
                move_path,
                chunks,
            } => {
                assert_eq!(path, PathBuf::from("path/update.py").as_path());
                assert_eq!(
                    move_path.as_ref().unwrap(),
                    &PathBuf::from("path/update2.py")
                );
                assert_eq!(chunks.len(), 1);
                assert_eq!(chunks[0].change_context.as_deref(), Some("def f():"));
                assert_eq!(chunks[0].old_lines, vec!["    pass".to_string()]);
                assert_eq!(chunks[0].new_lines, vec!["    return 123".to_string()]);
            }
            _ => panic!("expected update"),
        }
    }

    #[test]
    fn accepts_missing_stars_on_update_file() {
        let args = parse_patch(
            "*** Begin Patch\n\
             Update File: README.md\n\
             @@\n\
             -old\n\
             +new\n\
             *** End Patch",
        )
        .unwrap();
        assert!(matches!(
            &args.hunks[0],
            Hunk::Update { path, .. } if path == PathBuf::from("README.md").as_path()
        ));
    }

    #[test]
    fn accepts_unified_diff_headers() {
        let args = parse_patch(
            "*** Begin Patch\n--- a/README.md\n+++ b/README.md\n@@ -1,2 +1,2 @@\n keep\n-old\n+new\n*** End Patch",
        )
        .unwrap();
        match &args.hunks[0] {
            Hunk::Update { path, chunks, .. } => {
                assert_eq!(path, PathBuf::from("README.md").as_path());
                assert_eq!(
                    chunks[0].old_lines,
                    vec!["keep".to_string(), "old".to_string()]
                );
                assert_eq!(
                    chunks[0].new_lines,
                    vec!["keep".to_string(), "new".to_string()]
                );
            }
            _ => panic!("expected update"),
        }
    }

    #[test]
    fn accepts_bare_unified_diff_without_envelope() {
        let args = parse_patch(
            "--- a/src/app.rs\n\
+++ b/src/app.rs\n\
@@\n\
-foo\n\
+bar\n",
        )
        .unwrap();
        assert!(matches!(
            &args.hunks[0],
            Hunk::Update { path, .. } if path == PathBuf::from("src/app.rs").as_path()
        ));
    }
}
