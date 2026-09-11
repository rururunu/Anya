//! Bundle plugin workbench UI into one browser ESM file on enable.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::locate::{deno_command_candidates, first_existing};
use super::manifest::{load_manifest, plugin_dir};
use crate::core::tools::error::ToolError;

const BUNDLE_TIMEOUT: Duration = Duration::from_secs(90);
const BUNDLE_SCRIPT: &str = include_str!("bundle_ui.ts");

/// Output path written on enable (`ui/.anya/activate.js`).
pub fn bundled_ui_path(plugin_dir: &Path) -> PathBuf {
    plugin_dir.join("ui").join(".anya").join("activate.js")
}

/// Prefer `ui/src/activate.{ts,js}`, then the declared `ui.entry`.
pub fn find_ui_source(plugin_dir: &Path, declared_entry: &str) -> PathBuf {
    const CANDIDATES: &[&str] = &[
        "ui/src/activate.ts",
        "ui/src/activate.js",
        "ui/src/main.ts",
        "ui/src/main.js",
    ];
    for rel in CANDIDATES {
        let path = plugin_dir.join(rel);
        if path.is_file() {
            return path;
        }
    }
    plugin_dir.join(declared_entry.trim().replace('\\', "/"))
}

/// Path actually served to the workbench (bundled file when present).
pub fn resolve_ui_file(plugin_dir: &Path, declared_entry: &str) -> PathBuf {
    let bundled = bundled_ui_path(plugin_dir);
    if bundled.is_file() {
        bundled
    } else {
        plugin_dir.join(declared_entry.trim().replace('\\', "/"))
    }
}

pub fn read_plugin_ui_source(id: &str) -> Result<String, ToolError> {
    let manifest = load_manifest(id)?;
    let dir = plugin_dir(id)?;
    let path = resolve_ui_file(&dir, &manifest.ui.entry);
    fs::read_to_string(&path).map_err(|e| ToolError::new(format!("read {}: {e}", path.display())))
}

/// `rebuild`: enable/reload always rebundle; restore only if the output is missing.
pub fn bundle_plugin_ui(id: &str, rebuild: bool) -> Result<PathBuf, ToolError> {
    let manifest = load_manifest(id)?;
    let dir = plugin_dir(id)?;
    let out = bundled_ui_path(&dir);
    if !super::manifest::needs_workbench_ui(&manifest) {
        return Ok(out);
    }
    if !rebuild && out.is_file() {
        return Ok(out);
    }
    let source = find_ui_source(&dir, &manifest.ui.entry);
    if !source.is_file() {
        return Err(ToolError::new(format!(
            "plugin UI entry missing: {}",
            source.display()
        )));
    }
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|e| ToolError::new(e.to_string()))?;
    }
    match run_esbuild(&dir, &source, &out) {
        Ok(()) => Ok(out),
        Err(err) => {
            let text = fs::read_to_string(&source).unwrap_or_default();
            if has_bare_specifier(&text) {
                Err(ToolError::new(format!(
                    "UI bundle failed (npm imports need Deno + network on first enable): {err}"
                )))
            } else {
                tracing::warn!(plugin = %id, %err, "esbuild unavailable; copying UI source");
                fs::copy(&source, &out).map_err(|e| ToolError::new(e.to_string()))?;
                Ok(out)
            }
        }
    }
}

fn run_esbuild(plugin_dir: &Path, source: &Path, out: &Path) -> Result<(), ToolError> {
    let deno = first_existing(&deno_command_candidates())
        .ok_or_else(|| ToolError::new("Deno sidecar not found; run `pnpm fetch-deno`"))?;
    let cache = super::locate::deno_cache_dir();
    fs::create_dir_all(&cache).map_err(|e| ToolError::new(e.to_string()))?;
    let script = cache.join("bundle_ui.ts");
    fs::write(&script, BUNDLE_SCRIPT).map_err(|e| ToolError::new(e.to_string()))?;
    let mut cmd = Command::new(&deno);
    cmd.arg("run")
        .arg("--quiet")
        .arg("--no-prompt")
        .arg("--allow-read")
        .arg("--allow-write")
        .arg("--allow-net")
        .arg("--allow-env")
        .arg("--allow-run")
        .arg(&script)
        .arg(source)
        .arg(out)
        .current_dir(plugin_dir)
        .env("DENO_DIR", &cache)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| ToolError::new(format!("spawn ui bundler: {e}")))?;
    let stderr_pipe = child.stderr.take();
    let stderr_thread = std::thread::spawn(move || {
        let mut stderr = String::new();
        if let Some(mut pipe) = stderr_pipe {
            let _ = pipe.read_to_string(&mut stderr);
        }
        stderr
    });
    let status = wait_timeout(&mut child, BUNDLE_TIMEOUT)?;
    let stderr = stderr_thread.join().unwrap_or_default();
    if !status.success() {
        return Err(ToolError::new(format!(
            "esbuild exit {}: {}",
            status.code().unwrap_or(-1),
            stderr.trim()
        )));
    }
    if !out.is_file() {
        return Err(ToolError::new("bundler did not write activate.js"));
    }
    Ok(())
}

fn wait_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus, ToolError> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ToolError::new("UI bundle timed out"));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(err) => return Err(ToolError::new(format!("wait bundler: {err}"))),
        }
    }
}

pub fn has_bare_specifier(source: &str) -> bool {
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with("//") || line.starts_with('*') {
            continue;
        }
        let spec = specifier_after(line, " from ").or_else(|| specifier_after(line, "import "));
        if let Some(spec) = spec {
            if !spec.starts_with('.') && !spec.starts_with('/') && !spec.starts_with("http") {
                return true;
            }
        }
    }
    false
}

fn specifier_after<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    let idx = line.find(marker)?;
    let rest = line[idx + marker.len()..].trim_start();
    let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let body = rest.get(1..)?;
    let end = body.find(quote)?;
    Some(&body[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_npm_imports() {
        assert!(has_bare_specifier("import _ from \"lodash\";\n"));
        assert!(has_bare_specifier("export { z } from 'zod';"));
        assert!(!has_bare_specifier(
            "import { activate } from \"./lib.js\";"
        ));
        assert!(!has_bare_specifier("export async function activate() {}"));
    }

    #[test]
    fn prefers_src_entry() {
        let dir = std::env::temp_dir().join(format!("anya-bundle-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("ui/src")).unwrap();
        fs::write(
            dir.join("ui/src/activate.js"),
            "export function activate() {}",
        )
        .unwrap();
        fs::create_dir_all(dir.join("ui")).unwrap();
        fs::write(dir.join("ui/activate.js"), "old").unwrap();
        let found = find_ui_source(&dir, "ui/activate.js");
        assert_eq!(found, dir.join("ui/src/activate.js"));
        let _ = fs::remove_dir_all(&dir);
    }
}
