//! Learned computer-use procedures (Agent S2 retrieve/write + EchoPath callable steps).

mod store;

use serde_json::Value;

use crate::core::tools::error::ToolError;

pub use store::{delete_for_ui, get_for_ui, list_for_ui, prompt_catalog, Playbook};

/// `playbook` tool: list | lookup | save | fail | success | delete.
pub fn dispatch(args: &Value) -> Result<String, ToolError> {
    store::dispatch(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn scores_prefer_intent_and_exe() {
        let pb = store::Playbook {
            id: "1".into(),
            intent: "export PDF from Word".into(),
            app: Some("Word".into()),
            exe: Some("winword.exe".into()),
            version_last_ok: None,
            steps: vec![store::PlaybookStep {
                kind: "key".into(),
                action: None,
                name: None,
                role: None,
                keys: Some("ctrl+s".into()),
                text: None,
                window: None,
                note: None,
                precondition: None,
            }],
            success_assert: None,
            confidence: 0.9,
            status: store::PlaybookStatus::Active,
            success_count: 2,
            fail_count: 0,
            created_at: 1,
            updated_at: 1,
        };
        let high = store::score(&pb, "export pdf", Some("Word"), Some("winword.exe"));
        let low = store::score(&pb, "paint brush", Some("Paint"), Some("mspaint"));
        assert!(high > low);
        assert!(high > 2.0);
    }

    #[test]
    fn parse_steps_accepts_mixed_shapes() {
        let steps = store::parse_steps(&json!([
            { "kind": "window", "action": "launch", "name": "notepad" },
            { "kind": "act", "action": "type", "role": "edit", "text": "hi" },
            { "op": "key", "keys": "ctrl+s" }
        ]))
        .unwrap();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[2].kind, "key");
        assert_eq!(steps[2].keys.as_deref(), Some("ctrl+s"));
    }

    #[test]
    fn roundtrip_save_lookup_fail() {
        let dir = std::env::temp_dir().join(format!("anya-pb-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        store::set_dir_override(Some(dir.clone()));
        let marker = format!("test-intent-{}", Uuid::new_v4());
        let save = dispatch(&json!({
            "op": "save",
            "intent": marker,
            "app": "Notepad",
            "exe": "notepad",
            "steps": [
                { "kind": "window", "action": "launch", "name": "notepad" },
                { "kind": "act", "action": "type", "role": "edit", "text": "x" }
            ],
            "success_assert": "text contains x"
        }))
        .expect("save");
        assert!(save.contains("Saved playbook"), "{save}");
        let id = save.split('`').nth(1).expect("id").to_string();

        let found = dispatch(&json!({ "op": "lookup", "query": marker, "exe": "notepad" }))
            .expect("lookup");
        assert!(found.contains(&id), "{found}");

        let fail = dispatch(&json!({ "op": "fail", "id": id, "step": 2, "reason": "ui moved" }))
            .expect("fail");
        assert!(fail.contains("Recorded fail"), "{fail}");

        let _ = dispatch(&json!({ "op": "delete", "id": id }));
        store::set_dir_override(None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
