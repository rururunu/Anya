use std::fs;
use std::path::{Path, PathBuf};

const OAUTH_LOCAL_FILE_NAMES: &[&str] = &[
    "agy-oauth.local.json",
    "google-oauth.local.json",
    "client_secret.local.json",
];
const EMBEDDED_OAUTH_FILE_NAME: &str = "agy-oauth-credentials.bin";
const EMBEDDED_OAUTH_MAGIC: &[u8] = b"ANYA-OAUTH-1";
const EMBEDDED_OAUTH_KEY: &[u8] = b"Anya-build-credential";

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
    embed_oauth_credentials();
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

fn embed_oauth_credentials() {
    println!("cargo:rerun-if-env-changed=ANYA_AGY_OAUTH_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=ANYA_AGY_OAUTH_CLIENT_SECRET");
    println!("cargo:rerun-if-env-changed=AGY_OAUTH_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=AGY_OAUTH_CLIENT_SECRET");
    for name in OAUTH_LOCAL_FILE_NAMES {
        println!("cargo:rerun-if-changed={name}");
    }

    let credentials = credentials_from_environment().or_else(credentials_from_local_files);
    write_embedded_oauth_file(credentials.as_ref()).expect("write embedded OAuth credentials");
    if credentials.is_none() {
        println!(
            "cargo:warning=Antigravity OAuth credentials were not embedded; packaged Google sign-in will require runtime configuration"
        );
    }
}

fn write_embedded_oauth_file(credentials: Option<&(String, String)>) -> std::io::Result<()> {
    let out_dir = std::env::var_os("OUT_DIR").expect("OUT_DIR is set for build scripts");
    let mut payload = Vec::new();
    if let Some((client_id, client_secret)) = credentials {
        payload.extend_from_slice(EMBEDDED_OAUTH_MAGIC);
        append_obfuscated_field(&mut payload, client_id.as_bytes());
        append_obfuscated_field(&mut payload, client_secret.as_bytes());
    }
    fs::write(Path::new(&out_dir).join(EMBEDDED_OAUTH_FILE_NAME), payload)
}

fn append_obfuscated_field(payload: &mut Vec<u8>, value: &[u8]) {
    let length = u32::try_from(value.len()).expect("OAuth credential field is too long");
    payload.extend_from_slice(&length.to_le_bytes());
    payload.extend(
        value
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ EMBEDDED_OAUTH_KEY[index % EMBEDDED_OAUTH_KEY.len()]),
    );
}

fn credentials_from_environment() -> Option<(String, String)> {
    let client_id = read_first_env(&["ANYA_AGY_OAUTH_CLIENT_ID", "AGY_OAUTH_CLIENT_ID"]);
    let client_secret =
        read_first_env(&["ANYA_AGY_OAUTH_CLIENT_SECRET", "AGY_OAUTH_CLIENT_SECRET"]);
    normalize_credentials(client_id, client_secret)
}

fn read_first_env(names: &[&str]) -> String {
    names
        .iter()
        .find_map(|name| std::env::var(name).ok())
        .unwrap_or_default()
}

fn credentials_from_local_files() -> Option<(String, String)> {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")?;
    OAUTH_LOCAL_FILE_NAMES.iter().find_map(|name| {
        let raw = fs::read_to_string(Path::new(&manifest_dir).join(name)).ok()?;
        credentials_from_json(&raw)
    })
}

fn credentials_from_json(raw: &str) -> Option<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let root = value.as_object()?;
    let credentials = [
        Some(root),
        value
            .get("installed")
            .and_then(serde_json::Value::as_object),
        value.get("web").and_then(serde_json::Value::as_object),
    ]
    .into_iter()
    .flatten()
    .find_map(|object| {
        let client_id = json_string(object, &["client_id", "clientId"]);
        let client_secret = json_string(object, &["client_secret", "clientSecret"]);
        normalize_credentials(client_id, client_secret)
    });
    credentials
}

fn json_string(object: &serde_json::Map<String, serde_json::Value>, names: &[&str]) -> String {
    names
        .iter()
        .find_map(|name| object.get(*name).and_then(serde_json::Value::as_str))
        .unwrap_or_default()
        .to_string()
}

fn normalize_credentials(client_id: String, client_secret: String) -> Option<(String, String)> {
    let client_id = client_id.trim().to_string();
    let client_secret = client_secret.trim().to_string();
    if client_id.is_empty()
        || client_secret.is_empty()
        || client_id.contains('\r')
        || client_id.contains('\n')
        || client_secret.contains('\r')
        || client_secret.contains('\n')
    {
        return None;
    }
    Some((client_id, client_secret))
}
