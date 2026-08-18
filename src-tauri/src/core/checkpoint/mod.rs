use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::core::tools::error::ToolError;
use crate::core::tools::preview::ToolPreview;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSnap {
    pub path: String,
    /// None means the file did not exist (restore deletes it).
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Checkpoint {
    pub turn: usize,
    pub time: u64,
    pub prompt: String,
    pub files: Vec<FileSnap>,
    #[serde(default)]
    pub user_message_id: Option<String>,
    #[serde(default)]
    pub workspace_root: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CheckpointIndex {
    checkpoints: Vec<Checkpoint>,
}

struct ActiveTurn {
    turn: usize,
    prompt: String,
    user_message_id: Option<String>,
    workspace_root: Option<String>,
    snapped: HashMap<String, FileSnap>,
}

pub struct CheckpointStore {
    root: PathBuf,
    active: Mutex<HashMap<String, ActiveTurn>>,
}

impl CheckpointStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            active: Mutex::new(HashMap::new()),
        }
    }

    fn session_dir(&self, session_id: &str) -> PathBuf {
        self.root.join(session_id)
    }

    fn index_path(&self, session_id: &str) -> PathBuf {
        self.session_dir(session_id).join("index.json")
    }

    fn checkpoint_from_turn(turn: &ActiveTurn) -> Checkpoint {
        Checkpoint {
            turn: turn.turn,
            time: now_secs(),
            prompt: turn.prompt.clone(),
            files: turn.snapped.values().cloned().collect(),
            user_message_id: turn.user_message_id.clone(),
            workspace_root: turn.workspace_root.clone(),
        }
    }

    /// Persist (or replace) one turn's checkpoint on disk immediately.
    fn write_checkpoint(&self, session_id: &str, checkpoint: &Checkpoint) -> Result<(), ToolError> {
        if checkpoint.files.is_empty() && checkpoint.user_message_id.is_none() {
            return Ok(());
        }
        let dir = self.session_dir(session_id);
        fs::create_dir_all(&dir)?;
        let mut index = self.load_index(session_id)?;
        index.checkpoints.retain(|c| c.turn != checkpoint.turn);
        index.checkpoints.push(checkpoint.clone());
        index.checkpoints.sort_by_key(|c| c.turn);
        fs::write(
            self.index_path(session_id),
            serde_json::to_string_pretty(&index)?,
        )?;
        Ok(())
    }

    fn persist_active_turn(&self, session_id: &str, turn: &ActiveTurn) -> Result<(), ToolError> {
        self.write_checkpoint(session_id, &Self::checkpoint_from_turn(turn))
    }

    pub fn begin_turn(
        &self,
        session_id: &str,
        turn: usize,
        prompt: &str,
        user_message_id: Option<String>,
        workspace_root: Option<&Path>,
    ) {
        let active_turn = ActiveTurn {
            turn,
            prompt: prompt.to_string(),
            user_message_id,
            workspace_root: workspace_root.map(|path| path.to_string_lossy().into_owned()),
            snapped: HashMap::new(),
        };
        // Persist before tools run so conversation rewind survives crashes mid-turn.
        let _ = self.persist_active_turn(session_id, &active_turn);
        if let Ok(mut active) = self.active.lock() {
            active.insert(session_id.to_string(), active_turn);
        }
    }

    /// Ensure a conversation-only checkpoint exists (crash recovery / backfill).
    pub fn ensure_conversation_checkpoint(
        &self,
        session_id: &str,
        turn: usize,
        prompt: &str,
        user_message_id: &str,
        workspace_root: Option<&Path>,
    ) -> Result<(), ToolError> {
        let mut index = self.load_index(session_id)?;
        if index
            .checkpoints
            .iter()
            .any(|c| c.user_message_id.as_deref() == Some(user_message_id))
        {
            return Ok(());
        }
        if let Some(existing) = index.checkpoints.iter_mut().find(|c| c.turn == turn) {
            if existing.user_message_id.is_none() {
                existing.user_message_id = Some(user_message_id.to_string());
                if existing.prompt.trim().is_empty() {
                    existing.prompt = prompt.to_string();
                }
                let dir = self.session_dir(session_id);
                fs::create_dir_all(&dir)?;
                fs::write(
                    self.index_path(session_id),
                    serde_json::to_string_pretty(&index)?,
                )?;
            }
            return Ok(());
        }
        self.write_checkpoint(
            session_id,
            &Checkpoint {
                turn,
                time: now_secs(),
                prompt: prompt.to_string(),
                files: Vec::new(),
                user_message_id: Some(user_message_id.to_string()),
                workspace_root: workspace_root.map(|path| path.to_string_lossy().into_owned()),
            },
        )
    }

    pub fn snapshot_preview(
        &self,
        session_id: &str,
        workspace_root: &Path,
        preview: &ToolPreview,
    ) -> Result<(), ToolError> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| ToolError::new("checkpoint lock poisoned"))?;
        let Some(turn) = active.get_mut(session_id) else {
            return Ok(());
        };
        let paths = if preview.affected_paths.is_empty() {
            std::slice::from_ref(&preview.path)
        } else {
            preview.affected_paths.as_slice()
        };
        for path in paths {
            if turn.snapped.contains_key(path) {
                continue;
            }
            let abs = workspace_root.join(path);
            let content = if abs.exists() {
                Some(fs::read_to_string(&abs)?)
            } else {
                None
            };
            turn.snapped.insert(
                path.clone(),
                FileSnap {
                    path: path.clone(),
                    content,
                },
            );
        }
        // Keep disk in sync so file restore still works after a mid-turn crash.
        let _ = self.persist_active_turn(session_id, turn);
        Ok(())
    }

    pub fn finish_turn(&self, session_id: &str) -> Result<(), ToolError> {
        let finished = {
            let mut active = self
                .active
                .lock()
                .map_err(|_| ToolError::new("checkpoint lock poisoned"))?;
            active.remove(session_id)
        };
        let Some(turn) = finished else {
            return Ok(());
        };
        // Always persist a checkpoint when we have a user message id so conversation
        // rewind stays available even for turns that did not mutate files.
        self.persist_active_turn(session_id, &turn)
    }

    pub fn list(&self, session_id: &str) -> Result<Vec<Checkpoint>, ToolError> {
        Ok(self.load_index(session_id)?.checkpoints)
    }

    pub fn restore_code(
        &self,
        session_id: &str,
        turn: usize,
        workspace_root: &Path,
    ) -> Result<usize, ToolError> {
        let index = self.load_index(session_id)?;
        let Some(checkpoint) = index.checkpoints.iter().find(|c| c.turn == turn) else {
            return Err(ToolError::new(format!("checkpoint turn {turn} not found")));
        };
        let mut restored = 0usize;
        for snap in &checkpoint.files {
            let abs = workspace_root.join(&snap.path);
            match &snap.content {
                None => {
                    if abs.exists() {
                        fs::remove_file(&abs)?;
                        restored += 1;
                    }
                }
                Some(content) => {
                    if let Some(parent) = abs.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&abs, content)?;
                    restored += 1;
                }
            }
        }
        Ok(restored)
    }

    /// Drop the checkpoint for `turn` and all later turns (after conversation rewind).
    pub fn drop_from_turn(&self, session_id: &str, turn: usize) -> Result<(), ToolError> {
        let mut index = self.load_index(session_id)?;
        index.checkpoints.retain(|c| c.turn < turn);
        let dir = self.session_dir(session_id);
        fs::create_dir_all(&dir)?;
        fs::write(
            self.index_path(session_id),
            serde_json::to_string_pretty(&index)?,
        )?;
        Ok(())
    }

    fn load_index(&self, session_id: &str) -> Result<CheckpointIndex, ToolError> {
        let path = self.index_path(session_id);
        if !path.exists() {
            return Ok(CheckpointIndex::default());
        }
        let raw = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw).unwrap_or_default())
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn shared_checkpoint_store() -> &'static CheckpointStore {
    static STORE: OnceLock<CheckpointStore> = OnceLock::new();
    STORE.get_or_init(|| {
        let root = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(if cfg!(debug_assertions) {
                "peek-debug"
            } else {
                "peek"
            })
            .join("checkpoints");
        let _ = fs::create_dir_all(&root);
        CheckpointStore::new(root)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tools::preview::ChangeKind;

    #[test]
    fn begin_turn_persists_conversation_checkpoint_immediately() {
        let base =
            std::env::temp_dir().join(format!("peek-checkpoint-begin-{}", uuid::Uuid::new_v4()));
        let store = CheckpointStore::new(base.join("checkpoints"));
        store.begin_turn("session", 1, "hello", Some("user-1".into()), None);
        let listed = store.list("session").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].user_message_id.as_deref(), Some("user-1"));
        assert!(listed[0].files.is_empty());
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn snapshots_and_restores_every_affected_path() {
        let base = std::env::temp_dir().join(format!("peek-checkpoint-{}", uuid::Uuid::new_v4()));
        let workspace = base.join("workspace");
        let store = CheckpointStore::new(base.join("checkpoints"));
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("one.txt"), "old one\n").unwrap();
        fs::write(workspace.join("two.txt"), "old two\n").unwrap();

        store.begin_turn(
            "session",
            1,
            "edit both",
            Some("user-1".into()),
            Some(&workspace),
        );
        store
            .snapshot_preview(
                "session",
                &workspace,
                &ToolPreview {
                    path: "one.txt".into(),
                    affected_paths: vec!["one.txt".into(), "two.txt".into(), "new.txt".into()],
                    kind: ChangeKind::Modify,
                    old_text: None,
                    new_text: None,
                    unified_diff: String::new(),
                },
            )
            .unwrap();
        store.finish_turn("session").unwrap();

        fs::write(workspace.join("one.txt"), "new one\n").unwrap();
        fs::write(workspace.join("two.txt"), "new two\n").unwrap();
        fs::write(workspace.join("new.txt"), "created\n").unwrap();

        assert_eq!(store.restore_code("session", 1, &workspace).unwrap(), 3);
        assert_eq!(
            fs::read_to_string(workspace.join("one.txt")).unwrap(),
            "old one\n"
        );
        assert_eq!(
            fs::read_to_string(workspace.join("two.txt")).unwrap(),
            "old two\n"
        );
        assert!(!workspace.join("new.txt").exists());

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn legacy_preview_path_remains_supported() {
        let preview: ToolPreview = serde_json::from_value(serde_json::json!({
            "path": "legacy.txt",
            "kind": "modify",
            "oldText": "old",
            "newText": "new",
            "unifiedDiff": ""
        }))
        .unwrap();
        assert!(preview.affected_paths.is_empty());
    }
}
