use std::sync::Arc;

use tauri::AppHandle;

use super::antigravity::AntigravityProvider;
use super::deepseek::DeepSeekProvider;
use super::provider::AIProvider;
use crate::models::settings::{
    AppSettings, CustomProviderConfig, ModelWireProtocol, ProviderApiProtocol, ReasoningEffort,
};
use crate::services::gemini_oauth;
use crate::services::settings_store;

/// Resolve the provider selected for the primary chat model.
pub fn resolve_provider(app: AppHandle) -> Arc<dyn AIProvider> {
    let settings = settings_store::get_settings(&app).unwrap_or_default();
    let model = settings.chat_model.trim().to_string();
    let provider = settings.chat_model_provider.trim().to_string();
    resolve_provider_for_selection(app, model, provider)
}

/// Resolve a provider by model only for callers that do not own a provider selection.
pub fn resolve_provider_for_model(app: AppHandle, model: String) -> Arc<dyn AIProvider> {
    resolve_provider_for_selection(app, model, String::new())
}

fn provider_has_model(provider: &CustomProviderConfig, model: &str) -> bool {
    provider
        .models
        .split([',', '\n'])
        .map(str::trim)
        .any(|id| !id.is_empty() && id == model)
}

pub fn provider_model_is_disabled(provider: &CustomProviderConfig, model: &str) -> bool {
    provider
        .disabled_models
        .split([',', '\n'])
        .map(str::trim)
        .any(|id| !id.is_empty() && id == model)
}

fn provider_is_configured(provider: &CustomProviderConfig) -> bool {
    let url = provider.base_url.trim();
    let key = provider.api_key.trim();
    !url.is_empty()
        && !key.is_empty()
        && (url.starts_with("http://") || url.starts_with("https://"))
}

fn custom_provider_for_selection<'a>(
    settings: &'a AppSettings,
    model: &str,
    provider_hint: &str,
) -> Option<&'a CustomProviderConfig> {
    if !provider_hint.is_empty() {
        if let Some(provider) = settings
            .custom_providers
            .iter()
            .find(|provider| provider.id == provider_hint)
        {
            // Explicit provider selection: allow any model once the endpoint is configured.
            if provider_is_configured(provider) || provider_has_model(provider, model) {
                return Some(provider);
            }
        }
        return None;
    }

    settings
        .custom_providers
        .iter()
        .find(|provider| provider_has_model(provider, model))
}

pub(crate) fn looks_like_deepseek_model(model: &str) -> bool {
    model.trim().to_ascii_lowercase().contains("deepseek")
}

/// Whether this model can be POSTed to a dedicated endpoint (custom provider,
/// Gemini, or DeepSeek). MiniMax on an aggregator is served via Anthropic
/// Messages, not by falling through to official DeepSeek.
pub(crate) fn can_serve_chat_model(
    settings: &AppSettings,
    model: &str,
    provider_hint: &str,
) -> bool {
    let model = model.trim();
    let hint = provider_hint.trim();
    if (hint.is_empty() || hint == "gemini")
        && gemini_oauth::is_gemini_model(model)
        && settings.gemini_oauth.is_logged_in()
    {
        return true;
    }
    if custom_provider_for_selection(settings, model, hint).is_some() {
        return true;
    }
    looks_like_deepseek_model(model)
}

/// Drop collaboration entries the current endpoints cannot actually host.
pub(crate) fn filter_servable_collaboration_models(
    settings: &AppSettings,
    allowed: &[String],
) -> Vec<String> {
    allowed
        .iter()
        .filter(|entry| {
            let parsed = super::model_ref::parse_model_ref(entry);
            !parsed.id.is_empty() && can_serve_chat_model(settings, &parsed.id, &parsed.provider)
        })
        .cloned()
        .collect()
}

pub(crate) fn cached_model_protocol(
    settings: &AppSettings,
    provider_id: &str,
    model: &str,
) -> Option<ModelWireProtocol> {
    let model = model.trim();
    let provider_id = provider_id.trim();
    if provider_id.is_empty() || model.is_empty() {
        return None;
    }
    settings
        .custom_providers
        .iter()
        .find(|provider| provider.id == provider_id)?
        .model_protocols
        .get(model)
        .copied()
}

pub(crate) fn remember_model_protocol(
    app: &AppHandle,
    provider_id: &str,
    model: &str,
    protocol: ModelWireProtocol,
) {
    let provider_id = provider_id.trim();
    let model = model.trim();
    if provider_id.is_empty() || model.is_empty() {
        return;
    }
    let Ok(mut settings) = settings_store::get_settings(app) else {
        return;
    };
    let Some(provider) = settings
        .custom_providers
        .iter_mut()
        .find(|provider| provider.id == provider_id)
    else {
        return;
    };
    if provider.model_protocols.get(model) == Some(&protocol) {
        return;
    }
    provider
        .model_protocols
        .insert(model.to_string(), protocol);
    let _ = settings_store::set_settings(app, settings);
}

/// Resolve the provider by an explicit model + provider-hint selection.
/// Used for per-conversation model overrides; empty hint resolves by model match.
pub(crate) fn resolve_provider_for_selection(
    app: AppHandle,
    model: String,
    provider_hint: String,
) -> Arc<dyn AIProvider> {
    let settings = settings_store::get_settings(&app).unwrap_or_default();
    let model = model.trim().to_string();
    let provider_hint = provider_hint.trim().to_string();

    if (provider_hint.is_empty() || provider_hint == "gemini")
        && gemini_oauth::is_gemini_model(&model)
        && settings.gemini_oauth.is_logged_in()
    {
        return Arc::new(AntigravityProvider::for_model(app, model));
    }
    let resolve_api_key = {
        let app = app.clone();
        let selected_model = model.clone();
        let selected_provider = provider_hint.clone();
        Arc::new(move || {
            let settings = settings_store::get_settings(&app).unwrap_or_default();
            custom_provider_for_selection(&settings, &selected_model, &selected_provider)
                .map(|custom| custom.api_key.clone())
                .unwrap_or(settings.deepseek_api_key)
        })
    };

    let resolve_model = {
        let model = model.clone();
        Arc::new(move || model.clone())
    };

    let resolve_effort = {
        let app = app.clone();
        Arc::new(move || {
            settings_store::get_settings(&app)
                .map(|settings| settings.reasoning_effort)
                .unwrap_or(ReasoningEffort::Disabled)
        })
    };

    let resolve_pass_tool_reasoning = {
        let app = app.clone();
        Arc::new(move || {
            settings_store::get_settings(&app)
                .map(|settings| settings.pass_tool_reasoning)
                .unwrap_or(true)
        })
    };

    let resolve_continue_thinking_after_tools = {
        let app = app.clone();
        Arc::new(move || {
            settings_store::get_settings(&app)
                .map(|settings| settings.continue_thinking_after_tools)
                .unwrap_or(true)
        })
    };

    let resolve_base_url = {
        let app = app.clone();
        let selected_model = model.clone();
        let selected_provider = provider_hint.clone();
        Arc::new(move || -> Option<String> {
            let settings = settings_store::get_settings(&app).unwrap_or_default();
            custom_provider_for_selection(&settings, &selected_model, &selected_provider).and_then(
                |custom| {
                    let base_url = custom.base_url.trim();
                    (!base_url.is_empty()).then(|| base_url.to_string())
                },
            )
        })
    };

    let resolve_api_protocol = {
        let app = app.clone();
        let selected_model = model.clone();
        let selected_provider = provider_hint.clone();
        Arc::new(move || -> ProviderApiProtocol {
            let settings = settings_store::get_settings(&app).unwrap_or_default();
            custom_provider_for_selection(&settings, &selected_model, &selected_provider)
                .map(|custom| custom.api_protocol)
                .unwrap_or_default()
        })
    };

    let resolve_blocked_reason = {
        let app = app.clone();
        let selected_model = model;
        let selected_provider = provider_hint.clone();
        Arc::new(move || -> Option<String> {
            let settings = settings_store::get_settings(&app).unwrap_or_default();
            let provider = if selected_provider.is_empty() {
                custom_provider_for_selection(&settings, &selected_model, "")
            } else {
                settings
                    .custom_providers
                    .iter()
                    .find(|provider| provider.id == selected_provider)
            };
            let disabled = provider
                .map(|provider| provider_model_is_disabled(provider, &selected_model))
                .unwrap_or(false);
            disabled.then(|| {
                format!(
                    "模型 “{selected_model}” 已在设置中被禁用，请先在供应商设置里重新启用后再使用。"
                )
            })
        })
    };

    Arc::new(DeepSeekProvider::new(
        app.clone(),
        provider_hint.clone(),
        resolve_api_key,
        resolve_model,
        resolve_effort,
        resolve_pass_tool_reasoning,
        resolve_continue_thinking_after_tools,
        Some(resolve_base_url),
        resolve_api_protocol,
        resolve_blocked_reason,
    ))
}

#[cfg(test)]
mod tests {
    use super::{custom_provider_for_selection, provider_model_is_disabled};
    use crate::models::settings::{AppSettings, CustomProviderConfig};

    fn provider(id: &str, api_key: &str) -> CustomProviderConfig {
        CustomProviderConfig {
            id: id.into(),
            name: id.into(),
            base_url: format!("https://{id}.example/v1"),
            api_key: api_key.into(),
            models: "shared-model".into(),
            disabled_models: String::new(),
            preset_id: None,
            api_protocol: Default::default(),
            model_protocols: Default::default(),
        }
    }

    #[test]
    fn provider_hint_disambiguates_duplicate_chat_model_ids() {
        let settings = AppSettings {
            custom_providers: vec![provider("first", "key-1"), provider("second", "key-2")],
            ..Default::default()
        };

        let selected = custom_provider_for_selection(&settings, "shared-model", "second")
            .expect("second provider should match");
        assert_eq!(selected.api_key, "key-2");
        assert_eq!(selected.base_url, "https://second.example/v1");
    }

    #[test]
    fn empty_provider_hint_keeps_legacy_first_model_match() {
        let settings = AppSettings {
            custom_providers: vec![provider("first", "key-1"), provider("second", "key-2")],
            ..Default::default()
        };

        let selected = custom_provider_for_selection(&settings, "shared-model", "")
            .expect("legacy model lookup should match");
        assert_eq!(selected.id, "first");
    }

    #[test]
    fn disabled_model_is_flagged_and_survives_comma_or_newline_lists() {
        let mut p = provider("first", "key-1");
        p.disabled_models = "shared-model\nother-model".into();
        assert!(provider_model_is_disabled(&p, "shared-model"));
        assert!(provider_model_is_disabled(&p, "other-model"));
        assert!(!provider_model_is_disabled(&p, "enabled-model"));

        p.disabled_models = "a, b ,c".into();
        assert!(provider_model_is_disabled(&p, "b"));
        assert!(!provider_model_is_disabled(&p, "d"));
    }

    #[test]
    fn mimo_does_not_fall_through_to_deepseek_without_a_custom_provider() {
        let settings = AppSettings::default();
        assert!(!super::can_serve_chat_model(
            &settings,
            "mimo-v2-omni",
            "deepseek"
        ));
        assert!(super::can_serve_chat_model(
            &settings,
            "deepseek-v4-pro",
            "deepseek"
        ));

        let mut hosted = provider("console-go", "key");
        hosted.models = "mimo-v2-omni,deepseek-v4-flash".into();
        let settings = AppSettings {
            custom_providers: vec![hosted],
            ..Default::default()
        };
        assert!(super::can_serve_chat_model(
            &settings,
            "mimo-v2-omni",
            "console-go"
        ));
        assert!(super::can_serve_chat_model(
            &settings,
            "deepseek-v4-flash",
            "console-go"
        ));
    }

    #[test]
    fn minimax_on_custom_aggregator_is_served() {
        let mut hosted = provider("console-go", "key");
        hosted.name = "Console Go".into();
        hosted.models = "deepseek-v4-pro,minimax-m3".into();
        let settings = AppSettings {
            custom_providers: vec![hosted],
            ..Default::default()
        };
        assert!(super::can_serve_chat_model(
            &settings,
            "minimax-m3",
            "console-go"
        ));
        assert!(super::can_serve_chat_model(
            &settings,
            "deepseek-v4-pro",
            "console-go"
        ));

        let allowed = vec![
            r#"["console-go","deepseek-v4-pro"]"#.into(),
            r#"["console-go","minimax-m3"]"#.into(),
        ];
        let filtered = super::filter_servable_collaboration_models(&settings, &allowed);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn dedicated_minimax_openai_endpoint_is_still_served() {
        let mut hosted = provider("minimax", "key");
        hosted.name = "MiniMax".into();
        hosted.models = "minimax-m3".into();
        let settings = AppSettings {
            custom_providers: vec![hosted],
            ..Default::default()
        };
        assert!(super::can_serve_chat_model(
            &settings,
            "minimax-m3",
            "minimax"
        ));
    }

    #[test]
    fn cached_model_protocol_reads_learned_map() {
        let mut hosted = provider("console-go", "key");
        hosted.models = "minimax-m3,deepseek-v4-pro".into();
        hosted.model_protocols.insert(
            "minimax-m3".into(),
            crate::models::settings::ModelWireProtocol::AnthropicMessages,
        );
        let settings = AppSettings {
            custom_providers: vec![hosted],
            ..Default::default()
        };
        assert_eq!(
            super::cached_model_protocol(&settings, "console-go", "minimax-m3"),
            Some(crate::models::settings::ModelWireProtocol::AnthropicMessages)
        );
        assert_eq!(
            super::cached_model_protocol(&settings, "console-go", "deepseek-v4-pro"),
            None
        );
        assert_eq!(
            super::cached_model_protocol(&settings, "", "unknown"),
            None
        );
    }
}
