//! Workspace knowledge index for coding-agent retrieval.
//!
//! Indexes full file content in overlapping chunks, plus extracted symbols,
//! file paths, and decision documents (AGENTS.md / ADR). Persisted as JSONL
//! under `<workspace>/.anya/index/index.jsonl` with per-file metadata in
//! `index.meta.json` so [`WorkspaceIndex::refresh`] only re-reads changed files.
//!
//! This is the keyword retrieval layer. Semantic re-ranking is a separate,
//! optional layer — see [`crate::core::ai::embed::Embedder`].

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use regex::Regex;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".anya",
    ".cursor",
    "vendor",
];

/// Maximum source file size to index (bytes).
const MAX_FILE_BYTES: usize = 512 * 1024;
/// Chunk size and overlap for full-content indexing (chars).
const CHUNK_CHARS: usize = 1200;
const CHUNK_OVERLAP: usize = 200;
/// Cap per-file chunk records to bound index size on pathological files.
const MAX_CHUNKS_PER_FILE: usize = 400;

const SYMBOL_RE: &str = r"(?m)^\s*(?:pub\s+)?(?:async\s+)?(?:fn|struct|enum|trait|class|function|def|interface|type)\s+([A-Za-z_][A-Za-z0-9_]*)";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexHit {
    pub kind: String,
    pub path: String,
    pub symbol: Option<String>,
    pub snippet: String,
    pub score: i32,
}

/// Per-file fingerprint used for incremental refresh.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileMeta {
    path: String,
    modified: u64,
    len: u64,
}

pub struct WorkspaceIndex {
    root: PathBuf,
    db_path: PathBuf,
}

impl WorkspaceIndex {
    pub fn open(workspace: &Path) -> Result<Self, String> {
        let dir = workspace.join(".anya").join("index");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let db_path = dir.join("index.jsonl");
        Ok(Self {
            root: workspace.to_path_buf(),
            db_path,
        })
    }

    fn meta_path(&self) -> PathBuf {
        self.db_path.with_extension("meta.json")
    }

    /// Full rebuild: walk every indexable file, re-read content, rewrite the index.
    pub fn rebuild(&self) -> Result<usize, String> {
        let files = self.walk_files()?;
        let symbol_re = Regex::new(SYMBOL_RE).map_err(|e| e.to_string())?;
        let mut records = Vec::new();
        let mut metas = HashMap::new();
        for (rel, modified, len) in &files {
            metas.insert(
                rel.clone(),
                FileMeta {
                    path: rel.clone(),
                    modified: *modified,
                    len: *len,
                },
            );
            let abs = self.root.join(rel);
            records.extend(self.index_file(rel, &abs, &symbol_re)?);
        }
        let count = records.len();
        self.write(&records, &metas)?;
        Ok(count)
    }

    /// Incremental refresh: re-index changed/new files, drop removed files,
    /// and leave unchanged content untouched. Returns the file count.
    pub fn refresh(&self) -> Result<usize, String> {
        if !self.db_path.exists() {
            return self.rebuild();
        }

        let existing = self.load_metas()?;
        let files = self.walk_files()?;
        let current: HashMap<String, FileMeta> = files
            .iter()
            .map(|(rel, modified, len)| {
                (
                    rel.clone(),
                    FileMeta {
                        path: rel.clone(),
                        modified: *modified,
                        len: *len,
                    },
                )
            })
            .collect();

        let changed: Vec<String> = files
            .iter()
            .filter(|(rel, modified, len)| match existing.get(rel) {
                Some(meta) => meta.modified != *modified || meta.len != *len,
                None => true,
            })
            .map(|(rel, _, _)| rel.clone())
            .collect();
        let removed: Vec<String> = existing
            .keys()
            .filter(|rel| !current.contains_key(rel.as_str()))
            .cloned()
            .collect();

        if changed.is_empty() && removed.is_empty() {
            return Ok(current.len());
        }

        let changed_set: HashSet<&str> = changed.iter().map(String::as_str).collect();
        let removed_set: HashSet<&str> = removed.iter().map(String::as_str).collect();

        // Keep existing data records for unchanged paths.
        let mut kept: Vec<serde_json::Value> = Vec::new();
        for line in self.read_lines()? {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            let kind = value.get("kind").and_then(|v| v.as_str()).unwrap_or("");
            if kind == "meta" || kind == "file_meta" {
                continue;
            }
            let path = value.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if removed_set.contains(path) || changed_set.contains(path) {
                continue;
            }
            kept.push(value);
        }

        let symbol_re = Regex::new(SYMBOL_RE).map_err(|e| e.to_string())?;
        for rel in &changed {
            let abs = self.root.join(rel);
            kept.extend(self.index_file(rel, &abs, &symbol_re)?);
        }

        let count = kept.len();
        self.write(&kept, &current)?;
        Ok(count)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<IndexHit>, String> {
        self.refresh()?;
        let raw = fs::read_to_string(&self.db_path).map_err(|e| e.to_string())?;
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let terms: Vec<String> = q.split_whitespace().map(str::to_string).collect();
        let mut hits = Vec::new();
        for line in raw.lines().skip(1) {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let kind = value
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("file")
                .to_string();
            if kind == "meta" || kind == "file_meta" {
                continue;
            }
            let path = value
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let symbol = value
                .get("symbol")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let snippet = value
                .get("snippet")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let score = score_record(&path, symbol.as_deref(), &snippet, &kind, &terms);
            if score > 0 {
                hits.push(IndexHit {
                    kind,
                    path,
                    symbol,
                    snippet,
                    score,
                });
            }
        }
        hits.sort_by(|a, b| b.score.cmp(&a.score).then(a.path.cmp(&b.path)));
        hits.truncate(limit.max(1));
        Ok(hits)
    }

    fn walk_files(&self) -> Result<Vec<(String, u64, u64)>, String> {
        let mut files = Vec::new();
        for entry in WalkDir::new(&self.root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !(e.file_type().is_dir() && SKIP_DIRS.iter().any(|s| *s == name))
            })
        {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let rel = path
                .strip_prefix(&self.root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            if !is_indexable(&rel) {
                continue;
            }
            let meta = entry.metadata().map_err(|e| e.to_string())?;
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            files.push((rel, modified, meta.len()));
        }
        Ok(files)
    }

    fn index_file(
        &self,
        rel: &str,
        abs: &Path,
        symbol_re: &Regex,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut records = Vec::new();
        let Ok(content) = fs::read_to_string(abs) else {
            return Ok(records);
        };
        if content.len() > MAX_FILE_BYTES {
            return Ok(records);
        }

        records.push(serde_json::json!({
            "kind": "file",
            "path": rel,
            "symbol": serde_json::Value::Null,
            "snippet": first_line_snippet(&content),
        }));

        for cap in symbol_re.captures_iter(&content).take(80) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            if name.is_empty() {
                continue;
            }
            records.push(serde_json::json!({
                "kind": "symbol",
                "path": rel,
                "symbol": name,
                "snippet": cap.get(0).map(|m| m.as_str()).unwrap_or("").trim(),
            }));
        }

        if is_decision_doc(rel) {
            records.push(serde_json::json!({
                "kind": "decision",
                "path": rel,
                "symbol": serde_json::Value::Null,
                "snippet": first_line_snippet(&content),
            }));
        }

        for chunk in chunk_content(&content) {
            records.push(serde_json::json!({
                "kind": "chunk",
                "path": rel,
                "symbol": serde_json::Value::Null,
                "snippet": chunk,
            }));
        }

        Ok(records)
    }

    fn read_lines(&self) -> Result<Vec<String>, String> {
        let raw = fs::read_to_string(&self.db_path).map_err(|e| e.to_string())?;
        Ok(raw.lines().map(str::to_string).collect())
    }

    fn load_metas(&self) -> Result<HashMap<String, FileMeta>, String> {
        if !self.meta_path().exists() {
            return Ok(HashMap::new());
        }
        let raw = fs::read_to_string(self.meta_path()).map_err(|e| e.to_string())?;
        serde_json::from_str(&raw).map_err(|e| e.to_string())
    }

    fn write(
        &self,
        records: &[serde_json::Value],
        metas: &HashMap<String, FileMeta>,
    ) -> Result<(), String> {
        let mut out = String::new();
        out.push_str(&format!(
            "{}\n",
            serde_json::json!({
                "kind": "meta",
                "builtAtMs": now_ms(),
                "count": records.len(),
            })
        ));
        for record in records {
            out.push_str(&format!("{record}\n"));
        }
        fs::write(&self.db_path, out).map_err(|e| e.to_string())?;
        fs::write(
            self.meta_path(),
            serde_json::to_string(metas).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn score_record(
    path: &str,
    symbol: Option<&str>,
    snippet: &str,
    kind: &str,
    terms: &[String],
) -> i32 {
    let path_l = path.to_ascii_lowercase();
    let sym_l = symbol.unwrap_or("").to_ascii_lowercase();
    let snip_l = snippet.to_ascii_lowercase();
    let mut score = 0;
    for term in terms {
        if sym_l == *term {
            score += 40;
        } else if sym_l.contains(term) {
            score += 25;
        }
        if path_l.contains(term) {
            score += 15;
        }
        let occurrences = snip_l.matches(term.as_str()).count().min(8);
        score += (occurrences as i32) * 10;
    }
    if score > 0 && kind == "decision" {
        score += 20;
    }
    score
}

fn chunk_content(content: &str) -> Vec<String> {
    let char_count = content.chars().count();
    if char_count <= CHUNK_CHARS {
        return vec![content.to_string()];
    }
    let chars: Vec<char> = content.chars().collect();
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < chars.len() && chunks.len() < MAX_CHUNKS_PER_FILE {
        let end = (start + CHUNK_CHARS).min(chars.len());
        chunks.push(chars[start..end].iter().collect());
        if end >= chars.len() {
            break;
        }
        start = end - CHUNK_OVERLAP;
    }
    chunks
}

fn is_indexable(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    lower.ends_with(".rs")
        || lower.ends_with(".ts")
        || lower.ends_with(".tsx")
        || lower.ends_with(".js")
        || lower.ends_with(".jsx")
        || lower.ends_with(".py")
        || lower.ends_with(".go")
        || lower.ends_with(".java")
        || lower.ends_with(".vue")
        || lower.ends_with(".css")
        || lower.ends_with(".html")
        || lower.ends_with(".md")
        || lower.ends_with(".toml")
        || lower.ends_with(".json")
}

fn is_decision_doc(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    lower.ends_with("agents.md")
        || lower.contains("/adr/")
        || lower.contains("architecture")
        || lower.ends_with("decisions.md")
}

fn first_line_snippet(content: &str) -> String {
    content
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .chars()
        .take(160)
        .collect()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_and_finds_symbols_and_content() {
        let root = std::env::temp_dir().join(format!("anya-index-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn hello_world() {}\nfn other() {}\n",
        )
        .unwrap();
        fs::write(
            root.join("src/notes.md"),
            "# Notes\n\nThe retry policy is exponential backoff.\n",
        )
        .unwrap();
        fs::write(root.join("AGENTS.md"), "# Agents\nUse pnpm.\n").unwrap();

        let index = WorkspaceIndex::open(&root).unwrap();
        let count = index.rebuild().unwrap();
        assert!(count >= 2);

        let symbol_hits = index.search("hello_world", 5).unwrap();
        assert!(symbol_hits
            .iter()
            .any(|h| h.symbol.as_deref() == Some("hello_world")));

        let docs = index.search("agents", 5).unwrap();
        assert!(docs.iter().any(|h| h.kind == "decision"));

        // Full-content chunk: phrase lives only in the middle of notes.md.
        let retry = index.search("retry policy", 5).unwrap();
        assert!(retry
            .iter()
            .any(|h| h.path == "src/notes.md" && h.kind == "chunk"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refresh_picks_up_changes_without_full_rebuild() {
        let root = std::env::temp_dir().join(format!("anya-refresh-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("a.txt"), "apple\n").unwrap();
        fs::write(root.join("a.py"), "apple\n").unwrap();

        let index = WorkspaceIndex::open(&root).unwrap();
        index.rebuild().unwrap();
        assert!(index
            .search("apple", 3)
            .unwrap()
            .iter()
            .any(|h| h.path == "a.py"));

        // New file appears after refresh.
        fs::write(root.join("b.py"), "banana split\n").unwrap();
        index.refresh().unwrap();
        assert!(index
            .search("banana", 3)
            .unwrap()
            .iter()
            .any(|h| h.path == "b.py"));

        // Removed file disappears after refresh.
        fs::remove_file(root.join("a.py")).unwrap();
        index.refresh().unwrap();
        assert!(index
            .search("apple", 3)
            .unwrap()
            .iter()
            .all(|h| h.path != "a.py"));

        let _ = fs::remove_dir_all(root);
    }
}
