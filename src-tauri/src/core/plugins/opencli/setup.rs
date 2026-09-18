//! Probe / install the OpenCLI npm package for the plugin settings UI.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use serde::Serialize;

use super::{resolve_opencli_bin, truncate};

const NPM_PACKAGE: &str = "@jackwener/opencli";
const INSTALL_TIMEOUT_SECS: u64 = 300;
const PROBE_TIMEOUT_SECS: u64 = 45;
const DOCTOR_TIMEOUT_SECS: u64 = 90;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCliSetupStatus {
    pub node_ok: bool,
    pub node_version: Option<String>,
    pub npm_ok: bool,
    pub npm_version: Option<String>,
    pub cli_installed: bool,
    pub cli_bin: String,
    pub cli_version: Option<String>,
    pub doctor_ok: Option<bool>,
    pub doctor_output: Option<String>,
    pub hint: String,
    pub extension_url: String,
    pub docs_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCliInstallResult {
    pub ok: bool,
    pub output: String,
    pub status: OpenCliSetupStatus,
}

pub fn probe_status(include_doctor: bool) -> OpenCliSetupStatus {
    let node = version_of("node", &["-v"]);
    let npm = version_of(npm_bin(), &["-v"]);
    let bin = resolve_opencli_bin().unwrap_or_else(|_| default_opencli_name());
    let cli = version_of(&bin, &["--version"]).or_else(|| version_of(&bin, &["-V"]));
    let cli_installed = cli.is_some()
        || run_capture(&bin, &["--help".into()], 15)
            .map(|o| o.code == 0 || looks_like_opencli_help(&o.text))
            .unwrap_or(false);

    let mut doctor_ok = None;
    let mut doctor_output = None;
    if include_doctor && cli_installed {
        match run_capture(&bin, &["doctor".into()], DOCTOR_TIMEOUT_SECS) {
            Ok(o) => {
                doctor_ok = Some(o.code == 0);
                doctor_output = Some(truncate(&sanitize_npm_log(&o.text), 12_000));
            }
            Err(e) => {
                doctor_ok = Some(false);
                doctor_output = Some(e);
            }
        }
    }

    let hint = if node.is_none() {
        "Install Node.js ≥ 20, then use one-click install.".into()
    } else if npm.is_none() {
        "Node is present but npm was not found on PATH.".into()
    } else if !cli_installed {
        "Click Install to run: npm i -g @jackwener/opencli".into()
    } else if doctor_ok == Some(false) {
        "CLI is installed but doctor failed — install the Chrome Browser Bridge extension.".into()
    } else if include_doctor && doctor_ok == Some(true) {
        "OpenCLI looks ready.".into()
    } else {
        "CLI found. Run doctor to verify Browser Bridge.".into()
    };

    OpenCliSetupStatus {
        node_ok: node.is_some(),
        node_version: node,
        npm_ok: npm.is_some(),
        npm_version: npm,
        cli_installed,
        cli_bin: bin,
        cli_version: cli,
        doctor_ok,
        doctor_output,
        hint,
        extension_url:
            "https://chromewebstore.google.com/detail/opencli/ildkmabpimmkaediidaifkhjpohdnifk"
                .into(),
        docs_url: "https://github.com/jackwener/OpenCLI".into(),
    }
}

pub fn install_cli() -> Result<OpenCliInstallResult, String> {
    let npm = npm_bin();
    if version_of(npm, &["-v"]).is_none() {
        return Err(
            "npm not found. Install Node.js ≥ 20 from https://nodejs.org, restart Anya, then retry."
                .into(),
        );
    }

    let output = run_capture(
        npm,
        &["install".into(), "-g".into(), NPM_PACKAGE.into()],
        INSTALL_TIMEOUT_SECS,
    )?;

    let status = probe_status(true);
    let packages_added = output.code == 0
        && (output.text.contains("added")
            || output.text.contains("up to date")
            || output.text.contains("changed")
            || status.cli_installed);
    let ok = output.code == 0 && (status.cli_installed || packages_added);
    let mut log = sanitize_npm_log(&output.text);
    if output.code == 0 && !status.cli_installed {
        log.push_str(
            "\n\nNote: npm finished, but `opencli` is not on Anya's PATH yet. Restart Anya, or set OPENCLI_BIN to the full path of opencli.cmd (usually under %APPDATA%\\npm).",
        );
    }
    Ok(OpenCliInstallResult {
        ok,
        output: truncate(&log, 20_000),
        status,
    })
}

/// Prefer PATH, then common npm global shim locations.
pub fn discover_opencli_bin() -> Option<String> {
    for path in opencli_candidate_paths() {
        if path.is_file() {
            return Some(path.to_string_lossy().into_owned());
        }
    }
    let bare = default_opencli_name();
    if version_of(&bare, &["--version"]).is_some() || version_of(&bare, &["-V"]).is_some() {
        return Some(bare);
    }
    None
}

fn opencli_candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(prefix) = run_capture(npm_bin(), &["prefix".into(), "-g".into()], 20) {
        if prefix.code == 0 {
            let root = prefix
                .text
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty() && !l.starts_with("npm warn"))
                .unwrap_or("");
            if !root.is_empty() {
                let base = PathBuf::from(root);
                if cfg!(windows) {
                    out.push(base.join("opencli.cmd"));
                    out.push(base.join("opencli.exe"));
                    out.push(base.join("node_modules").join(".bin").join("opencli.cmd"));
                } else {
                    out.push(base.join("bin").join("opencli"));
                    out.push(base.join("opencli"));
                }
            }
        }
    }
    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let npm = PathBuf::from(appdata).join("npm");
            out.push(npm.join("opencli.cmd"));
            out.push(npm.join("opencli"));
        }
    } else if let Ok(home) = std::env::var("HOME") {
        out.push(
            PathBuf::from(home)
                .join(".npm-global")
                .join("bin")
                .join("opencli"),
        );
    }
    out
}

fn default_opencli_name() -> String {
    if cfg!(windows) {
        "opencli.cmd".into()
    } else {
        "opencli".into()
    }
}

fn looks_like_opencli_help(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("opencli") || lower.contains("usage") || lower.contains("commands")
}

fn sanitize_npm_log(text: &str) -> String {
    let filtered: Vec<&str> = text
        .lines()
        .filter(|line| {
            let t = line.trim();
            if t.is_empty() {
                return true;
            }
            if t.eq_ignore_ascii_case("--- stderr ---") {
                return false;
            }
            if t.starts_with("npm warn Unknown env config") {
                return false;
            }
            true
        })
        .collect();
    let mut out = filtered.join("\n");
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    out.trim().to_string()
}

fn version_of(bin: &str, args: &[&str]) -> Option<String> {
    let argv: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    let out = run_capture(bin, &argv, PROBE_TIMEOUT_SECS).ok()?;
    if out.code != 0 && out.text.trim().is_empty() {
        return None;
    }
    let line = out
        .text
        .lines()
        .map(str::trim)
        .find(|l| {
            !l.is_empty()
                && !l.starts_with("npm warn")
                && !l.eq_ignore_ascii_case("--- stderr ---")
        })?
        .to_string();
    if line.starts_with("failed") || line.contains("not found") || line.contains("不是内部") {
        return None;
    }
    Some(line)
}

fn npm_bin() -> &'static str {
    if cfg!(windows) {
        "npm.cmd"
    } else {
        "npm"
    }
}

pub struct Captured {
    pub code: i32,
    pub text: String,
}

pub fn run_capture_public(bin: &str, argv: &[String], timeout_secs: u64) -> Result<Captured, String> {
    run_capture(bin, argv, timeout_secs)
}

fn run_capture(bin: &str, argv: &[String], timeout_secs: u64) -> Result<Captured, String> {
    let bin2 = bin.to_string();
    let argv = argv.to_vec();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut cmd = Command::new(&bin2);
        cmd.args(&argv)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        for key in [
            "npm_config_allow_builds",
            "npm_config_verify_deps_before_run",
            "npm_config__jsr_registry",
        ] {
            cmd.env_remove(key);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let _ = tx.send(cmd.output());
    });
    match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(Ok(output)) => {
            let code = output.status.code().unwrap_or(-1);
            let mut text = String::new();
            let out = String::from_utf8_lossy(&output.stdout);
            let err = String::from_utf8_lossy(&output.stderr);
            if !out.is_empty() {
                text.push_str(&out);
            }
            if !err.is_empty() {
                if !text.is_empty() {
                    text.push_str("\n--- stderr ---\n");
                }
                text.push_str(&err);
            }
            if text.is_empty() {
                text = format!("(no output, exit {code})");
            }
            Ok(Captured { code, text })
        }
        Ok(Err(e)) => Err(format!("failed to run `{bin}`: {e}")),
        Err(_) => Err(format!("`{bin}` timed out after {timeout_secs}s")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_npm_unknown_env_warnings() {
        let raw = "added 17 packages in 25s\n\n--- stderr ---\nnpm warn Unknown env config \"allow-builds\". This will stop working in the next major version of npm.\nnpm warn Unknown env config \"verify-deps-before-run\". This will stop working in the next major version of npm.\n";
        let cleaned = sanitize_npm_log(raw);
        assert!(cleaned.contains("added 17 packages"));
        assert!(!cleaned.contains("Unknown env config"));
        assert!(!cleaned.contains("--- stderr ---"));
    }
}
