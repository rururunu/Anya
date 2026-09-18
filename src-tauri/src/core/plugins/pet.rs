//! Built-in host RPC: desktop pet visibility, size, appearance, and expression.

use serde_json::{json, Value};
use tauri::AppHandle;

use super::grant::grant_has;
use crate::core::tools::error::ToolError;
use crate::services::desktop_pet::{
    clear_pet_appearance, get_pet_appearance, get_pet_size, is_desktop_pet_visible,
    set_pet_appearance, set_pet_expression, set_pet_size, toggle_desktop_pet, PetAppearance,
    PetSize,
};

const MEDIA_KINDS: &[&str] = &["image", "video", "lottie", "svg", "html"];

/// Host RPC `pet.*` (workbench `ctx.host.rpc`).
pub fn dispatch_rpc(
    app: &AppHandle,
    plugin_id: &str,
    method: &str,
    params: &Value,
) -> Result<Value, ToolError> {
    if !grant_has(plugin_id, "pet") {
        return Err(ToolError::new("plugin is not granted pet"));
    }
    let action = method
        .strip_prefix("pet.")
        .filter(|name| !name.is_empty())
        .ok_or_else(|| ToolError::new(format!("unknown pet method: {method}")))?;
    match action {
        "show" => Ok(json!({ "visible": toggle_desktop_pet(app, Some(true)) })),
        "hide" => Ok(json!({ "visible": toggle_desktop_pet(app, Some(false)) })),
        "toggle" => {
            let visible = params.get("visible").and_then(|value| value.as_bool());
            Ok(json!({ "visible": toggle_desktop_pet(app, visible) }))
        }
        "visible" => Ok(json!({ "visible": is_desktop_pet_visible(app) })),
        "setSize" => {
            let size = parse_size(params)?;
            set_pet_size(app, size);
            Ok(json!({ "size": size.as_str() }))
        }
        "getSize" => Ok(json!({ "size": get_pet_size(app).as_str() })),
        "setAppearance" => {
            let appearance = parse_appearance(plugin_id, params)?;
            set_pet_appearance(app, Some(appearance.clone()));
            Ok(json!({ "appearance": appearance }))
        }
        "getAppearance" => Ok(json!({ "appearance": get_pet_appearance(app) })),
        "clearAppearance" => {
            clear_pet_appearance(app);
            Ok(json!({ "cleared": true }))
        }
        "setMode" => {
            let appearance = parse_mode(params)?;
            set_pet_appearance(app, Some(appearance.clone()));
            Ok(json!({ "appearance": appearance }))
        }
        "setSkin" => {
            let appearance = parse_media_skin(plugin_id, params)?;
            set_pet_appearance(app, Some(appearance.clone()));
            Ok(json!({ "appearance": appearance, "skin": appearance }))
        }
        "clearSkin" => {
            clear_pet_appearance(app);
            Ok(json!({ "cleared": true }))
        }
        "setExpression" => {
            let expression = params
                .get("expression")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| ToolError::new("pet.setExpression requires expression"))?;
            if !is_known_expression(expression) {
                return Err(ToolError::new(format!(
                    "unknown pet expression: {expression}"
                )));
            }
            set_pet_expression(app, Some(expression.to_string()));
            Ok(json!({ "expression": expression }))
        }
        "clearExpression" => {
            set_pet_expression(app, None);
            Ok(json!({ "cleared": true }))
        }
        other => Err(ToolError::new(format!("unknown pet method: {other}"))),
    }
}

fn parse_size(params: &Value) -> Result<PetSize, ToolError> {
    let raw = params
        .get("size")
        .and_then(|value| value.as_str())
        .ok_or_else(|| ToolError::new("pet.setSize requires size"))?;
    PetSize::parse(raw).ok_or_else(|| ToolError::new(format!("invalid pet size: {raw}")))
}

fn parse_appearance(plugin_id: &str, params: &Value) -> Result<PetAppearance, ToolError> {
    let mode = params
        .get("mode")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ToolError::new("pet.setAppearance requires mode"))?;
    match mode {
        "mascot" => Ok(PetAppearance {
            mode: "mascot".to_string(),
            kind: None,
            source: None,
            config: None,
        }),
        "companion" => Ok(PetAppearance {
            mode: "companion".to_string(),
            kind: None,
            source: None,
            config: params.get("config").cloned(),
        }),
        "media" => parse_media_skin(plugin_id, params),
        "spritesheet" => parse_spritesheet(plugin_id, params),
        other => Err(ToolError::new(format!(
            "pet.setAppearance mode must be mascot|media|companion|spritesheet, got {other}"
        ))),
    }
}

fn parse_mode(params: &Value) -> Result<PetAppearance, ToolError> {
    let mode = params
        .get("mode")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ToolError::new("pet.setMode requires mode"))?;
    match mode {
        "mascot" => Ok(PetAppearance {
            mode: "mascot".to_string(),
            kind: None,
            source: None,
            config: None,
        }),
        "companion" => Ok(PetAppearance {
            mode: "companion".to_string(),
            kind: None,
            source: None,
            config: params.get("config").cloned(),
        }),
        other => Err(ToolError::new(format!(
            "pet.setMode mode must be mascot|companion, got {other}"
        ))),
    }
}

fn parse_spritesheet(plugin_id: &str, params: &Value) -> Result<PetAppearance, ToolError> {
    let source = params
        .get("source")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ToolError::new("pet.setAppearance spritesheet requires source (pet.json)"))?;
    Ok(PetAppearance {
        mode: "spritesheet".to_string(),
        kind: Some("spritesheet".to_string()),
        source: Some(resolve_media_url(plugin_id, source)),
        config: params.get("config").cloned(),
    })
}

fn parse_media_skin(plugin_id: &str, params: &Value) -> Result<PetAppearance, ToolError> {
    let kind = params
        .get("kind")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ToolError::new("pet media appearance requires kind"))?;
    if !MEDIA_KINDS.contains(&kind) {
        return Err(ToolError::new(format!(
            "pet media kind must be one of {}, got {kind}",
            MEDIA_KINDS.join("|")
        )));
    }
    let source = params
        .get("source")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ToolError::new("pet media appearance requires source"))?;
    Ok(PetAppearance {
        mode: "media".to_string(),
        kind: Some(kind.to_string()),
        source: Some(resolve_media_url(plugin_id, source)),
        config: None,
    })
}

fn resolve_media_url(plugin_id: &str, source: &str) -> String {
    if source.starts_with("http://")
        || source.starts_with("https://")
        || source.starts_with("data:")
        || source.starts_with("anya-plugin:")
        || source.starts_with("asset:")
        || source.starts_with("blob:")
    {
        return source.to_string();
    }
    let rel = source.trim_start_matches('/');
    format!("anya-plugin://localhost/{plugin_id}/{rel}")
}

fn is_known_expression(value: &str) -> bool {
    matches!(
        value,
        "idle" | "thinking" | "working" | "talking" | "waiting" | "done" | "error" | "sleeping"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_media_to_plugin_url() {
        assert_eq!(
            resolve_media_url("demo", "skins/cat.svg"),
            "anya-plugin://localhost/demo/skins/cat.svg"
        );
        assert_eq!(
            resolve_media_url("demo", "https://example.com/a.png"),
            "https://example.com/a.png"
        );
    }

    #[test]
    fn parses_companion_mode() {
        let appearance = parse_mode(&json!({ "mode": "companion", "config": { "accent": "#3366ff" } }))
            .expect("companion mode");
        assert_eq!(appearance.mode, "companion");
        assert!(appearance.config.is_some());
    }

    #[test]
    fn parses_media_appearance_with_svg() {
        let appearance =
            parse_media_skin("demo", &json!({ "kind": "svg", "source": "orb.svg" })).expect("svg");
        assert_eq!(appearance.mode, "media");
        assert_eq!(appearance.kind.as_deref(), Some("svg"));
    }

    #[test]
    fn parses_spritesheet_appearance() {
        let appearance = parse_appearance(
            "demo",
            &json!({ "mode": "spritesheet", "source": "ui/pets/socksy/pet.json" }),
        )
        .expect("spritesheet");
        assert_eq!(appearance.mode, "spritesheet");
        assert_eq!(appearance.kind.as_deref(), Some("spritesheet"));
        assert_eq!(
            appearance.source.as_deref(),
            Some("anya-plugin://localhost/demo/ui/pets/socksy/pet.json")
        );
    }

    #[test]
    fn accepts_known_expressions() {
        assert!(is_known_expression("idle"));
        assert!(is_known_expression("sleeping"));
        assert!(!is_known_expression("dance"));
    }

    #[test]
    fn ungrafted_plugin_is_rejected() {
        assert!(!grant_has("never-enabled-plugin", "pet"));
    }
}
