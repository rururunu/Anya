use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewWindow};
const WORKER: &str = "--anya-factory-reset-worker";

fn clear_root(path: &Path, preserve_settings: bool) -> std::io::Result<()> {
    if !preserve_settings {
        return remove_owned_tree(path);
    }
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    #[cfg(windows)]
    let link = {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let link = meta.file_type().is_symlink();
    if link || !meta.is_dir() {
        return remove_owned_tree(path);
    }
    // Keep configuration in place, avoiding a temporary copy of API keys.
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name();
        if matches!(
            name.to_str(),
            Some(
                "settings.json"
                    | ".factory-reset"
                    | "settings.json.bak"
                    | "models"
                    | "plugins"
                    | "skills"
                    | "mcp-auth"
                    | "icons"
                    | "desktop-pet.json"
                    | "workbench-window.json"
                    | "remote_gateway.json"
                    | "remote_devices.json"
            )
        ) {
            continue;
        }
        remove_owned_tree(&entry.path())?;
    }
    Ok(())
}

fn mark_reset(config_root: &Path) -> std::io::Result<()> {
    if let Ok(meta) = std::fs::symlink_metadata(config_root) {
        #[cfg(windows)]
        let link = {
            use std::os::windows::fs::MetadataExt;
            meta.file_attributes() & 0x400 != 0
        };
        #[cfg(not(windows))]
        let link = meta.file_type().is_symlink();
        if link {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Reset marker cannot follow a linked configuration directory",
            ));
        }
    }
    std::fs::create_dir_all(config_root)?;
    std::fs::write(
        config_root.join(super::settings_store::RESET_MARKER),
        b"legacy migration disabled\n",
    )
}

fn data_roots(id: &str) -> Result<Vec<PathBuf>, String> {
    if id.is_empty() || id == "." || id == ".." || id.contains(['/', '\\']) {
        return Err("Invalid application identifier".into());
    }
    let mut roots = Vec::new();
    for base in [
        dirs::config_dir(),
        dirs::data_dir(),
        dirs::data_local_dir(),
        dirs::cache_dir(),
    ] {
        let base = base.ok_or("Application data directory unavailable")?;
        if !base.is_absolute() {
            return Err("Unsafe data directory".into());
        }
        let root = base.join(id);
        if root.parent() != Some(base.as_path())
            || root.file_name() != Some(std::ffi::OsStr::new(id))
        {
            return Err("Unsafe reset path".into());
        }
        if !roots.contains(&root) {
            roots.push(root);
        }
    }
    // Local skills/plugins and memory use a separate application-owned root.
    if let Some(base) = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        roots.push(base.join(if cfg!(debug_assertions) {
            "Anya Debug"
        } else {
            "Anya"
        }));
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join("Library/Logs").join(id));
    }
    Ok(roots)
}
fn remove_owned_tree(path: &Path) -> std::io::Result<()> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(value) => value,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    #[cfg(windows)]
    let link = {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let link = meta.file_type().is_symlink();
    if link {
        if meta.is_dir() {
            std::fs::remove_dir(path)
        } else {
            std::fs::remove_file(path)
        }
    } else if meta.is_dir() {
        for entry in std::fs::read_dir(path)? {
            remove_owned_tree(&entry?.path())?;
        }
        std::fs::remove_dir(path)
    } else {
        std::fs::remove_file(path)
    }
}
#[cfg(windows)]
fn wait_for_exit(pid: u32) -> Result<(), String> {
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_ACCESS_RIGHTS,
    };
    unsafe {
        let handle = match OpenProcess(PROCESS_ACCESS_RIGHTS(0x00100000), false, pid) {
            Ok(h) => h,
            Err(e) if (e.code().0 as u32 & 0xffff) == 87 => return Ok(()),
            Err(e) => return Err(e.to_string()),
        };
        let result = WaitForSingleObject(handle, 60000);
        let _ = CloseHandle(handle);
        if result != WAIT_OBJECT_0 {
            return Err("Application did not exit; reset was aborted".into());
        }
    }
    Ok(())
}
#[cfg(unix)]
fn wait_for_exit(pid: u32) -> Result<(), String> {
    extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }
    for _ in 0..600 {
        if unsafe { kill(pid as i32, 0) } != 0 {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err("Application did not exit; reset was aborted".into())
}
pub fn run_worker() -> bool {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some(WORKER) {
        return false;
    }
    let result = (|| -> Result<(), String> {
        let pid = args
            .get(2)
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|v| *v > 0 && *v != std::process::id())
            .ok_or("Invalid reset parent")?;
        let context: tauri::Context<tauri::Wry> = tauri::generate_context!();
        let roots = data_roots(&context.config().identifier)?;
        let preserve_settings = args.get(3).map(String::as_str) == Some("keep-settings");
        wait_for_exit(pid)?;
        let config_root = dirs::config_dir()
            .ok_or("Configuration directory unavailable")?
            .join(&context.config().identifier);
        for root in roots {
            let mut result = clear_root(&root, preserve_settings);
            for _ in 0..100 {
                if result.is_ok() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
                result = clear_root(&root, preserve_settings);
            }
            if !preserve_settings && root == config_root {
                // Also mark partial failures: startup must never recover old keys after reset.
                mark_reset(&root).map_err(|e| e.to_string())?;
            }
            result.map_err(|e| format!("Cannot clear {}: {e}", root.display()))?;
        }
        if let Some(local) = dirs::data_local_dir().filter(|_| !preserve_settings) {
            for name in [
                "browser-bridge.json",
                "browser-extension-origin.txt",
                "native-messaging.json",
            ] {
                match std::fs::remove_file(local.join("Anya").join(name)) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e.to_string()),
                }
            }
        }
        Ok(())
    })();
    if let Ok(exe) = std::env::current_exe() {
        let mut command = Command::new(exe);
        if let Err(error) = result {
            command.arg("--anya-factory-reset-failed").arg(error);
        }
        let _ = command.spawn();
    }
    true
}
#[tauri::command]
pub fn delete_all_user_information(
    app: AppHandle,
    window: WebviewWindow,
    confirmation: String,
    preserve_settings: bool,
) -> Result<(), String> {
    if window.label() != "workbench" || confirmation != "DELETE_ALL_USER_INFORMATION" {
        return Err("Explicit workbench confirmation is required".into());
    }
    data_roots(&app.config().identifier)?;
    let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    command.arg(WORKER).arg(std::process::id().to_string());
    command.arg(if preserve_settings {
        "keep-settings"
    } else {
        "reset-all"
    });
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command.spawn().map_err(|e| e.to_string())?;
    if let Some(state) = app.try_state::<crate::app_state::AppState>() {
        state.core.chat().cancel_all_active();
    }
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(1000));
        app.exit(0);
    });
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keeps_keys_providers_and_settings_while_removing_user_information() {
        let root = std::env::temp_dir().join(format!("anya-reset-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("EBWebView/Local Storage")).unwrap();
        let settings =
            r#"{"apiKey":"secret","customProviders":[{"id":"custom"}],"colorScheme":"dark"}"#;
        std::fs::write(root.join("settings.json"), settings).unwrap();
        std::fs::write(root.join("settings.json.bak"), settings).unwrap();
        for name in [
            "history.db",
            "history.db-wal",
            "history.db-shm",
            "memories.json",
        ] {
            std::fs::write(root.join(name), "data").unwrap();
        }
        clear_root(&root, true).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("settings.json")).unwrap(),
            settings
        );
        assert!(root.join("settings.json.bak").exists());
        assert!(!root.join("history.db").exists());
        assert!(!root.join("history.db-wal").exists());
        assert!(!root.join("memories.json").exists());
        assert!(!root.join("EBWebView").exists());
        clear_root(&root, false).unwrap();
        assert!(!root.exists());
    }
    #[test]
    fn removes_database_sidecars_and_settings_without_touching_other_paths() {
        let parent = std::env::temp_dir().join(format!("anya-reset-test-{}", uuid::Uuid::new_v4()));
        let root = parent.join("owned");
        std::fs::create_dir_all(root.join("WebView/Local Storage")).unwrap();
        std::fs::write(parent.join("keep.txt"), "keep").unwrap();
        for name in [
            "history.db",
            "history.db-wal",
            "history.db-shm",
            "settings.json",
            "settings.json.bak",
        ] {
            std::fs::write(root.join(name), "data").unwrap();
        }
        remove_owned_tree(&root).unwrap();
        assert!(!root.exists());
        assert!(parent.join("keep.txt").exists());
        remove_owned_tree(&parent).unwrap();
    }
    #[test]
    fn rejects_traversal_identifiers() {
        assert!(data_roots("../outside").is_err());
        assert!(data_roots("..").is_err());
    }

    #[test]
    fn information_only_cleanup_retains_reset_marker() {
        let root = std::env::temp_dir().join(format!("anya-reset-marker-{}", uuid::Uuid::new_v4()));
        mark_reset(&root).unwrap();
        std::fs::write(root.join("history.db"), "data").unwrap();
        clear_root(&root, true).unwrap();
        assert!(root
            .join(super::super::settings_store::RESET_MARKER)
            .exists());
        assert!(!root.join("history.db").exists());
        remove_owned_tree(&root).unwrap();
    }
}
