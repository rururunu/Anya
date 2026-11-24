# Computer Use

[简体中文](./computer-use.zh-CN.md)

Computer Use is built into Anya on Windows. Describe the task in chat.

- Use `computer_window` to select a target, then `computer_see` or `computer_find` to inspect it.
- Use `computer_act` to click or type, and check `verified`. Use `computer_wait` and `computer_assert` to confirm the result.
- Use `computer_click` or `computer_drag` for canvases and `computer_browser` or `computer_tab` for browser automation.
- Desktop mutations follow the current chat's tool approval settings.

On the first Computer Use call in a turn, Anya starts a dedicated [Ghost MCP](https://github.com/NORTHTEKDevs/ghost) process and communicates over private stdio. Anya checks the official Windows x64 archive's SHA-256, MCP identity and available tools before activating a release. Anya checks for updates at most once every 24 hours before starting a session. It retains verified versions and tries the preceding version if a candidate fails to start. The first use needs access to the official release download.

One chat turn owns desktop control at a time. **Stop** on the HUD cancels that turn and stops Ghost. Ending the turn releases the process and its window anchor. After a timeout or disconnect, inspect the target to confirm the last action's outcome.

## Terminal

The Terminal tab in the right workbench sidebar opens local PowerShell. It inherits the current Windows user's permissions and environment, with access to files, drives, and executables available to that user. The current chat's workspace is the starting directory when available. The terminal session stays open when the sidebar is hidden or another tab is selected. Agent shell commands use built-in shell tools and follow the chat's tool approval mode.

The Rust implementation lives in `src-tauri/src/core/computer/` and `src-tauri/src/commands/terminal.rs`.
