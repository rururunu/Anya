use std::fs::{self, File};
use std::io;
use std::sync::Mutex;
use tauri::AppHandle;

use crate::core::tools::skills::office_runtime::{bundled_runtime, disabled_marker};

static INSTALL: Mutex<()> = Mutex::new(());

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeRuntimeStatus {
    version: Option<String>,
    enabled: bool,
}

#[tauri::command]
pub fn office_runtime_status(app: AppHandle) -> Result<OfficeRuntimeStatus, String> {
    let version = bundled_runtime(Some(&app))
        .and_then(|directory| fs::read(directory.join("kit/BUILD.json")).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|manifest| manifest["api"].as_str().map(str::to_owned));
    let disabled = disabled_marker(&app)
        .map_err(|error| error.to_string())?
        .exists();
    Ok(OfficeRuntimeStatus {
        version,
        enabled: !disabled,
    })
}

#[tauri::command]
pub async fn set_office_runtime_enabled(
    app: AppHandle,
    enabled: bool,
) -> Result<OfficeRuntimeStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = INSTALL
            .try_lock()
            .map_err(|_| "An Office operation is already running")?;
        let marker = disabled_marker(&app).map_err(|error| error.to_string())?;
        if enabled {
            match fs::remove_file(marker) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
        } else {
            fs::create_dir_all(marker.parent().expect("Office components directory"))
                .and_then(|_| File::create(marker))
                .map_err(|error| error.to_string())?;
        }
        office_runtime_status(app)
    })
    .await
    .map_err(|error| error.to_string())?
}
