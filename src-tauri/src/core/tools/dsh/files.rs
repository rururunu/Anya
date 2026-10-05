//! Rust implementation of dsh-tool-fs's native read/write/edit contract.
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::core::tools::context::ToolContext;
use crate::core::tools::error::ToolError;
use crate::core::tools::file_io::atomic_write;
use crate::core::tools::path::resolve_tool_path;
use crate::core::tools::path_permission::PathAccess;
use crate::core::tools::preview::{unified_diff, ChangeKind, ToolPreview};

type Observations = HashMap<(String, PathBuf), Option<Vec<u8>>>;

pub(super) fn read_image(ctx: &ToolContext, args: &Value) -> Result<String, ToolError> {
    let target = path(ctx, super::string(args, "file_path")?, false, "read_image")?;
    let size = fs::metadata(&target)?.len();
    if size == 0 || size > 20 * 1024 * 1024 {
        return Err(ToolError::new("Image must be non-empty and at most 20 MiB"));
    }
    let data = fs::read(&target)?;
    let format = image::guess_format(&data).map_err(|e| ToolError::new(e.to_string()))?;
    if !matches!(
        format,
        image::ImageFormat::Png
            | image::ImageFormat::Jpeg
            | image::ImageFormat::WebP
            | image::ImageFormat::Gif
    ) {
        return Err(ToolError::new("Unsupported image format"));
    }
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(data), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|e| ToolError::new(format!("Cannot read image: {e}")))?;
    let original = (decoded.width(), decoded.height());
    let image = if original.0 > 2048 || original.1 > 2048 {
        decoded.thumbnail(2048, 2048)
    } else {
        decoded
    };
    ctx.ensure_not_cancelled()?;
    // Safe filenames keep parentheses and other markdown delimiters out of image references.
    let raw = format!(".anya/dsh-images/{}.png", uuid::Uuid::new_v4());
    let snapshot = path(ctx, &raw, true, "read_image")?;
    fs::create_dir_all(
        snapshot
            .parent()
            .ok_or_else(|| ToolError::new("Image has no parent"))?,
    )?;
    image
        .save_with_format(&snapshot, image::ImageFormat::Png)
        .map_err(|e| ToolError::new(e.to_string()))?;
    let scaled = if (image.width(), image.height()) != original {
        format!(" (downscaled from {}x{} px; multiply coordinates by {:.3} horizontally and {:.3} vertically to locate features in the original file)", original.0, original.1, original.0 as f64 / image.width() as f64, original.1 as f64 / image.height() as f64)
    } else {
        String::new()
    };
    Ok(format!(
        "Image {}: {}x{} px{scaled}\n![image]({})",
        target.display(),
        image.width(),
        image.height(),
        snapshot.display()
    ))
}
fn observations() -> &'static Mutex<Observations> {
    static STORE: OnceLock<Mutex<Observations>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}
pub(super) fn clear_session(session_id: &str) {
    let child_prefix = format!("{session_id}-sub-");
    if let Ok(mut store) = observations().lock() {
        store
            .retain(|(session, _), _| session != session_id && !session.starts_with(&child_prefix));
    }
}
fn version(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

pub(super) fn path(
    ctx: &ToolContext,
    raw: &str,
    write: bool,
    name: &str,
) -> Result<PathBuf, ToolError> {
    let access = if write {
        PathAccess::Write
    } else {
        PathAccess::Read
    };
    let lexical = resolve_tool_path(ctx, raw, access, name)?;
    // Re-authorize a canonical path so junctions cannot hide an external target.
    let actual = if lexical.exists() {
        fs::canonicalize(&lexical)?
    } else {
        let mut ancestor = lexical.as_path();
        while !ancestor.exists() {
            ancestor = ancestor
                .parent()
                .ok_or_else(|| ToolError::new("Path has no existing parent"))?;
        }
        fs::canonicalize(ancestor)?.join(
            lexical
                .strip_prefix(ancestor)
                .map_err(|e| ToolError::new(e.to_string()))?,
        )
    };
    // Remove Windows' verbatim prefix before the existing lexical permission check.
    let display = actual.to_string_lossy();
    let plain = if let Some(unc) = display.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        display
            .strip_prefix(r"\\?\")
            .unwrap_or(&display)
            .to_string()
    };
    resolve_tool_path(ctx, &plain, access, name)
}

pub(super) fn read(ctx: &ToolContext, args: &Value) -> Result<String, ToolError> {
    let raw = super::string(args, "file_path")?;
    let offset = super::positive(args, "offset", 1, usize::MAX)?;
    let limit = super::positive(args, "limit", 2000, 2000)?;
    let target = path(ctx, raw, false, "read")?;
    let key = (ctx.session_id.clone(), target.clone());
    let file = match fs::File::open(&target) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            observations()
                .lock()
                .map_err(|_| ToolError::new("Observation lock failed"))?
                .insert(key, None);
            return Err(error.into());
        }
        Err(error) => return Err(error.into()),
    };
    if !file.metadata()?.is_file() {
        return Err(ToolError::new("read requires a regular file"));
    }
    let mut reader = BufReader::new(file);
    let mut digest = Sha256::new();
    let mut bytes = Vec::new();
    let mut total = 0;
    let mut lines = Vec::new();
    let mut retained = 0;
    let mut capped = false;
    let mut line_bytes = 0usize;
    // Keep only a bounded window while hashing the complete observed version.
    loop {
        ctx.ensure_not_cancelled()?;
        let chunk = reader.fill_buf()?;
        if chunk.is_empty() && line_bytes == 0 {
            break;
        }
        if !chunk.is_empty() {
            let count = chunk
                .iter()
                .position(|b| *b == b'\n')
                .map(|n| n + 1)
                .unwrap_or(chunk.len());
            let complete = chunk[count - 1] == b'\n';
            digest.update(&chunk[..count]);
            line_bytes = line_bytes.saturating_add(count);
            let keep = count.min(8004usize.saturating_sub(bytes.len()));
            bytes.extend_from_slice(&chunk[..keep]);
            reader.consume(count);
            if !complete {
                continue;
            }
        }
        total += 1;
        let long_line = line_bytes > 8004;
        line_bytes = 0;
        if total < offset || lines.len() == limit || capped {
            bytes.clear();
            continue;
        }
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
        }
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
        let text = String::from_utf8_lossy(&bytes);
        // JavaScript's length/substr counts UTF-16 code units.
        let units: Vec<u16> = text.encode_utf16().collect();
        let text = if units.len() > 2000 || long_line {
            format!(
                "{}... (line truncated to 2000 chars)",
                String::from_utf16_lossy(&units[..2000])
            )
        } else {
            text.into_owned()
        };
        let size = text.len() + usize::from(!lines.is_empty());
        if retained + size > 51200 {
            capped = true;
            bytes.clear();
            continue;
        }
        retained += size;
        lines.push((total, text));
        bytes.clear();
    }
    if offset > total && !(total == 0 && offset == 1) {
        return Err(ToolError::new(format!(
            "offset {offset} is out of range for \"{}\" ({total} lines)",
            target.display()
        )));
    }
    observations()
        .lock()
        .map_err(|_| ToolError::new("Observation lock failed"))?
        .insert(key, Some(digest.finalize().to_vec()));
    let end = lines
        .last()
        .map(|(n, _)| *n)
        .unwrap_or(offset.saturating_sub(1));
    let footer = if capped {
        format!(
            "(Output capped. Showing lines {offset}-{end}. Use offset={} to continue.)",
            end + 1
        )
    } else if end < total {
        format!(
            "(Showing lines {offset}-{end} of {total}. Use offset={} to continue.)",
            end + 1
        )
    } else {
        format!("(End of file - total {total} lines)")
    };
    let body = if lines.is_empty() {
        footer
    } else {
        format!(
            "{}\n\n{footer}",
            lines
                .into_iter()
                .map(|(n, t)| format!("{n}: {t}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    Ok(format!(
        "<path>{}</path>\n<type>file</type>\n<content>\n{body}\n</content>",
        target.display()
    ))
}

fn changed(args: &Value, before: Option<&str>, edit: bool) -> Result<String, ToolError> {
    if !edit {
        return Ok(super::string_allow_empty(args, "content")?.to_string());
    }
    let before = before.ok_or_else(|| ToolError::new("File does not exist"))?;
    let old = super::string_allow_empty(args, "old_string")?;
    let new = super::string_allow_empty(args, "new_string")?;
    if old.is_empty() {
        return Err(ToolError::new("old_string must be a non-empty string"));
    }
    if old == new {
        return Err(ToolError::new("old_string and new_string must differ"));
    }
    let count = before.matches(old).count();
    if count == 0 {
        return Err(ToolError::new("old_string was not found in the file"));
    }
    let all = args["replace_all"].as_bool().unwrap_or(false);
    if count > 1 && !all {
        return Err(ToolError::new(format!(
            "old_string appears {count} times; provide more context or set replace_all to true"
        )));
    }
    Ok(if all {
        before.replace(old, new)
    } else {
        before.replacen(old, new, 1)
    })
}

fn ensure_observed(
    store: &Observations,
    ctx: &ToolContext,
    target: &PathBuf,
    bytes: Option<&[u8]>,
) -> Result<(), ToolError> {
    let known = store.get(&(ctx.session_id.clone(), target.clone()));
    let current = bytes.map(version);
    if bytes.is_some() && known.is_none() {
        return Err(ToolError::new("Read the file before modifying it."));
    }
    if known.is_some_and(|previous| *previous != current) {
        return Err(ToolError::new(
            "File changed since it was last read or modified. Read it again before retrying.",
        ));
    }
    Ok(())
}

pub(super) fn preview(
    ctx: &ToolContext,
    args: &Value,
    edit: bool,
) -> Result<ToolPreview, ToolError> {
    let target = path(
        ctx,
        super::string(args, "file_path")?,
        true,
        if edit { "edit" } else { "write" },
    )?;
    let bytes = match fs::read(&target) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    let store = observations()
        .lock()
        .map_err(|_| ToolError::new("Observation lock failed"))?;
    ensure_observed(&store, ctx, &target, bytes.as_deref())?;
    let old = bytes
        .as_deref()
        .map(|b| String::from_utf8_lossy(b).into_owned());
    let new = changed(args, old.as_deref(), edit)?;
    let display = target.to_string_lossy().to_string();
    Ok(ToolPreview {
        path: display.clone(),
        affected_paths: vec![],
        kind: if old.is_some() {
            ChangeKind::Modify
        } else {
            ChangeKind::Create
        },
        unified_diff: unified_diff(&display, old.as_deref().unwrap_or(""), &new),
        old_text: old,
        new_text: Some(new),
    })
}

pub(super) fn mutate(ctx: &ToolContext, args: &Value, edit: bool) -> Result<String, ToolError> {
    let target = path(
        ctx,
        super::string(args, "file_path")?,
        true,
        if edit { "edit" } else { "write" },
    )?;
    let mut store = observations()
        .lock()
        .map_err(|_| ToolError::new("Observation lock failed"))?;
    let bytes = match fs::read(&target) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };
    ensure_observed(&store, ctx, &target, bytes.as_deref())?;
    let old = bytes
        .as_deref()
        .map(|b| String::from_utf8_lossy(b).into_owned());
    let new = changed(args, old.as_deref(), edit)?;
    ctx.ensure_not_cancelled()?;
    atomic_write(&target, &new)?;
    store.insert(
        (ctx.session_id.clone(), target.clone()),
        Some(version(new.as_bytes())),
    );
    if edit {
        Ok(if args["replace_all"].as_bool().unwrap_or(false) {
            format!(
                "The file {} has been updated. All occurrences were successfully replaced.",
                target.display()
            )
        } else {
            format!(
                "The file {} has been updated successfully.",
                target.display()
            )
        })
    } else {
        Ok(format!(
            "<path>{}</path>\n<type>file</type>\n<content>\n{} file\n</content>",
            target.display(),
            if bytes.is_some() {
                "Updated"
            } else {
                "Created"
            }
        ))
    }
}
