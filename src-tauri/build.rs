use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    if let Ok(triple) = std::env::var("TARGET") {
        println!("cargo:rustc-env=TARGET_TRIPLE={triple}");
    }
    ensure_windows_rc_tool();
    configure_debug_identity();
    configure_updater_pubkey();
    tauri_build::build();
    // Prefer Common Controls 6.0 (TaskDialogIndirect / SetWindowSubclass).
    // Use MANIFESTDEPENDENCY instead of embedding a second RT_MANIFEST — tauri_build
    // already embeds one into the app binary, and a duplicate causes CVT1100 / LNK1123.
    // This also covers the `cargo test --lib` harness, which otherwise has no manifest.
    println!(
        "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' \
         name='Microsoft.Windows.Common-Controls' \
         version='6.0.0.0' \
         processorArchitecture='*' \
         publicKeyToken='6595b64144ccf1df' \
         language='*'"
    );
}

fn ensure_windows_rc_tool() {
    if !cfg!(target_os = "windows") || std::env::var_os("RC").is_some() {
        return;
    }

    let host = std::env::var("HOST").unwrap_or_default();
    let arch_dir = if host.starts_with("aarch64") {
        "arm64"
    } else if host.starts_with("x86_64") {
        "x64"
    } else {
        "x86"
    };

    let mut kit_roots = Vec::<PathBuf>::new();
    if let Some(pf86) = std::env::var_os("ProgramFiles(x86)") {
        kit_roots.push(PathBuf::from(pf86).join("Windows Kits").join("10"));
    }
    if let Some(pf) = std::env::var_os("ProgramFiles") {
        kit_roots.push(PathBuf::from(pf).join("Windows Kits").join("10"));
    }
    kit_roots.push(PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10"));

    for kit_root in kit_roots {
        let bin_root = kit_root.join("bin");
        let Ok(entries) = fs::read_dir(&bin_root) else {
            continue;
        };
        let mut versions: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_dir())
            .collect();
        versions.sort();
        versions.reverse();

        for ver_dir in versions {
            let rc_path = ver_dir.join(arch_dir).join("rc.exe");
            if rc_path.is_file() {
                std::env::set_var("RC", &rc_path);
                append_windows_kit_includes(&kit_root);
                return;
            }
        }
    }
}

fn append_windows_kit_includes(kit_root: &Path) {
    let include_root = kit_root.join("Include");
    let Ok(entries) = fs::read_dir(&include_root) else {
        return;
    };
    let mut versions: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    versions.sort();
    versions.reverse();

    let Some(latest_ver) = versions.into_iter().next() else {
        return;
    };
    let Ok(sub_dirs) = fs::read_dir(&latest_ver) else {
        return;
    };

    let mut include = std::env::var("INCLUDE").unwrap_or_default();
    if !include.is_empty() && !include.ends_with(';') {
        include.push(';');
    }
    for sub in sub_dirs.filter_map(|e| e.ok().map(|e| e.path())) {
        if sub.is_dir() {
            if let Some(s) = sub.to_str() {
                if !include.contains(s) {
                    include.push_str(s);
                    include.push(';');
                }
            }
        }
    }
    std::env::set_var("INCLUDE", include);
}

fn configure_updater_pubkey() {
    println!("cargo:rerun-if-env-changed=ANYA_UPDATER_PUBKEY");
    println!("cargo:rerun-if-env-changed=AAAI_UPDATER_PUBKEY");
    println!("cargo:rerun-if-changed=updater.pubkey");

    let pubkey = std::env::var("ANYA_UPDATER_PUBKEY")
        .or_else(|_| std::env::var("AAAI_UPDATER_PUBKEY"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(read_updater_pubkey_file);

    let Some(pubkey) = pubkey else {
        println!(
            "cargo:warning=Updater public key not configured; set ANYA_UPDATER_PUBKEY or create src-tauri/updater.pubkey"
        );
        return;
    };

    let config_from_env = std::env::var("TAURI_CONFIG")
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
    let mut config = config_from_env.unwrap_or_else(|| {
        let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory");
        let raw = fs::read_to_string(Path::new(&manifest_dir).join("tauri.conf.json"))
            .expect("read tauri.conf.json");
        serde_json::from_str(&raw).expect("parse tauri.conf.json")
    });

    if !config
        .get("plugins")
        .is_some_and(serde_json::Value::is_object)
    {
        config["plugins"] = serde_json::json!({});
    }
    if !config
        .pointer("/plugins/updater")
        .is_some_and(serde_json::Value::is_object)
    {
        config["plugins"]["updater"] = serde_json::json!({});
    }
    config["plugins"]["updater"]["pubkey"] = serde_json::Value::String(pubkey);

    let serialized = serde_json::to_string(&config).expect("serialize updater Tauri config");
    println!("cargo:rustc-env=TAURI_CONFIG={serialized}");
    std::env::set_var("TAURI_CONFIG", serialized);
}

fn read_updater_pubkey_file() -> Option<String> {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")?;
    let path = Path::new(&manifest_dir).join("updater.pubkey");
    let raw = fs::read_to_string(path).ok()?;
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn configure_debug_identity() {
    println!("cargo:rerun-if-env-changed=DEBUG");
    if std::env::var("DEBUG").as_deref() != Ok("true") {
        return;
    }

    let config_from_env = std::env::var("TAURI_CONFIG")
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
    let mut config = config_from_env.unwrap_or_else(|| {
        let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory");
        let raw = fs::read_to_string(Path::new(&manifest_dir).join("tauri.conf.json"))
            .expect("read tauri.conf.json");
        serde_json::from_str(&raw).expect("parse tauri.conf.json")
    });

    config["productName"] = serde_json::Value::String("Anya Debug".into());
    config["identifier"] = serde_json::Value::String("ai.anya.desktop.debug".into());
    if let Some(windows) = config
        .get_mut("app")
        .and_then(|app| app.get_mut("windows"))
        .and_then(serde_json::Value::as_array_mut)
    {
        for window in windows {
            window["title"] = serde_json::Value::String("Anya Debug".into());
        }
    }

    let serialized = serde_json::to_string(&config).expect("serialize debug Tauri config");
    // tauri_build reads the build-script environment, while generate_context!
    // runs later inside rustc. Export the same config to both so runtime identity
    // (including the single-instance mutex) matches the Windows metadata.
    println!("cargo:rustc-env=TAURI_CONFIG={serialized}");
    std::env::set_var("TAURI_CONFIG", serialized);
}
