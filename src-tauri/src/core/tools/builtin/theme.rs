//! Custom theme management tool for Agent.
//! Allows creating, updating, previewing, listing, applying, and deleting custom themes.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::models::settings::{ColorScheme, CustomThemeConfig, ThemeBackgroundConfig};

pub struct ManageCustomThemeTool;

impl Tool for ManageCustomThemeTool {
    fn name(&self) -> &str {
        "manage_custom_theme"
    }

    fn description(&self) -> &str {
        "Manage Anya's application themes and backgrounds. Create, update, apply, list, delete custom themes or set a custom background image with instantaneous UI response."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "update", "set_background", "apply", "list", "delete"],
                    "description": "Action to perform on custom themes or backgrounds"
                },
                "id": {
                    "type": "string",
                    "description": "Theme ID (kebab-case, e.g. 'custom-eva-01' or 'ocean'). Optional for 'create', 'list', and 'set_background'."
                },
                "name": {
                    "type": "string",
                    "description": "Display name of the theme, e.g. 'EVA 初号机' or 'Cyberpunk Neon'. Required for create."
                },
                "mode": {
                    "type": "string",
                    "enum": ["light", "dark"],
                    "description": "Base color mode ('light' or 'dark'). Inherits fallback colors from this mode and controls window chrome."
                },
                "description": {
                    "type": "string",
                    "description": "Short explanation of theme aesthetic and palette rationale."
                },
                "tokens": {
                    "type": "object",
                    "description": "Map of CSS custom property tokens, e.g. {\"--peek-bg\": \"#120d1c\", \"--peek-accent\": \"#39ff14\"}",
                    "additionalProperties": { "type": "string" }
                },
                "background": {
                    "type": "object",
                    "description": "Background image configuration. e.g. {\"image\": \"path:C:/.../img.png\", \"opacity\": 0.2, \"blur\": 0, \"fit\": \"cover\"}",
                    "properties": {
                        "image": { "type": "string", "description": "Local file path, path: URI, or URL to the background image" },
                        "opacity": { "type": "number", "minimum": 0.0, "maximum": 1.0, "description": "Background opacity (0.05 - 1.0, default 0.2 for optimal readability)" },
                        "blur": { "type": "integer", "minimum": 0, "maximum": 50, "description": "Background blur radius in px (default 0)" },
                        "fit": { "type": "string", "enum": ["cover", "contain", "fill", "center"], "description": "Background sizing fit (default 'cover')" }
                    }
                },
                "backgroundImage": {
                    "type": "string",
                    "description": "Shortcut for background.image"
                },
                "backgroundOpacity": {
                    "type": "number",
                    "description": "Shortcut for background.opacity"
                },
                "backgroundBlur": {
                    "type": "integer",
                    "description": "Shortcut for background.blur"
                },
                "backgroundFit": {
                    "type": "string",
                    "description": "Shortcut for background.fit"
                },
                "clearBackground": {
                    "type": "boolean",
                    "description": "Set true to clear/remove the background image"
                }
            },
            "required": ["action"]
        })
    }

    fn read_only(&self) -> bool {
        false
    }

    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let action = args["action"].as_str().unwrap_or("").trim();
        if action.is_empty() {
            return Err(ToolError::new(
                "action is required ('create', 'update', 'apply', 'list', 'delete')",
            ));
        }

        let Some(app) = &ctx.app_handle else {
            return Err(ToolError::new(
                "Tauri application context unavailable for theme management",
            ));
        };

        let mut current_settings = crate::services::settings_store::get_settings(app)
            .map_err(|e| ToolError::new(format!("failed to load settings: {e}")))?;

        fn parse_background_update(
            args: &Value,
            current: Option<&ThemeBackgroundConfig>,
        ) -> (bool, Option<ThemeBackgroundConfig>) {
            let clear = args["clearBackground"].as_bool() == Some(true)
                || (args.get("background").is_some() && args["background"].is_null())
                || (args.get("backgroundImage").is_some()
                    && args["backgroundImage"].as_str().map(str::trim) == Some(""));

            if clear {
                return (true, None);
            }

            let bg_obj = args.get("background").and_then(Value::as_object);
            let image_val = bg_obj
                .and_then(|o| o.get("image"))
                .and_then(Value::as_str)
                .or_else(|| args["backgroundImage"].as_str());

            let opacity_val = bg_obj
                .and_then(|o| o.get("opacity"))
                .and_then(Value::as_f64)
                .or_else(|| args["backgroundOpacity"].as_f64());

            let blur_val = bg_obj
                .and_then(|o| o.get("blur"))
                .and_then(Value::as_u64)
                .or_else(|| args["backgroundBlur"].as_u64())
                .map(|v| v as u32);

            let fit_val = bg_obj
                .and_then(|o| o.get("fit"))
                .and_then(Value::as_str)
                .or_else(|| args["backgroundFit"].as_str());

            if image_val.is_none()
                && opacity_val.is_none()
                && blur_val.is_none()
                && fit_val.is_none()
            {
                return (false, current.cloned());
            }

            let merged_image = image_val
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .or_else(|| current.and_then(|c| c.image.clone()));

            let merged_opacity = opacity_val
                .map(|o| o.clamp(0.0, 1.0))
                .or_else(|| current.and_then(|c| c.opacity))
                .or(Some(0.2));

            let merged_blur = blur_val
                .or_else(|| current.and_then(|c| c.blur))
                .or(Some(0));

            let merged_fit = fit_val
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .or_else(|| current.and_then(|c| c.fit.clone()))
                .or(Some("cover".to_string()));

            (
                true,
                Some(ThemeBackgroundConfig {
                    image: merged_image,
                    opacity: merged_opacity,
                    blur: merged_blur,
                    fit: merged_fit,
                }),
            )
        }

        match action {
            "list" => {
                let current_id = current_settings.color_scheme.as_str();
                let result = json!({
                    "activeTheme": current_id,
                    "isDarkMode": current_settings.is_dark_mode(),
                    "customThemes": current_settings.custom_themes,
                    "customBackground": current_settings.custom_background,
                    "builtinPresets": [
                        {"id": "light", "mode": "light", "name": "Light (Default)"},
                        {"id": "dark", "mode": "dark", "name": "Dark (Default)"}
                    ]
                });
                serde_json::to_string_pretty(&result).map_err(|e| ToolError::new(e.to_string()))
            }
            "set_background" => {
                let id = args["id"].as_str().map(str::trim).filter(|s| !s.is_empty());
                let target_theme_id =
                    id.map(ToString::to_string)
                        .or_else(|| match &current_settings.color_scheme {
                            ColorScheme::Custom(active_id) => Some(active_id.clone()),
                            _ => None,
                        });

                if let Some(target_id) = target_theme_id {
                    if let Some(theme) = current_settings
                        .custom_themes
                        .iter_mut()
                        .find(|t| t.id == target_id)
                    {
                        let (changed, next_bg) =
                            parse_background_update(&args, theme.background.as_ref());
                        if changed {
                            theme.background = next_bg;
                            theme.updated_at = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();
                            crate::services::settings_store::set_settings(app, current_settings)
                                .map_err(|e| {
                                    ToolError::new(format!("failed to save theme background: {e}"))
                                })?;
                            return Ok(format!(
                                "Background updated successfully for theme '{target_id}'."
                            ));
                        } else {
                            return Err(ToolError::new("no background image or settings provided"));
                        }
                    } else {
                        return Err(ToolError::new(format!(
                            "custom theme '{target_id}' not found"
                        )));
                    }
                } else {
                    let (changed, next_bg) =
                        parse_background_update(&args, current_settings.custom_background.as_ref());
                    if changed {
                        current_settings.custom_background = next_bg;
                        crate::services::settings_store::set_settings(app, current_settings)
                            .map_err(|e| {
                                ToolError::new(format!("failed to save custom background: {e}"))
                            })?;
                        return Ok("Custom background updated successfully.".to_string());
                    } else {
                        return Err(ToolError::new("no background image or settings provided"));
                    }
                }
            }
            "create" | "update" => {
                let mode = args["mode"].as_str().unwrap_or("").trim();
                let mode = if mode == "light" || mode == "dark" {
                    mode.to_string()
                } else if action == "update" {
                    String::new()
                } else {
                    return Err(ToolError::new(
                        "mode ('light' or 'dark') is required when creating a theme",
                    ));
                };

                let name = args["name"].as_str().unwrap_or("").trim().to_string();
                if name.is_empty() && action == "create" {
                    return Err(ToolError::new("name is required when creating a theme"));
                }

                let id = args["id"]
                    .as_str()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| {
                        let slug: String = name
                            .chars()
                            .map(|c| {
                                if c.is_alphanumeric() {
                                    c.to_ascii_lowercase()
                                } else {
                                    '-'
                                }
                            })
                            .collect();
                        let trimmed = slug.trim_matches('-');
                        if trimmed.is_empty() {
                            format!("custom-{}", uuid::Uuid::new_v4().simple())
                        } else if trimmed.starts_with("custom-") {
                            trimmed.to_string()
                        } else {
                            format!("custom-{}", trimmed)
                        }
                    });

                let mut tokens: HashMap<String, String> = HashMap::new();
                if let Some(obj) = args["tokens"].as_object() {
                    for (k, v) in obj {
                        if let Some(s) = v.as_str() {
                            let key = if k.starts_with("--") {
                                k.to_string()
                            } else {
                                format!("--{k}")
                            };
                            tokens.insert(key, s.to_string());
                        }
                    }
                }

                let desc = args["description"]
                    .as_str()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                if let Some(pos) = current_settings
                    .custom_themes
                    .iter()
                    .position(|t| t.id == id)
                {
                    let existing = &mut current_settings.custom_themes[pos];
                    if !name.is_empty() {
                        existing.name = name;
                    }
                    if !mode.is_empty() {
                        existing.mode = mode;
                    }
                    if desc.is_some() {
                        existing.description = desc;
                    }
                    for (k, v) in tokens {
                        existing.tokens.insert(k, v);
                    }
                    let (bg_changed, next_bg) =
                        parse_background_update(&args, existing.background.as_ref());
                    if bg_changed {
                        existing.background = next_bg;
                    }
                    existing.updated_at = now;
                } else {
                    let (_, next_bg) = parse_background_update(&args, None);
                    current_settings.custom_themes.push(CustomThemeConfig {
                        id: id.clone(),
                        name: if name.is_empty() { id.clone() } else { name },
                        mode: if mode.is_empty() {
                            "dark".to_string()
                        } else {
                            mode
                        },
                        description: desc,
                        tokens,
                        background: next_bg,
                        updated_at: now,
                    });
                }

                current_settings.color_scheme = ColorScheme::Custom(id.clone());

                crate::services::settings_store::set_settings(app, current_settings)
                    .map_err(|e| ToolError::new(format!("failed to save updated theme: {e}")))?;

                Ok(format!("Theme '{id}' saved and activated successfully."))
            }
            "apply" => {
                let id = args["id"].as_str().unwrap_or("").trim();
                if id.is_empty() {
                    return Err(ToolError::new("id is required to apply a theme"));
                }
                let scheme = match id {
                    "light" => ColorScheme::Light,
                    "dark" => ColorScheme::Dark,
                    other => ColorScheme::Custom(other.to_string()),
                };
                current_settings.color_scheme = scheme;
                crate::services::settings_store::set_settings(app, current_settings)
                    .map_err(|e| ToolError::new(format!("failed to apply theme: {e}")))?;
                Ok(format!("Theme '{id}' applied successfully."))
            }
            "delete" => {
                let id = args["id"].as_str().unwrap_or("").trim();
                if id.is_empty() {
                    return Err(ToolError::new("id is required to delete a theme"));
                }
                let before_len = current_settings.custom_themes.len();
                current_settings.custom_themes.retain(|t| t.id != id);
                if current_settings.custom_themes.len() == before_len {
                    return Err(ToolError::new(format!("custom theme '{id}' not found")));
                }
                if let ColorScheme::Custom(active_id) = &current_settings.color_scheme {
                    if active_id == id {
                        current_settings.color_scheme = ColorScheme::Dark;
                    }
                }
                crate::services::settings_store::set_settings(app, current_settings).map_err(
                    |e| {
                        ToolError::new(format!("failed to save settings after theme deletion: {e}"))
                    },
                )?;
                Ok(format!("Custom theme '{id}' deleted."))
            }
            other => Err(ToolError::new(format!(
                "unknown action '{other}'. Valid actions: create, update, apply, list, delete"
            ))),
        }
    }
}
