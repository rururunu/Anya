use serde_json::Value;

/// A chat-model identity: optional provider hint plus the API model id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRef {
    pub provider: String,
    pub id: String,
}

/// Parse a stored or requested model token.
///
/// Settings checkboxes store `JSON.stringify([provider, id])`. Coordinators
/// usually pass the bare API id (`deepseek-v4-pro`). Accept both, plus a
/// two-element JSON array in tool arguments.
pub fn parse_model_ref(token: &str) -> ModelRef {
    let token = token.trim();
    if token.is_empty() {
        return ModelRef {
            provider: String::new(),
            id: String::new(),
        };
    }
    if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(token) {
        if let Some(parsed) = ref_from_array(&items) {
            return parsed;
        }
    }
    ModelRef {
        provider: String::new(),
        id: token.to_string(),
    }
}

/// Read `model` from a tool-call argument that may be a string or `[provider, id]`.
pub fn model_ref_from_value(value: &Value) -> Option<ModelRef> {
    match value {
        Value::Null => None,
        Value::String(token) => {
            let parsed = parse_model_ref(token);
            if parsed.id.is_empty() {
                None
            } else {
                Some(parsed)
            }
        }
        Value::Array(items) => ref_from_array(items).filter(|parsed| !parsed.id.is_empty()),
        _ => None,
    }
}

fn ref_from_array(items: &[Value]) -> Option<ModelRef> {
    if items.len() != 2 {
        return None;
    }
    let provider = items[0].as_str()?.trim();
    let id = items[1].as_str()?.trim();
    if id.is_empty() {
        return None;
    }
    Some(ModelRef {
        provider: provider.to_string(),
        id: id.to_string(),
    })
}

/// Pick the allow-list entry that matches `requested` (bare id or encoded pair).
pub fn select_collaboration_model(
    allowed: &[String],
    requested: &str,
) -> Result<(String, String), String> {
    let requested = parse_model_ref(requested);
    if requested.id.is_empty() {
        return Err("model is empty".into());
    }

    let matches: Vec<ModelRef> = allowed
        .iter()
        .map(|entry| parse_model_ref(entry))
        .filter(|entry| !entry.id.is_empty() && entry.id == requested.id)
        .filter(|entry| {
            requested.provider.is_empty()
                || entry.provider.is_empty()
                || entry.provider == requested.provider
        })
        .collect();

    let Some(chosen) = matches.into_iter().next() else {
        let enabled = if allowed.is_empty() {
            "(none)".to_string()
        } else {
            allowed
                .iter()
                .map(|entry| parse_model_ref(entry).id)
                .filter(|id| !id.is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        };
        return Err(format!(
            "model `{}` is not enabled for collaboration; enabled: {enabled}",
            requested.id
        ));
    };

    let provider = if !requested.provider.is_empty() {
        requested.provider
    } else {
        chosen.provider
    };
    Ok((provider, requested.id))
}

/// Bullet list of API model ids for the coordinator prompt.
pub fn format_collaboration_prompt_ids(allowed: &[String]) -> String {
    let parsed: Vec<ModelRef> = allowed
        .iter()
        .map(|entry| parse_model_ref(entry))
        .filter(|entry| !entry.id.is_empty())
        .collect();
    parsed
        .iter()
        .map(|entry| {
            let duplicate = parsed.iter().filter(|other| other.id == entry.id).count() > 1;
            if duplicate && !entry.provider.is_empty() {
                format!("- `{}` (provider `{}`)", entry.id, entry.provider)
            } else {
                format!("- `{}`", entry.id)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_frontend_pair_and_bare_id() {
        assert_eq!(
            parse_model_ref(r#"["deepseek","deepseek-v4-pro"]"#),
            ModelRef {
                provider: "deepseek".into(),
                id: "deepseek-v4-pro".into(),
            }
        );
        assert_eq!(
            parse_model_ref("deepseek-v4-flash"),
            ModelRef {
                provider: String::new(),
                id: "deepseek-v4-flash".into(),
            }
        );
    }

    #[test]
    fn reads_model_arg_as_string_or_pair_array() {
        assert_eq!(
            model_ref_from_value(&json!("deepseek-v4-pro")).unwrap().id,
            "deepseek-v4-pro"
        );
        let parsed = model_ref_from_value(&json!(["deepseek", "deepseek-v4-flash"])).unwrap();
        assert_eq!(parsed.provider, "deepseek");
        assert_eq!(parsed.id, "deepseek-v4-flash");
        assert!(model_ref_from_value(&json!(null)).is_none());
    }

    #[test]
    fn bare_id_matches_encoded_allow_list() {
        let allowed = vec![
            r#"["deepseek","deepseek-v4-pro"]"#.into(),
            r#"["deepseek","deepseek-v4-flash"]"#.into(),
        ];
        assert_eq!(
            select_collaboration_model(&allowed, "deepseek-v4-pro").unwrap(),
            ("deepseek".into(), "deepseek-v4-pro".into())
        );
        assert_eq!(
            select_collaboration_model(&allowed, r#"["deepseek","deepseek-v4-flash"]"#).unwrap(),
            ("deepseek".into(), "deepseek-v4-flash".into())
        );
        let err = select_collaboration_model(&allowed, "gpt-4o").unwrap_err();
        assert!(err.contains("deepseek-v4-pro"));
        assert!(err.contains("not enabled"));
    }

    #[test]
    fn prompt_lists_api_ids_not_json_pairs() {
        let allowed = vec![
            r#"["deepseek","deepseek-v4-pro"]"#.into(),
            r#"["deepseek","deepseek-v4-flash"]"#.into(),
        ];
        let listed = format_collaboration_prompt_ids(&allowed);
        assert_eq!(
            listed,
            "- `deepseek-v4-pro`\n- `deepseek-v4-flash`"
        );
        assert!(!listed.contains('['));
    }
}
