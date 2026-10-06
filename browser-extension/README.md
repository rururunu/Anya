# Anya Selection Bridge

Optional Chrome / Edge extension for accurate webpage text selection. Anya's native capture remains available without it.

## Install on Windows

For the prebuilt bridge ZIP, extract it to a permanent folder, run `install.ps1` from that folder, then perform steps 3–4 below. It already contains the host executable. The desktop app must be a new build with selection bridge support.

1. Build the desktop application with the current source. Build the small native host:

   ```powershell
   cargo build --manifest-path src-tauri/Cargo.toml --bin anya-browser-bridge
   ```

2. Register the host for your Windows account:

   ```powershell
   powershell -ExecutionPolicy Bypass -File browser-extension/install.ps1
   ```

3. In `chrome://extensions` or `edge://extensions`, enable Developer mode and **Load unpacked**, selecting this `browser-extension` directory. Reload existing webpages.
4. Restart Anya, select text in a webpage, and press the configured global shortcut. An inline selection tag appears in the composer.

For a release host, pass `-HostExecutable C:\path\anya-browser-bridge.exe` to the installer. Do not move the host after registration. The extension's fixed public key keeps its ID stable.

## Behavior and boundaries

- Only selected text is sent to the local app; no remote endpoint or browsing-history collection. Password input selections are excluded.
- Selection length is capped to keep large webpages from freezing the composer.
- Browser host permissions cover HTTP(S) pages; Chrome displays them during installation. Browser-internal pages and built-in PDF viewers fall back to native capture.
- Selection snapshots must match the foreground browser window, process and title. Empty selections / tab changes invalidate the cache; snapshots expire after 60 seconds.
- Native messaging checks the registered extension origin. The loopback bridge uses a random port and token, rejects oversized packets and never logs selection content.
- Read-only webpage selections, editable fields and same/cross-origin frames with content-script access are supported. Restricted frames or custom canvas editors can require native copy fallback.
- Native capture runs before overlay focus, in a worker. UI Automation / MSAA are preferred, with WM_COPY / Ctrl+Insert as fallbacks. Ctrl+C is never synthesized in any application, avoiding interruption of embedded terminals. If capture fails, copy and paste manually.

Remove the extension in the browser, then unregister the host:

```powershell
powershell -ExecutionPolicy Bypass -File browser-extension/install.ps1 -Uninstall
```

[中文](README.zh-CN.md)

Maintainers can create the optional ZIP after building the host with `powershell -ExecutionPolicy Bypass -File browser-extension/package.ps1`.
