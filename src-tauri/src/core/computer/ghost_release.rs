//! Install official Ghost MCP releases when no automation process is running.
use std::{
    fs,
    io::{self, Cursor},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use serde_json::Value;
use sha2::{Digest, Sha256};

use super::ghost_process::GhostProcess;

const REPO: &str = "https://github.com/NORTHTEKDevs/ghost";
const LATEST: &str = "https://api.github.com/repos/NORTHTEKDevs/ghost/releases/latest";
const ASSET: &str = "ghost-windows-x64.zip";
const EXE: &str = "ghost-mcp.exe";
const BASELINE: &str = "0.23.4";
const BASELINE_DIGEST: &str = "c7cdbed52b587381a147b30eef62f10ce5276c52a8093740071a67b9306aab4a";
const MAX_ASSET: u64 = 100 * 1024 * 1024;
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

pub(super) async fn start() -> Result<GhostProcess, String> {
    let root = crate::core::tools::memory::user_data_dir()
        .join("tools")
        .join("ghost-managed");
    start_from(&root).await
}

async fn start_from(root: &Path) -> Result<GhostProcess, String> {
    fs::create_dir_all(root).map_err(|err| format!("Create Ghost MCP cache: {err}"))?;
    let mut installed = installed_versions(root);
    let stamp = root.join("last-update-check");
    let checked_recently = fs::metadata(&stamp)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age < CHECK_INTERVAL);
    let mut failure = None;
    if !checked_recently && cfg!(target_arch = "x86_64") {
        let update = async {
            let client = http_client()?;
            let release = latest(&client).await?;
            if installed
                .first()
                .is_none_or(|version| version_number(&release.version) > version_number(version))
            {
                install(&client, root, &release).await?;
            }
            Ok::<_, String>(())
        };
        match tokio::time::timeout(Duration::from_secs(25), update).await {
            Ok(Ok(())) => installed = installed_versions(root),
            Ok(Err(err)) => failure = Some(err),
            Err(_) => failure = Some("Ghost MCP update check timed out".into()),
        }
        if let Some(err) = &failure {
            tracing::warn!(error = %err, "Ghost MCP update unavailable");
        }
        // An empty cache must be able to retry immediately after a failed first install.
        if !installed.is_empty() {
            let _ = fs::write(&stamp, []);
        }
    }

    for version in installed {
        match GhostProcess::start(&root.join(&version).join(EXE), &version).await {
            Ok(child) => return Ok(child),
            Err(err) => {
                tracing::warn!(%version, error = %err, "Ghost MCP startup failed; trying previous release");
                failure = Some(err);
            }
        }
    }

    if !cfg!(target_arch = "x86_64") {
        return Err("Managed Ghost MCP requires a Windows x64 release".into());
    }
    let release = Release {
        version: BASELINE.into(),
        digest: BASELINE_DIGEST.into(),
    };
    let result = install(&http_client()?, root, &release).await;
    if let Err(err) = result {
        return Err(match failure {
            Some(update) => format!("{update}. Ghost MCP {BASELINE} fallback: {err}"),
            None => err,
        });
    }
    let child = GhostProcess::start(&root.join(BASELINE).join(EXE), BASELINE).await?;
    let _ = fs::write(&stamp, []);
    Ok(child)
}

fn installed_versions(root: &Path) -> Vec<String> {
    let mut versions: Vec<_> = fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join(EXE).is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| version_number(name).is_some())
        .collect();
    versions.sort_by_key(|name| std::cmp::Reverse(version_number(name)));
    versions
}

fn version_number(version: &str) -> Option<(u32, u32, u32)> {
    let mut parts = version.split('.');
    let version = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    parts.next().is_none().then_some(version)
}

struct Release {
    version: String,
    digest: String,
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("Anya/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(75))
        .build()
        .map_err(|err| err.to_string())
}

async fn latest(client: &reqwest::Client) -> Result<Release, String> {
    let response = client
        .get(LATEST)
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|err| format!("Check official Ghost releases: {err}"))?;
    let release = response
        .json::<Value>()
        .await
        .map_err(|err| err.to_string())?;
    let version = release["tag_name"]
        .as_str()
        .and_then(|tag| tag.strip_prefix('v'))
        .filter(|version| version_number(version).is_some())
        .ok_or("Invalid Ghost release version")?;
    let asset = release["assets"]
        .as_array()
        .and_then(|assets| assets.iter().find(|asset| asset["name"] == ASSET))
        .ok_or("Official Ghost release has no Windows x64 asset")?;
    let digest = asset["digest"]
        .as_str()
        .and_then(|value| value.strip_prefix("sha256:"))
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or("Official Ghost release has no valid SHA-256 digest")?;
    Ok(Release {
        version: version.into(),
        digest: digest.to_ascii_lowercase(),
    })
}

async fn install(client: &reqwest::Client, root: &Path, release: &Release) -> Result<(), String> {
    let dir = root.join(&release.version);
    if dir.join(EXE).is_file() {
        return Ok(());
    }
    let url = format!(
        "{REPO}/releases/download/v{}/{ASSET}?download=1",
        release.version
    );
    let mut response = client
        .get(&url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|err| format!("Download Ghost MCP {}: {err}", release.version))?;
    if response.content_length().is_some_and(|len| len > MAX_ASSET) {
        return Err("Ghost MCP archive exceeds the size limit".into());
    }
    let mut archive = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|err| err.to_string())? {
        if archive.len() as u64 + chunk.len() as u64 > MAX_ASSET {
            return Err("Ghost MCP archive exceeds the size limit".into());
        }
        archive.extend_from_slice(&chunk);
    }
    if format!("{:x}", Sha256::digest(&archive)) != release.digest {
        return Err(format!("Ghost MCP {} SHA-256 mismatch", release.version));
    }
    let staging = Staging::new(root)?;
    extract_exe(&archive, &staging.0)?;
    let candidate = GhostProcess::start(&staging.0.join(EXE), &release.version).await?;
    candidate.stop(false).await;
    match fs::rename(&staging.0, &dir) {
        Ok(()) => Ok(()),
        Err(_) if dir.join(EXE).is_file() => Ok(()), // Another Anya process installed it.
        Err(err) => Err(format!("Activate Ghost MCP {}: {err}", release.version)),
    }
}

fn extract_exe(bytes: &[u8], dir: &Path) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|err| err.to_string())?;
    for index in 0..zip.len() {
        let entry = zip.by_index(index).map_err(|err| err.to_string())?;
        let path = entry
            .enclosed_name()
            .ok_or("Invalid Ghost MCP archive entry")?;
        if !entry.is_file() || path.file_name().and_then(|name| name.to_str()) != Some(EXE) {
            continue;
        }
        if entry.size() > MAX_ASSET {
            return Err("Ghost MCP executable exceeds the size limit".into());
        }
        let mut exe = fs::File::create_new(dir.join(EXE)).map_err(|err| err.to_string())?;
        let written = io::copy(&mut io::Read::take(entry, MAX_ASSET + 1), &mut exe)
            .map_err(|err| err.to_string())?;
        return if written > MAX_ASSET {
            Err("Ghost MCP executable exceeds the size limit".into())
        } else {
            Ok(())
        };
    }
    Err("Official Ghost MCP archive is missing ghost-mcp.exe".into())
}

struct Staging(PathBuf);

impl Staging {
    fn new(root: &Path) -> Result<Self, String> {
        let dir = root.join(format!(".staging-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&dir).map_err(|err| err.to_string())?;
        Ok(Self(dir))
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.0.join(EXE));
        let _ = fs::remove_dir(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn release_selection_uses_numeric_versions_and_rejects_prereleases() {
        assert!(version_number("0.24.0") > version_number("0.23.19"));
        assert!(version_number("0.24.0-rc1").is_none());
        assert!(version_number("../../another-file").is_none());
    }

    #[tokio::test]
    #[ignore = "requires access to the official GitHub release"]
    async fn ghost_release_installs_from_a_clean_cache() {
        let root = Staging::new(&std::env::temp_dir()).unwrap();
        let process = start_from(&root.0).await.unwrap();
        process.stop(false).await;
        let installed = installed_versions(&root.0);
        assert_eq!(installed.len(), 1);
        let dir = root.0.join(&installed[0]);
        fs::remove_file(dir.join(EXE)).unwrap();
        fs::remove_dir(dir).unwrap();
        fs::remove_file(root.0.join("last-update-check")).unwrap();
    }

    #[tokio::test]
    #[ignore = "requires a verified official release archive"]
    async fn ghost_release_falls_back_and_handles_a_real_mcp_call() {
        let archive = fs::read(std::env::var_os("ANYA_GHOST_TEST_ARCHIVE").unwrap()).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(&archive)), BASELINE_DIGEST);
        let root = Staging::new(&std::env::temp_dir()).unwrap();
        let baseline = root.0.join(BASELINE);
        fs::create_dir(&baseline).unwrap();
        extract_exe(&archive, &baseline).unwrap();
        let higher = root.0.join("999.0.0");
        fs::create_dir(&higher).unwrap();
        fs::write(higher.join(EXE), b"incompatible release").unwrap();
        fs::write(root.0.join("last-update-check"), []).unwrap();
        let result = async {
            let mut process = start_from(&root.0).await?;
            let screen =
                super::super::ghost_ops::dispatch(&mut process, "screen_info", &json!({})).await?;
            assert_eq!(
                serde_json::from_str::<Value>(&screen).unwrap()["policy"],
                "background"
            );
            let error =
                super::super::ghost_ops::dispatch(&mut process, "key", &json!({"keys":"Enter"}))
                    .await
                    .unwrap_err();
            assert!(error.contains("Select a live target"));
            process.call("ghost_reset", json!({})).await?;
            let mut wait = Box::pin(process.call(
                "ghost_wait",
                json!({"for":"event","since_seq":4294967295_u64,"timeout_ms":10000}),
            ));
            assert!(tokio::time::timeout(Duration::from_millis(100), &mut wait)
                .await
                .is_err());
            drop(wait);
            assert!(process.transport_failed());
            tokio::time::timeout(Duration::from_secs(2), process.stop(true))
                .await
                .unwrap();
            Ok::<_, String>(())
        }
        .await;
        for dir in [&higher, &baseline] {
            fs::remove_file(dir.join(EXE)).unwrap();
            fs::remove_dir(dir).unwrap();
        }
        fs::remove_file(root.0.join("last-update-check")).unwrap();
        result.unwrap();
    }
}
