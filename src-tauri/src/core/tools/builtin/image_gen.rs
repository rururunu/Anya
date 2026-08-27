//! Generate images via `POST /v1/images/generations` (`gpt-image-2`).

use std::path::PathBuf;

use serde_json::{json, Value};
use tauri::Manager;

use crate::core::ai::image_gen::{
    decode_image_source, generate_images_blocking, normalize_count, normalize_quality,
    normalize_size, ImageGenRequest, DEFAULT_IMAGE_MODEL,
};
use crate::core::chat::limits::truncate_chars;
use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::services::settings_store;

pub(super) struct GenerateImageTool;

impl Tool for GenerateImageTool {
    fn name(&self) -> &str {
        "generate_image"
    }

    fn description(&self) -> &str {
        "Generate an image from a text prompt using the configured image model (default gpt-image-2). \
When a reference image is attached (composer image or a style template example), this becomes image-to-image via Images edits. \
Saves PNG files locally and returns their paths plus markdown the chat UI can render. \
`n` greater than 1 issues that many separate image requests (hosts typically allow only one image per call). \
Use when the user asks to draw, illustrate, design, or generate a picture. \
Requires a provider configured under Settings → Image whose Base URL is the Images API (official example: https://api.openai.com/v1), not a chat provider."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Detailed description of the image to generate. Include subject, composition, style, lighting, and any on-image text."
                },
                "size": {
                    "type": "string",
                    "description": "Width x height, e.g. 1024x1024, 1024x1536, 1536x1024, or auto. Default 1024x1024."
                },
                "quality": {
                    "type": "string",
                    "enum": ["auto", "low", "medium", "high"],
                    "description": "Rendering quality. Default auto."
                },
                "n": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 4,
                    "description": "How many images to generate (1–4). Default 1."
                },
                "image": {
                    "type": "string",
                    "description": "Optional local path or path: URI of a reference image for image-to-image. Image mode already supplies composer attachments and style-template examples; only set this to override."
                }
            },
            "required": ["prompt"]
        })
    }

    fn read_only(&self) -> bool {
        false
    }

    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        ctx.ensure_not_cancelled()?;
        let mut prompt = args["prompt"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ToolError::new("prompt is required"))?
            .to_string();
        let mut size = args["size"].as_str().unwrap_or("1024x1024").to_string();
        let mut quality = args["quality"].as_str().unwrap_or("auto").to_string();
        let mut n = args["n"]
            .as_u64()
            .or_else(|| args["n"].as_i64().map(|v| v.max(0) as u64))
            .unwrap_or(1);
        let mut sources: Vec<String> = Vec::new();
        if let Some(path) = args["image"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            sources.push(path.to_string());
        }

        if let Some(options) =
            crate::core::tools::image_mode::image_mode_options(ctx.root_session_id())
        {
            size = options.size;
            quality = options.quality;
            n = options.n as u64;
            let style = options.style_prompt.trim();
            if !style.is_empty()
                && !prompt
                    .to_ascii_lowercase()
                    .contains(&style.to_ascii_lowercase())
            {
                prompt = format!(
                    "{style}\n\nSubject and scene (keep this content; do not override the style above):\n{prompt}"
                );
            }
            for src in options.reference_images {
                if !sources.iter().any(|item| item == &src) {
                    sources.push(src);
                }
            }
        }
        sources.truncate(4);

        let mut reference_images = Vec::new();
        for src in &sources {
            match decode_image_source(src) {
                Ok(bytes) => reference_images.push(bytes),
                Err(error) => tracing::warn!(error = %error, "skipping reference image"),
            }
        }
        if !sources.is_empty() && reference_images.is_empty() {
            return Err(ToolError::new(
                "could not read any reference image for image-to-image",
            ));
        }

        let app = ctx
            .app_handle
            .clone()
            .ok_or_else(|| ToolError::new("app handle unavailable"))?;
        let settings = settings_store::get_settings(&app).map_err(|error| ToolError::new(error))?;
        let model = if settings.image_model.trim().is_empty() {
            DEFAULT_IMAGE_MODEL.to_string()
        } else {
            settings.image_model.trim().to_string()
        };
        let request = ImageGenRequest {
            prompt: prompt.clone(),
            model: model.clone(),
            size: normalize_size(&size),
            quality: normalize_quality(&quality),
            n: normalize_count(n),
            reference_images,
        };
        let reference_count = request.reference_images.len();
        ctx.ensure_not_cancelled()?;

        let settings_for_http = settings.clone();
        let images = crate::runtime::isolated::run_isolated(move || {
            generate_images_blocking(&settings_for_http, &request)
        })
        .map_err(ToolError::new)?;
        ctx.ensure_not_cancelled()?;
        if images.is_empty() {
            return Err(ToolError::new("image provider returned no images"));
        }

        let durable_dir = durable_output_dir(&app)?;
        std::fs::create_dir_all(&durable_dir).map_err(|e| ToolError::new(e.to_string()))?;
        let workspace_dir = workspace_output_dir(ctx);

        let size_norm = normalize_size(&size);
        let quality_norm = normalize_quality(&quality);
        let mut lines = vec![format!(
            "Generated {} image{} with {model} ({}, {}){}.",
            images.len(),
            if images.len() == 1 { "" } else { "s" },
            size_norm,
            quality_norm,
            if reference_count == 0 {
                String::new()
            } else {
                format!(
                    " from {reference_count} reference image{}",
                    if reference_count == 1 { "" } else { "s" }
                )
            }
        )];
        let mut structured_images = Vec::with_capacity(images.len());
        for (index, image) in images.iter().enumerate() {
            let stem = unique_stem(index);
            let file_name = format!("{stem}.{}", image.extension);
            let durable_path = durable_dir.join(&file_name);
            std::fs::write(&durable_path, &image.bytes)
                .map_err(|e| ToolError::new(format!("failed to save image: {e}")))?;
            let display = durable_path.display().to_string();
            if let Some(workspace_dir) = workspace_dir.as_ref() {
                if let Err(error) = std::fs::create_dir_all(workspace_dir)
                    .and_then(|_| std::fs::copy(&durable_path, workspace_dir.join(&file_name)))
                {
                    tracing::warn!("could not copy generated image into workspace: {error}");
                } else {
                    lines.push(format!(
                        "Workspace copy: {}",
                        workspace_dir.join(&file_name).display()
                    ));
                }
            }
            let alt = image
                .revised_prompt
                .as_deref()
                .unwrap_or(&prompt)
                .replace(['\n', '\r'], " ");
            let alt = truncate_chars(&alt, 120);
            lines.push(format!("Saved: {display}"));
            lines.push(format!("![{alt}](path:{display})"));
            let revised = image
                .revised_prompt
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty() && *value != prompt.trim())
                .map(|value| value.to_string());
            if let Some(revised) = revised.as_deref() {
                lines.push(format!("Revised prompt: {}", truncate_chars(revised, 400)));
            }
            structured_images.push(json!({
                "path": display,
                "revised_prompt": revised,
            }));
        }
        lines.push(
            "Show the user the markdown image above. Do not inline base64. The local path can be used with other file tools."
                .into(),
        );
        // Machine-readable trailer for the UI; markdown above stays for the model.
        let structured = json!({
            "version": 1,
            "model": model,
            "size": size_norm,
            "quality": quality_norm,
            "images": structured_images,
        });
        lines.push(format!(
            "```anya-images\n{}\n```",
            serde_json::to_string(&structured).unwrap_or_else(|_| "{\"version\":1,\"images\":[]}".into())
        ));
        Ok(lines.join("\n"))
    }
}

fn durable_output_dir(app: &tauri::AppHandle) -> Result<PathBuf, ToolError> {
    let root = app
        .path()
        .app_data_dir()
        .map_err(|e| ToolError::new(format!("app data dir unavailable: {e}")))?;
    Ok(root.join("generated"))
}

fn workspace_output_dir(ctx: &ToolContext) -> Option<PathBuf> {
    let root = &ctx.workspace_root;
    if root.as_os_str().is_empty() || !root.is_dir() {
        return None;
    }
    Some(root.join(".anya").join("generated"))
}

fn unique_stem(index: usize) -> String {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let id = uuid::Uuid::new_v4().simple().to_string();
    format!("{stamp}-{}-{index}", &id[..8])
}
