//! Shared directory skip list for workspace walks (find/list/search/index).
//!
//! Prunes generated and dependency trees *before* descending. Matching the last
//! path component after a full walk (as `glob()` did) still visits `target/`.

use walkdir::DirEntry;

pub const SKIP_DIR_NAMES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    "__pycache__",
    ".venv",
    "venv",
    ".anya",
    ".cursor",
    "vendor",
];

/// True when a directory with this name should not be descended into.
pub fn should_skip_dir_name(name: &str) -> bool {
    SKIP_DIR_NAMES.iter().any(|skip| *skip == name)
}

/// WalkDir `filter_entry` helper: keep the walk root, skip named dirs below it.
pub fn should_skip_walk_entry(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    if !entry.file_type().is_dir() {
        return false;
    }
    // Skip list is ASCII-only; non-UTF8 names are never in it.
    entry
        .file_name()
        .to_str()
        .is_some_and(should_skip_dir_name)
}

/// ripgrep `--glob` excludes so a search still skips build trees without gitignore.
pub fn rg_exclude_globs() -> impl Iterator<Item = String> {
    SKIP_DIR_NAMES.iter().map(|name| format!("!**/{name}/**"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_list_covers_build_and_deps() {
        for name in ["target", "node_modules", ".git", "dist", "build", ".anya"] {
            assert!(should_skip_dir_name(name), "{name}");
        }
        assert!(!should_skip_dir_name("src"));
        assert!(!should_skip_dir_name("lib.rs"));
    }
}
