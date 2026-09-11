//! Locate the bundled ripgrep sidecar, then a PATH `rg`.

use std::path::{Path, PathBuf};

/// Command paths to try, in order: bundled sidecar then PATH `rg`.
pub fn rg_command_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(bundled) = bundled_rg() {
        out.push(bundled);
    }
    out.push(PathBuf::from(if cfg!(windows) { "rg.exe" } else { "rg" }));
    out
}

fn bundled_rg() -> Option<PathBuf> {
    for dir in candidate_dirs() {
        for name in sidecar_file_names() {
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

fn sidecar_file_names() -> Vec<String> {
    let suffix = std::env::consts::EXE_SUFFIX;
    let mut names = vec![format!("rg{suffix}")];
    if let Some(triple) = option_env!("TARGET_TRIPLE") {
        names.insert(0, format!("rg-{triple}{suffix}"));
    }
    names
}

/// Relativize a search hit path against the search root, using `/`.
pub fn display_hit_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_hit_path_strips_root() {
        let root = std::env::temp_dir().join("anya-rg-root");
        let file = root.join("src").join("lib.rs");
        assert_eq!(display_hit_path(&root, &file), "src/lib.rs");
    }

    #[test]
    fn candidates_always_include_path_rg() {
        let names: Vec<_> = rg_command_candidates()
            .into_iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(
            names.iter().any(|n| n == "rg" || n == "rg.exe"),
            "{names:?}"
        );
    }
}
