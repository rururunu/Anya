use serde_json::json;
use tauri::AppHandle;
use tauri::Manager;

use crate::app_state::AppState;

pub(super) async fn list_remote_models_payload(app: &AppHandle) -> serde_json::Value {
    let settings = crate::services::settings_store::get_settings(app).ok();
    let models = match crate::commands::chat::list_chat_models(app.clone()).await {
        Ok(models) => models,
        Err(_) => Vec::new(),
    };
    let catalog = remote_provider_catalog(settings.as_ref(), &models);
    let enriched: Vec<serde_json::Value> = models
        .into_iter()
        .filter_map(|model| {
            let provider = model.provider.clone();
            let mut value = serde_json::to_value(model).ok()?;
            if let Some(meta) = catalog.get(&provider) {
                if let Some(obj) = value.as_object_mut() {
                    obj.insert("providerName".into(), json!(meta.name));
                    if let Some(preset) = meta.preset_id.as_ref() {
                        obj.insert("providerPresetId".into(), json!(preset));
                    }
                    if let Some(favicon) = meta.favicon_url.as_ref() {
                        obj.insert("providerFaviconUrl".into(), json!(favicon));
                    }
                }
            }
            Some(value)
        })
        .collect();
    json!({
        "models": enriched,
        "providers": catalog.values().collect::<Vec<_>>(),
    })
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoteProviderMeta {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    preset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    favicon_url: Option<String>,
}

fn remote_provider_catalog(
    settings: Option<&crate::models::settings::AppSettings>,
    models: &[crate::models::chat::ChatModelInfo],
) -> std::collections::BTreeMap<String, RemoteProviderMeta> {
    let mut catalog = std::collections::BTreeMap::new();
    let custom = settings
        .map(|item| item.custom_providers.as_slice())
        .unwrap_or(&[]);
    for model in models {
        let key = model.provider.trim();
        if key.is_empty() || catalog.contains_key(key) {
            continue;
        }
        catalog.insert(key.to_string(), remote_provider_meta(key, custom));
    }
    catalog
}

fn remote_provider_meta(
    provider_id: &str,
    custom_providers: &[crate::models::settings::CustomProviderConfig],
) -> RemoteProviderMeta {
    let custom = custom_providers.iter().find(|item| item.id == provider_id);
    RemoteProviderMeta {
        id: provider_id.to_string(),
        name: remote_provider_display_name(provider_id, custom),
        preset_id: custom
            .and_then(|item| item.preset_id.as_ref())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or_else(|| match provider_id {
                "deepseek" | "gemini" => Some(provider_id.to_string()),
                _ => None,
            }),
        favicon_url: custom
            .and_then(|item| favicon_url_for_base(&item.base_url))
            .filter(|_| provider_id != "deepseek" && provider_id != "gemini"),
    }
}

fn remote_provider_display_name(
    provider_id: &str,
    custom: Option<&crate::models::settings::CustomProviderConfig>,
) -> String {
    if provider_id == "deepseek" {
        return "DeepSeek".into();
    }
    if provider_id == "gemini" {
        return "Gemini".into();
    }
    if let Some(name) = custom
        .map(|item| item.name.trim())
        .filter(|name| !name.is_empty())
    {
        return name.to_string();
    }
    let preset = custom
        .and_then(|item| item.preset_id.as_deref())
        .unwrap_or("")
        .trim();
    match if preset.is_empty() {
        provider_id
    } else {
        preset
    } {
        "mimo" => "小米 MiMo".into(),
        "zhipu" | "glm" => "智谱 GLM".into(),
        "volcengine" => "火山方舟".into(),
        "minimax" => "MiniMax".into(),
        "kimi" | "moonshot" => "Kimi".into(),
        "openai" => "OpenAI".into(),
        "claude" | "anthropic" => "Claude".into(),
        "grok" | "xai" => "Grok".into(),
        "qwen" => "通义千问".into(),
        other => other
            .split(|ch: char| ch == '-' || ch == '_' || ch.is_whitespace())
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn favicon_url_for_base(base_url: &str) -> Option<String> {
    let trimmed = base_url.trim();
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))?;
    let host = rest
        .split('/')
        .next()?
        .split(':')
        .next()?
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');
    if host.is_empty() {
        return None;
    }
    Some(format!(
        "https://www.google.com/s2/favicons?sz=64&domain={host}"
    ))
}

/// Image-mode choices for Companion: current selection, every configured image provider's
/// enabled models, and the user's custom style templates (prompt only; example images stay
/// on the desktop and are resolved from `styleId` at send time).
pub(super) fn image_gen_options_payload(
    settings: &crate::models::settings::AppSettings,
) -> serde_json::Value {
    let choices: Vec<serde_json::Value> = settings
        .image_providers
        .iter()
        .flat_map(|provider| {
            let disabled: std::collections::HashSet<&str> = provider
                .disabled_models
                .split([',', '\n'])
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .collect();
            let name = if provider.name.trim().is_empty() {
                provider.id.clone()
            } else {
                provider.name.trim().to_string()
            };
            provider
                .models
                .split([',', '\n'])
                .map(str::trim)
                .filter(|id| !id.is_empty() && !disabled.contains(id))
                .map(|id| {
                    json!({
                        "provider": provider.id,
                        "providerName": name,
                        "model": id,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let styles: Vec<serde_json::Value> = settings
        .image_style_templates
        .iter()
        .map(|template| {
            json!({
                "id": template.id,
                "name": template.name,
                "prompt": template.prompt,
                "hasExampleImage": template.example_image.is_some(),
            })
        })
        .collect();
    json!({
        "model": settings.image_model,
        "provider": settings.image_model_provider,
        "choices": choices,
        "styles": styles,
    })
}

pub(super) async fn session_history(app: &AppHandle, session_id: &str) -> serde_json::Value {
    let Some(state) = app.try_state::<AppState>() else {
        return json!({ "sessionId": session_id, "messages": [] });
    };
    // Companion renders the same turn footer as the desktop ("已处理 12 s"), which
    // needs the persisted completion time — the runtime message does not carry it.
    let completed_at = crate::core::chat::db::load_message_completed_at(
        &state.core.chat().conversation().db_pool(),
        session_id,
    )
    .await
    .unwrap_or_default();
    let cache_usages: Vec<serde_json::Value> = crate::core::chat::db::load_message_cache_usages(
        &state.core.chat().conversation().db_pool(),
        session_id,
    )
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|usage| {
        json!({
            "messageId": usage.message_id,
            "inputTokens": usage.input_tokens,
            "cacheReadTokens": usage.cache_read_tokens,
        })
    })
    .collect();
    // Rewind points: one per user turn that ran with a workspace snapshot.
    let checkpoints: Vec<serde_json::Value> = crate::core::checkpoint::shared_checkpoint_store()
        .list(session_id)
        .unwrap_or_default()
        .into_iter()
        .map(|checkpoint| {
            json!({
                "turn": checkpoint.turn,
                "timeEpochMs": checkpoint.time,
                "prompt": checkpoint.prompt,
                "userMessageId": checkpoint.user_message_id,
                "fileCount": checkpoint.files.len(),
            })
        })
        .collect();
    match state.core.chat().history(session_id) {
        Ok(messages) => {
            let mapped: Vec<serde_json::Value> = messages
                .into_iter()
                .filter(|message| {
                    !matches!(
                        message.role,
                        crate::core::runtime::Role::Tool | crate::core::runtime::Role::System
                    )
                })
                .map(|message| {
                    let completed = completed_at.get(&message.id).copied();
                    remote_chat_message(message, completed)
                })
                .collect();
            json!({
                "sessionId": session_id,
                "messages": mapped,
                "planModeActive": crate::core::tools::plan_mode::shared_plan_mode_store()
                    .is_active(session_id),
                "messageCacheUsages": cache_usages,
                "checkpoints": checkpoints,
            })
        }
        Err(_) => json!({
            "sessionId": session_id,
            "messages": [],
            "planModeActive": crate::core::tools::plan_mode::shared_plan_mode_store()
                .is_active(session_id),
            "messageCacheUsages": cache_usages,
            "checkpoints": checkpoints,
        }),
    }
}

fn remote_chat_message(
    message: crate::core::runtime::ChatMessage,
    completed_at: Option<u64>,
) -> serde_json::Value {
    use crate::core::runtime::{MessageStatus, Role};
    let role = match message.role {
        Role::User => "User",
        Role::Assistant => "Assistant",
        Role::System => "System",
        Role::Tool => "System",
    };
    let status = match message.status {
        MessageStatus::Pending => "Pending",
        MessageStatus::Streaming => "Streaming",
        MessageStatus::Done => "Complete",
        MessageStatus::Error => "Error",
        MessageStatus::Cancelled => "Cancelled",
    };
    let code_changes = extract_code_changes(&message);
    let plan_tasks = extract_plan_tasks(&message);
    let tool_activities = message
        .tool_activities
        .as_ref()
        .map(|activities| {
            activities
                .iter()
                .map(|activity| {
                    json!({
                        "id": activity.id,
                        "subagentId": activity.subagent_id,
                        "parentActivityId": activity.parent_activity_id,
                        "toolName": activity.tool_name,
                        "title": activity.title,
                        "kind": activity.kind,
                        "detail": activity.detail,
                        "arguments": activity.arguments,
                        "result": activity.result,
                        "preview": activity.preview.as_ref().map(|preview| json!({
                            "path": preview.path,
                            "unifiedDiff": preview.unified_diff,
                            "affectedPaths": preview.affected_paths,
                        })),
                        "success": activity.success,
                        "status": activity.status,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    // Same wire shape as the desktop store: `{type, id, content | toolActivityId}`.
    let work_timeline = message
        .work_timeline
        .as_ref()
        .map(|items| serde_json::to_value(items).unwrap_or(serde_json::Value::Array(Vec::new())))
        .unwrap_or(serde_json::Value::Array(Vec::new()));
    json!({
        "id": message.id,
        "sessionId": message.session_id,
        "role": role,
        "content": message.content,
        "reasoning": message.reasoning,
        "status": status,
        "createdAtEpochMs": message.timestamp,
        "completedAtEpochMs": completed_at,
        "estimatedTokens": message.estimated_tokens,
        "codeChanges": code_changes,
        "planTasks": plan_tasks,
        "toolActivities": tool_activities,
        "workTimeline": work_timeline,
    })
}

fn extract_code_changes(message: &crate::core::runtime::ChatMessage) -> Vec<serde_json::Value> {
    let Some(activities) = message.tool_activities.as_ref() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for activity in activities {
        if activity.status != "done" || !activity.success {
            continue;
        }
        if let Some(preview) = activity.preview.as_ref() {
            let (added, removed) = count_diff_lines(&preview.unified_diff);
            if !preview.path.is_empty()
                && (added > 0 || removed > 0 || !preview.unified_diff.is_empty())
            {
                out.push(json!({
                    "id": format!("{}:{}", message.id, activity.id),
                    "path": preview.path,
                    "added": added,
                    "removed": removed,
                }));
            }
            for path in &preview.affected_paths {
                if path != &preview.path && !path.is_empty() {
                    out.push(json!({
                        "id": format!("{}:{}:{}", message.id, activity.id, path),
                        "path": path,
                        "added": 0,
                        "removed": 0,
                    }));
                }
            }
        } else if let Some(args) = activity.arguments.as_ref() {
            if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                if matches!(
                    activity.tool_name.as_str(),
                    "write_file" | "replace_in_file" | "replace_many_in_file" | "apply_patch"
                ) {
                    out.push(json!({
                        "id": format!("{}:{}", message.id, activity.id),
                        "path": path,
                        "added": 0,
                        "removed": 0,
                    }));
                }
            }
        }
    }
    out
}

fn extract_plan_tasks(message: &crate::core::runtime::ChatMessage) -> Vec<serde_json::Value> {
    let Some(activities) = message.tool_activities.as_ref() else {
        return Vec::new();
    };
    for activity in activities.iter().rev() {
        if !matches!(activity.tool_name.as_str(), "update_tasks" | "todo_write") {
            continue;
        }
        let Some(args) = activity.arguments.as_ref() else {
            continue;
        };
        let Some(tasks) = args.get("tasks").and_then(|v| v.as_array()) else {
            continue;
        };
        return tasks
            .iter()
            .filter_map(|task| {
                let content = task.get("content")?.as_str()?.to_string();
                let status = task
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("pending")
                    .to_string();
                let level = task.get("level").and_then(|v| v.as_i64()).unwrap_or(0);
                Some(json!({
                    "content": content,
                    "status": status,
                    "level": level,
                }))
            })
            .collect();
    }
    Vec::new()
}

fn count_diff_lines(diff: &str) -> (usize, usize) {
    let mut added = 0usize;
    let mut removed = 0usize;
    for line in diff.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            added += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            removed += 1;
        }
    }
    (added, removed)
}
pub(super) fn list_skills_payload(app: &AppHandle) -> serde_json::Value {
    let enabled_builtins: std::collections::HashSet<String> =
        crate::services::settings_store::get_settings(app)
            .ok()
            .map(|settings| {
                settings
                    .enabled_builtin_skills
                    .into_iter()
                    .collect::<std::collections::HashSet<_>>()
            })
            .unwrap_or_default();
    let skills = crate::core::tools::skills::list_skill_infos()
        .unwrap_or_default()
        .into_iter()
        .filter(|skill| skill.source != "builtin" || enabled_builtins.contains(&skill.name))
        .map(|skill| {
            let icon_url =
                resolve_remote_icon_url(app, "skill", &skill.name, skill.icon_url.as_deref());
            json!({
                "id": skill.name,
                "name": skill.name,
                "title": skill.title,
                "description": skill.description,
                "source": skill.source,
                "iconUrl": icon_url,
            })
        })
        .collect::<Vec<_>>();
    json!({ "skills": skills })
}

pub(super) fn list_mcp_payload(app: &AppHandle) -> serde_json::Value {
    let servers = crate::services::settings_store::get_settings(app)
        .ok()
        .map(|settings| settings.mcp_servers)
        .unwrap_or_default()
        .into_iter()
        .filter(|server| server.enabled)
        .map(|server| {
            let icon_url =
                resolve_remote_icon_url(app, "mcp", &server.id, server.icon_url.as_deref());
            json!({
                "id": server.id,
                "title": server.title.unwrap_or_else(|| server.id.clone()),
                "description": server.description.unwrap_or_default(),
                "qualifiedName": server.qualified_name,
                "iconUrl": icon_url,
            })
        })
        .collect::<Vec<_>>();
    json!({ "mcpServers": servers })
}

/// Prefer an http(s)/data icon URL for the phone companion; fall back to disk cache.
fn resolve_remote_icon_url(
    app: &AppHandle,
    kind: &str,
    cache_key: &str,
    remote: Option<&str>,
) -> Option<String> {
    if let Some(url) = remote.map(str::trim).filter(|u| !u.is_empty()) {
        if url.starts_with("https://") || url.starts_with("http://") || url.starts_with("data:") {
            return Some(url.to_string());
        }
    }
    crate::commands::icons::install_icon_data_url(app, kind, cache_key)
}
