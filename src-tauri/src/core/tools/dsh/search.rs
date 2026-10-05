use crate::core::tools::context::ToolContext;
use crate::core::tools::error::ToolError;
use base64::Engine;
use serde_json::Value;
use std::process::Command;

pub(super) fn execute(ctx: &ToolContext, name: &str, args: &Value) -> Result<String, ToolError> {
    let pattern = super::string_allow_empty(args, "pattern")?;
    if pattern.is_empty() || (name == "glob" && pattern.trim().is_empty()) {
        return Err(ToolError::new("pattern must be a non-empty string"));
    }
    let root = match args.get("path") {
        Some(_) => super::files::path(ctx, super::string(args, "path")?, false, name)?,
        None => ctx.workspace_root.clone(),
    };
    let mut argv = Vec::new();
    if name == "glob" {
        argv.extend([
            "--files".into(),
            format!("--glob={pattern}"),
            "--sort=modified".into(),
            "--no-ignore".into(),
            "--hidden".into(),
        ]);
        for vcs in [".git", ".svn", ".hg", ".bzr", ".jj", ".sl"] {
            argv.push(format!("--glob=!**/{vcs}"));
            argv.push(format!("--glob=!**/{vcs}/**"));
        }
    } else {
        argv.extend(["--json".into(), format!("--regexp={pattern}")]);
        if args.get("include").is_some() {
            let include = super::string(args, "include")?;
            if include.starts_with('!') {
                return Err(ToolError::new("include must be a positive glob filter; negated patterns (\"!…\") are not supported"));
            }
            let mut depth = 0usize;
            for c in include.chars() {
                match c { '{' => depth += 1, '}' => depth = depth.saturating_sub(1), ',' if depth == 0 => return Err(ToolError::new("include must be one glob, not a comma-separated list (use {a,b} alternation instead)")), _ => {} }
            }
            argv.push(format!("--glob={include}"));
        }
    }
    argv.extend(["--".to_string(), root.to_string_lossy().into_owned()]);
    let mut result = None;
    for binary in crate::core::tools::rg::rg_command_candidates() {
        let mut command = Command::new(binary);
        command.args(&argv).current_dir(&ctx.workspace_root);
        match super::shell::capture(ctx, &mut command) {
            Ok(output) => {
                result = Some(output);
                break;
            }
            Err(error) if error.message.contains("os error 2") => continue,
            Err(error) => return Err(error),
        }
    }
    let (code, output, errors) =
        result.ok_or_else(|| ToolError::new("Bundled ripgrep is unavailable"))?;
    if code > 1 {
        return Err(ToolError::new(format!("Search failed: {errors}")));
    }
    if name == "glob" {
        let paths: Vec<_> = output.lines().map(|p| relative(ctx, p)).collect();
        if paths.is_empty() {
            return Ok("No files found".into());
        }
        let shown = paths
            .iter()
            .take(100)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        return Ok(if paths.len() <= 100 {
            shown
        } else {
            format!(
                "{shown}\n\n(Showing 100 of {} paths. {})",
                paths.len(),
                spill(ctx, &paths.join("\n"), "Full sorted result stored at:")?
            )
        });
    }
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    let mut count = 0;
    let mut full = Vec::new();
    for line in output.lines() {
        let record: Value = serde_json::from_str(line)
            .map_err(|_| ToolError::new("Malformed ripgrep JSON output"))?;
        if record["type"] != "match" {
            continue;
        }
        let path = rg_text(&record["data"]["path"])?;
        let path = relative(ctx, &path);
        let number = record["data"]["line_number"]
            .as_u64()
            .ok_or_else(|| ToolError::new("Malformed ripgrep line number"))?;
        let line_text = rg_text(&record["data"]["lines"])?;
        let text = line_text.trim_end_matches(['\n', '\r']);
        let mut end = text.len().min(2000);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let preview = if end < text.len() {
            format!("{} (line truncated)", &text[..end])
        } else {
            text.to_string()
        };
        full.push(format!("{path}\nLine {number}: {text}"));
        count += 1;
        if count > 250 {
            continue;
        }
        if groups.last().is_none_or(|(p, _)| p != &path) {
            groups.push((path, Vec::new()));
        }
        groups
            .last_mut()
            .unwrap()
            .1
            .push(format!("Line {number}: {preview}"));
    }
    if count == 0 {
        return Ok("No matches found".into());
    }
    let body = groups
        .into_iter()
        .map(|(p, rows)| format!("{p}\n{}", rows.join("\n")))
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok(if count <= 250 {
        format!(
            "Found {count} {}\n\n{body}",
            if count == 1 { "match" } else { "matches" }
        )
    } else {
        format!(
            "Found 250 of {count} matches\n\n{body}\n\n({})",
            spill(ctx, &full.join("\n\n"), "Full grep result stored at:")?
        )
    })
}
fn rg_text(value: &Value) -> Result<String, ToolError> {
    if let Some(text) = value["text"].as_str() {
        return Ok(text.to_string());
    }
    if let Some(bytes) = value["bytes"].as_str() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(bytes)
            .map_err(|e| ToolError::new(format!("Invalid ripgrep base64: {e}")))?;
        return Ok(String::from_utf8_lossy(&bytes).into_owned());
    }
    Err(ToolError::new("Malformed ripgrep text/bytes field"))
}
fn relative(ctx: &ToolContext, raw: &str) -> String {
    crate::core::tools::rg::display_hit_path(&ctx.workspace_root, std::path::Path::new(raw))
}
fn spill(ctx: &ToolContext, content: &str, label: &str) -> Result<String, ToolError> {
    let raw = format!(".anya/dsh-output/{}.txt", uuid::Uuid::new_v4());
    let path = super::files::path(ctx, &raw, true, "grep")?;
    crate::core::tools::file_io::atomic_write(&path, content)?;
    Ok(format!(
        "{label} {}. Use read to retrieve it.",
        path.display()
    ))
}
