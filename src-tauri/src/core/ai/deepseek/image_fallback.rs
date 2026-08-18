use tokio::sync::mpsc::Sender;

use crate::core::runtime::{ChatRequest, Role, StreamEvent};
use crate::models::settings::ProviderApiProtocol;

use super::models::endpoint_url_for_protocol;
use super::multimodal::{describe_image, multimodal_http_client, resolve_multimodal_endpoint};
use super::ProviderError;

pub(super) enum FallbackPlan {
    RetryPrimary,
    SwitchToMultimodal {
        model: String,
        api_key: String,
        url: String,
        protocol: ProviderApiProtocol,
    },
}

pub(super) async fn apply_image_input_fallback(
    request: &mut ChatRequest,
    settings: &crate::models::settings::AppSettings,
    app: &tauri::AppHandle,
    tx: &Sender<StreamEvent>,
) -> Result<FallbackPlan, ProviderError> {
    let mm_model = settings.multimodal_model.trim();

    if !mm_model.is_empty()
        && (settings.multimodal_model_provider.trim().is_empty()
            || settings.multimodal_model_provider.trim() == "gemini")
        && crate::services::gemini_oauth::can_use_antigravity_for_model(settings, mm_model)
        && !settings.multimodal_split_analysis
    {
        apply_split_image_analysis(request, settings, app, tx).await?;
        return Ok(FallbackPlan::RetryPrimary);
    }

    if settings.multimodal_split_analysis {
        apply_split_image_analysis(request, settings, app, tx).await?;
        return Ok(FallbackPlan::RetryPrimary);
    }

    if mm_model.is_empty() {
        return Err(ProviderError::message(
            "The primary model does not support image input. Configure a multimodal model in Settings, or enable multimodal split analysis.",
        ));
    }

    if (settings.multimodal_model_provider.trim().is_empty()
        || settings.multimodal_model_provider.trim() == "gemini")
        && crate::services::gemini_oauth::can_use_antigravity_for_model(settings, mm_model)
    {
        return Err(ProviderError::message(
            "Gemini multimodal models cannot be switched wholesale to the OpenAI API. Enable multimodal split analysis, or use a model such as gpt-4o.",
        ));
    }

    let endpoint = resolve_multimodal_endpoint(
        settings,
        mm_model,
        settings.multimodal_model_provider.trim(),
    )?;
    Ok(FallbackPlan::SwitchToMultimodal {
        model: mm_model.to_string(),
        api_key: endpoint.api_key,
        url: endpoint_url_for_protocol(&endpoint.base_url, endpoint.protocol),
        protocol: endpoint.protocol,
    })
}

async fn apply_split_image_analysis(
    request: &mut ChatRequest,
    settings: &crate::models::settings::AppSettings,
    app: &tauri::AppHandle,
    tx: &Sender<StreamEvent>,
) -> Result<(), ProviderError> {
    let client = multimodal_http_client();
    let mm_model = if settings.multimodal_model.trim().is_empty() {
        "gpt-4o".to_string()
    } else {
        settings.multimodal_model.trim().to_string()
    };

    let image_re = match regex::Regex::new(r"!\[image\]\((.*?)\)") {
        Ok(re) => re,
        Err(_) => return Err(ProviderError::message("invalid image regex")),
    };

    let mut any_api_calls = false;
    let mut patches: Vec<(String, String)> = Vec::new();

    for message in &mut request.messages {
        if message.role != Role::User || !message.content.contains("![image](") {
            continue;
        }

        let image_markdowns: Vec<String> = image_re
            .find_iter(&message.content)
            .map(|m| m.as_str().to_string())
            .collect();

        let mut stored = message.content.clone();
        let original = stored.clone();

        for image_markdown in &image_markdowns {
            if crate::core::ai::image_analysis::usable_analysis_after_image(&stored, image_markdown)
                .is_some()
            {
                continue;
            }

            if crate::core::ai::image_analysis::analysis_after_image(&stored, image_markdown)
                .is_some()
            {
                stored = crate::core::ai::image_analysis::remove_analysis_after_image(
                    &stored,
                    image_markdown,
                );
            }

            if !any_api_calls {
                any_api_calls = true;
                let _ = tx
                    .send(StreamEvent::Status {
                        kind: "analyzing_images".to_string(),
                    })
                    .await;
            }

            let image_url = image_re
                .captures(image_markdown)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str())
                .unwrap_or("");

            let text = match describe_image(&client, app, image_url).await {
                Ok(desc) => desc,
                Err(err) => {
                    let _ = tx
                        .send(StreamEvent::Status {
                            kind: String::new(),
                        })
                        .await;
                    return Err(err);
                }
            };

            stored = crate::core::ai::image_analysis::insert_analysis_after_image(
                &stored,
                image_markdown,
                &mm_model,
                &text,
            );
        }

        if stored != original {
            patches.push((message.id.clone(), stored.clone()));
        }
        message.content = stored;
    }

    for (message_id, content) in &patches {
        let _ = tx
            .send(StreamEvent::UserContentPatch {
                message_id: message_id.clone(),
                content: content.clone(),
            })
            .await;
    }

    if any_api_calls {
        let _ = tx
            .send(StreamEvent::Status {
                kind: String::new(),
            })
            .await;
    }

    for message in &mut request.messages {
        if message.role == Role::User
            && (message.content.contains("![image](")
                || message.content.contains("peek-image-analysis"))
        {
            message.content = crate::core::ai::image_analysis::replace_images_with_analysis_text(
                &message.content,
            );
        }
    }

    Ok(())
}
