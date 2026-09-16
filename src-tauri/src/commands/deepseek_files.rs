use tauri::AppHandle;

use crate::core::ai::deepseek::{
    self, DeepSeekFileDeleteResult, DeepSeekFileList, DeepSeekFileObject,
};
use crate::services::settings_store::get_settings;

fn api_key(app: &AppHandle) -> Result<String, String> {
    let settings = get_settings(app)?;
    let key = settings.deepseek_api_key.trim().to_string();
    if key.is_empty() {
        return Err("DeepSeek API key is not configured".into());
    }
    Ok(key)
}

/// DeepSeek POST /files — upload an image for later `file_id` citation.
#[tauri::command]
pub async fn upload_deepseek_file(
    app: AppHandle,
    path: String,
) -> Result<DeepSeekFileObject, String> {
    let key = api_key(&app)?;
    deepseek::upload_path(&key, &path, None)
        .await
        .map_err(|error| error.to_string())
}

/// DeepSeek GET /files — list uploaded files.
#[tauri::command]
pub async fn list_deepseek_files(
    app: AppHandle,
    after: Option<String>,
    limit: Option<u32>,
) -> Result<DeepSeekFileList, String> {
    let key = api_key(&app)?;
    deepseek::list_files(&key, after.as_deref(), limit, Some("desc"))
        .await
        .map_err(|error| error.to_string())
}

/// DeepSeek GET /files/:file_id — retrieve one file's metadata.
#[tauri::command]
pub async fn retrieve_deepseek_file(
    app: AppHandle,
    file_id: String,
) -> Result<DeepSeekFileObject, String> {
    let key = api_key(&app)?;
    deepseek::retrieve_file(&key, &file_id)
        .await
        .map_err(|error| error.to_string())
}

/// DeepSeek DELETE /files/:file_id.
#[tauri::command]
pub async fn delete_deepseek_file(
    app: AppHandle,
    file_id: String,
) -> Result<DeepSeekFileDeleteResult, String> {
    let key = api_key(&app)?;
    deepseek::delete_file(&key, &file_id)
        .await
        .map_err(|error| error.to_string())
}
