use tauri::AppHandle;

use crate::models::settings::{AppSettings, AppSettingsPatch};
use crate::services::settings_store::{get_settings, patch_settings};

#[cfg(windows)]
fn installed_font_families() -> Result<Vec<String>, String> {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::Graphics::Gdi::{
        EnumFontFamiliesExW, GetDC, ReleaseDC, DEFAULT_CHARSET, LOGFONTW, TEXTMETRICW,
    };

    unsafe extern "system" fn collect_font(
        font: *const LOGFONTW,
        _metric: *const TEXTMETRICW,
        _font_type: u32,
        data: LPARAM,
    ) -> i32 {
        let families = &mut *(data.0 as *mut Vec<String>);
        let face = &(*font).lfFaceName;
        let length = face
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(face.len());
        let name = String::from_utf16_lossy(&face[..length]);
        if !name.is_empty() && !name.starts_with('@') {
            families.push(name);
        }
        1
    }

    let desktop = HWND::default();
    let dc = unsafe { GetDC(desktop) };
    if dc.is_invalid() {
        return Err("Could not access the system font list".into());
    }

    let mut request = LOGFONTW::default();
    request.lfCharSet = DEFAULT_CHARSET;
    let mut families: Vec<String> = Vec::new();
    unsafe {
        EnumFontFamiliesExW(
            dc,
            &request,
            Some(collect_font),
            LPARAM((&mut families as *mut Vec<String>) as isize),
            0,
        );
        ReleaseDC(desktop, dc);
    }
    families.sort_by_key(|name| name.to_lowercase());
    families.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    Ok(families)
}

#[cfg(not(windows))]
fn installed_font_families() -> Result<Vec<String>, String> {
    let output = std::process::Command::new("fc-list")
        .args(["-f", "%{family}\n"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("Could not access the system font list".into());
    }
    let mut families: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .flat_map(|line| line.split(','))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect();
    families.sort_by_key(|name| name.to_lowercase());
    families.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    Ok(families)
}

#[tauri::command]
pub fn list_system_fonts() -> Result<Vec<String>, String> {
    installed_font_families()
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn lists_installed_fonts() {
        let families = super::installed_font_families().expect("Windows font enumeration succeeds");
        assert!(
            !families.is_empty(),
            "Windows font enumeration returned no fonts"
        );
    }
}

#[tauri::command]
pub async fn get_app_settings(app: AppHandle) -> Result<AppSettings, String> {
    // The first call marks the far end of the startup blind spot: the webview
    // finished its own eval and reached the host.
    crate::boot_timing::phase_once("first settings IPC from webview");
    // The settings mutex may be held by a durable save. Never wait for it on
    // the webview event thread, including reads arriving during a picker change.
    tauri::async_runtime::spawn_blocking(move || get_settings(&app))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn set_app_settings(
    app: AppHandle,
    patch: AppSettingsPatch,
) -> Result<AppSettings, String> {
    // Backup + atomic replacement includes sync_all / MOVEFILE_WRITE_THROUGH.
    // Keep durability and transaction ordering without blocking window events.
    tauri::async_runtime::spawn_blocking(move || patch_settings(&app, patch))
        .await
        .map_err(|error| error.to_string())?
}
