//! Materialize a bundled, offline document runtime; no Office process or COM.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tauri::Manager;

use crate::core::tools::context::ToolContext;
use crate::core::tools::error::ToolError;

const API: &str = include_str!("../../../../prompts/skills/office-api.md");

fn runtime_source(ctx: &ToolContext) -> Result<PathBuf, ToolError> {
    if let Some(app) = &ctx.app_handle {
        if let Ok(resources) = app.path().resource_dir() {
            let installed = resources.join("office");
            if installed.join("runtime.mjs").is_file() {
                return Ok(installed);
            }
        }
    }
    let development = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/office");
    if development.join("runtime.mjs").is_file() {
        return Ok(development);
    }
    Err(ToolError::new("Office runtime resources are missing. Build with `pnpm build:office-runtime`, or reinstall the packaged app."))
}

fn materialize(source: &Path, workspace: &Path) -> Result<PathBuf, ToolError> {
    static MATERIALIZE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = MATERIALIZE_LOCK
        .lock()
        .map_err(|_| ToolError::new("Office runtime materialization lock failed"))?;
    let root = fs::canonicalize(workspace)?;
    let dest = root.join(".anya/office-runtime/v1");
    // Never follow a user-created junction/symlink outside the workspace.
    for parent in [
        root.join(".anya"),
        root.join(".anya/office-runtime"),
        dest.clone(),
    ] {
        fs::create_dir_all(&parent)?;
        if !fs::canonicalize(&parent)?.starts_with(&root) {
            return Err(ToolError::new(
                "Office runtime directory resolves outside the workspace",
            ));
        }
    }
    for filename in ["runtime.mjs", "cli.mjs", "THIRD_PARTY_LICENSES.txt"] {
        let target = dest.join(filename);
        let bytes = fs::read(source.join(filename))?;
        if fs::read(&target).is_ok_and(|existing| existing == bytes) {
            continue;
        }
        if fs::symlink_metadata(&target).is_ok() {
            fs::remove_file(&target)?;
        }
        fs::write(target, bytes)?;
    }
    if fs::symlink_metadata(dest.join("API.md")).is_ok() {
        fs::remove_file(dest.join("API.md"))?;
    }
    fs::write(dest.join("API.md"), API)?;
    // Engine binaries stay in installed resources, not in a writable workspace.
    let config = dest.join("kit-path.json");
    if fs::symlink_metadata(&config).is_ok() {
        fs::remove_file(&config)?;
    }
    fs::write(
        config,
        serde_json::to_vec(&serde_json::json!({
            "resourceDirectory": fs::canonicalize(source)?.to_string_lossy(),
        }))
        .map_err(|error| ToolError::new(error.to_string()))?,
    )?;
    Ok(dest)
}

pub fn prepare(ctx: &ToolContext) -> Result<String, ToolError> {
    let deno = crate::core::plugins::deno_path().ok_or_else(|| ToolError::new("Bundled Deno runtime is missing. Reinstall Anya or run `pnpm fetch-deno` in development."))?;
    let dir = materialize(&runtime_source(ctx)?, &ctx.workspace_root)?;
    let module_url = reqwest::Url::from_file_path(dir.join("runtime.mjs"))
        .map_err(|_| ToolError::new("Cannot construct document runtime module URL"))?;
    Ok(format!(
        "## Document runtime\nDeno executable: `{}`\nRuntime import URL: `{module_url}`\nCLI: `{}/cli.mjs`\nAPI reference: `{}/API.md`\nBundled Office Node: `{}`\n\nUse these exact paths; dependencies and LibreOffice Kit are bundled, so no npm install or network is required.\nWrite task-specific JavaScript in the workspace and execute it with run_shell using the bundled Deno (`run --no-prompt --allow-read --allow-write --allow-env <script>`). Rendering, conversion and recalculation additionally need `--allow-run` for the exact bundled Office Node path and `taskkill.exe`. System LibreOffice and Poppler are not required. Use PowerShell's `&` operator before a quoted executable. Read API.md before authoring.\nNo active Office window or unsaved selection is read; operate on saved files. Do not use COM/ActiveX/PowerShell Office automation.\n",
        deno.display(), dir.display(), dir.display(), runtime_source(ctx)?.join(if cfg!(windows) { "node.exe" } else { "node" }).display()
    ))
}

pub fn is_office_document(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|v| v.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("docx" | "dotx" | "xlsx" | "xlsm" | "pptx")
    )
}

pub fn read_file(
    ctx: &ToolContext,
    path: &Path,
    options: &serde_json::Value,
) -> Result<String, ToolError> {
    ctx.ensure_not_cancelled()?;
    let deno = crate::core::plugins::deno_path()
        .ok_or_else(|| ToolError::new("Bundled Deno runtime is missing"))?;
    let dir = materialize(&runtime_source(ctx)?, &ctx.workspace_root)?;
    let mut command = Command::new(deno);
    command
        .args([
            "run",
            "--cached-only",
            "--no-prompt",
            "--allow-read",
            "--allow-env",
        ])
        .arg(dir.join("cli.mjs"))
        .arg("read")
        .arg(path)
        .arg(options.to_string())
        .current_dir(&ctx.workspace_root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::runtime::terminal::prepare_command(&mut command);
    let mut child = command
        .spawn()
        .map_err(|error| ToolError::new(format!("Office reader failed to start: {error}")))?;
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = (&mut stdout).take(4 * 1024 * 1024).read_to_end(&mut bytes);
        bytes
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = (&mut stderr).take(64 * 1024).read_to_end(&mut bytes);
        bytes
    });
    let started = Instant::now();
    let result = loop {
        if ctx.is_cancelled() {
            break Err(ToolError::cancelled());
        }
        if started.elapsed() >= Duration::from_secs(120) {
            break Err(ToolError::new("Office reader timed out after 120 seconds"));
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(error) => break Err(ToolError::new(error.to_string())),
        }
    };
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let output = String::from_utf8_lossy(&out.join().unwrap_or_default()).into_owned();
    let errors = String::from_utf8_lossy(&err.join().unwrap_or_default()).into_owned();
    if !result?.success() {
        return Err(ToolError::new(format!("Office reader failed: {errors}")));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materializes_packaged_assets_without_source_tree_dependency() {
        let temp =
            std::env::temp_dir().join(format!("anya-office-runtime-{}", uuid::Uuid::new_v4()));
        let source = temp.join("installed-resources");
        let workspace = temp.join("workspace");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&workspace).unwrap();
        for name in ["runtime.mjs", "cli.mjs", "THIRD_PARTY_LICENSES.txt"] {
            fs::write(source.join(name), format!("packaged {name}")).unwrap();
        }
        let dest = materialize(&source, &workspace).unwrap();
        fs::write(dest.join("builder.mjs"), "user script").unwrap();
        materialize(&source, &workspace).unwrap();
        assert_eq!(
            fs::read_to_string(dest.join("runtime.mjs")).unwrap(),
            "packaged runtime.mjs"
        );
        assert_eq!(
            fs::read_to_string(dest.join("builder.mjs")).unwrap(),
            "user script"
        );
        assert!(dest.join("API.md").is_file());
        let config: serde_json::Value =
            serde_json::from_slice(&fs::read(dest.join("kit-path.json")).unwrap()).unwrap();
        assert_eq!(
            config["resourceDirectory"],
            fs::canonicalize(&source)
                .unwrap()
                .to_string_lossy()
                .as_ref()
        );
        fs::remove_dir_all(temp).unwrap();
    }
}
