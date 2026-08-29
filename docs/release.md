# Releases and remote updates

Anya uses the [Tauri Updater](https://v2.tauri.app/plugin/updater/) plugin. The client reads:

`https://github.com/rururunu/Anya/releases/latest/download/latest.json`

## Where is `latest.json`?

| Location           | Notes                                                                                       |
| ------------------ | ------------------------------------------------------------------------------------------- |
| **Git repo**       | No committed `latest.json` by default                                                       |
| **Local output**   | **`release/latest.json`** after `pnpm release:json` (plus a copy named with the notes slug) |
| **GitHub Release** | Uploaded asset; **must be named `latest.json`**                                             |
| **Client URL**     | `releases/latest/download/latest.json` (latest non-prerelease)                              |

Signed build artifacts:

- `src-tauri/target/release/bundle/msi/Anya_<version>_x64.msi`
- `src-tauri/target/release/bundle/msi/Anya_<version>_x64.msi.sig`

The `.sig` file **contents** go into `platforms.windows-x86_64.signature` (not a URL).

---

## Keys (one-time setup)

```powershell
pnpm tauri signer generate -w "$env:USERPROFILE\.tauri\anya.key" --ci
```

- **Private key**: `%USERPROFILE%\.tauri\anya.key` (never commit)
- **Public key**: `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`

GitHub Actions secrets: `TAURI_SIGNING_PRIVATE_KEY` (full `.key` file contents, including `untrusted comment:` line), optional `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, optional `ANYA_UPDATER_PUBKEY`.

If CI fails with **`Missing comment in secret key`**, the private-key secret is empty, truncated, or not a minisign private key — see [release.zh-CN.md](./release.zh-CN.md) FAQ.

---

## Manual release

1. Bump `version` in `src-tauri/tauri.conf.json` and `package.json`.
2. Build with signing:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$env:USERPROFILE\.tauri\anya.key" -Raw
pnpm tauri:build
```

3. Generate **`release/latest.json`**:

```powershell
pnpm release:json -- --tag v0.2.14 --notes "Optimistic workspace/session archive, DeepSeek multimodal with the built-in key, IME placeholder fix, CSS-token light/dark theme (WebView2 stays only-light), and 16px workbench window corners."
```

4. On GitHub → [Releases](https://github.com/rururunu/Anya/releases), create tag `v0.2.14` and upload:
   - `Anya_0.2.14_x64.msi`
   - `Anya_0.2.14_x64.msi.sig`
   - `latest.json` (from `release/latest.json`)

5. Verify `https://github.com/rururunu/Anya/releases/latest/download/latest.json` and check for updates in the app.

---

## Optional CI

Push a `v*` tag to run `.github/workflows/release.yml` (build, sign, upload Release + `latest.json`).

See [简体中文](./release.zh-CN.md) for the full walkthrough and troubleshooting.

**Related:** [Architecture](./architecture-overview.md) · [Docs index](./README.md)
