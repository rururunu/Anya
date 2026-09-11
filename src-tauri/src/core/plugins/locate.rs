//! Locate bundled Deno / plugin-host binaries the same way as ripgrep.

use std::path::PathBuf;

use crate::core::tools::memory::plugins_dir;

/// Writable Deno/npm cache under the user plugins directory (not the install dir).
pub fn deno_cache_dir() -> PathBuf {
    plugins_dir().join(".cache").join("deno")
}

pub fn deno_command_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(bundled) = bundled_named("deno") {
        out.push(bundled);
    }
    out.push(PathBuf::from(if cfg!(windows) {
        "deno.exe"
    } else {
        "deno"
    }));
    out
}

pub fn plugin_host_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(bundled) = bundled_named("anya-plugin-host") {
        out.push(bundled);
    }
    if let Ok(exe) = std::env::current_exe() {
        let suffix = std::env::consts::EXE_SUFFIX;
        if let Some(dir) = exe.parent() {
            out.push(dir.join(format!("anya-plugin-host{suffix}")));
            if dir.file_name().is_some_and(|name| name == "deps") {
                if let Some(parent) = dir.parent() {
                    out.push(parent.join(format!("anya-plugin-host{suffix}")));
                }
            }
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let suffix = std::env::consts::EXE_SUFFIX;
        let target = PathBuf::from(manifest).join("..").join("target");
        out.push(
            target
                .join("debug")
                .join(format!("anya-plugin-host{suffix}")),
        );
        out.push(
            target
                .join("release")
                .join(format!("anya-plugin-host{suffix}")),
        );
    }
    out
}

pub fn first_existing(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|p| p.is_file()).cloned()
}

fn bundled_named(stem: &str) -> Option<PathBuf> {
    for dir in candidate_dirs() {
        for name in sidecar_file_names(stem) {
            let path = dir.join(name);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

fn candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.to_path_buf());
            if dir.file_name().is_some_and(|name| name == "deps") {
                if let Some(parent) = dir.parent() {
                    dirs.push(parent.to_path_buf());
                }
            }
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        dirs.push(PathBuf::from(manifest).join("binaries"));
    }
    dirs
}

fn sidecar_file_names(stem: &str) -> Vec<String> {
    let suffix = std::env::consts::EXE_SUFFIX;
    let mut names = vec![format!("{stem}{suffix}")];
    if let Some(triple) = option_env!("TARGET_TRIPLE") {
        names.insert(0, format!("{stem}-{triple}{suffix}"));
    }
    names
}
