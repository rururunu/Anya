use std::fs;
use std::path::{Path, PathBuf};

fn warn_if_docx_vendor_missing() {
    let manifest_dir = match std::env::var_os("CARGO_MANIFEST_DIR") {
        Some(value) => PathBuf::from(value),
        None => return,
    };
    let marker = manifest_dir.join("prompts/skills/vendor/docx/scripts/merge_runs.py");
    println!("cargo:rerun-if-changed=prompts/skills/vendor/docx");
    if !marker.is_file() {
        println!(
            "cargo:warning=docx skill vendor missing; run `pnpm sync-skills` before using #skill:docx"
        );
    }
}

fn main() {
    warn_if_docx_vendor_missing();
    if let Ok(triple) = std::env::var("TARGET") {
        println!("cargo:rustc-env=TARGET_TRIPLE={triple}");
    }
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
