# Computer use

<p align="center">
  <a href="./computer-use.md">English</a>
  &nbsp;·&nbsp;
  <a href="./computer-use.zh-CN.md">简体中文</a>
</p>

Official **agent** plugin `computer-use` (Windows, **off until Enable**). No workbench UI — grant `computer`, then ask Anya in chat, or type `#plugin:computer-use`. Mutating tools use the existing approval picker; **Allow for session** avoids a confirm on every click.

Execution engine: in-process **[Ghost](https://github.com/NORTHTEKDevs/ghost)** (`ghost-session` v0.23.4, MIT). Anya keeps permission grants, tool approval, and the Computer Use HUD; Ghost owns UIA, background focus policy, verification, and browser CDP.

## 1. Ghost-style pipeline

| Step     | Tool                            | Notes                                                               |
| -------- | ------------------------------- | ------------------------------------------------------------------- |
| 1        | `window`                        | `launch` / `focus` / `anchor` — session remembers the target window |
| 2        | `see`                           | Prefer `mode=text`; else name/role/enabled/center list              |
| 3        | `act`                           | Click/type by name/role; read **`verified`**                        |
| 4        | `wait` / `assert`               | Element / value / idle — avoid long sleeps                          |
| 5        | `browser` + `tab`               | CDP for the web — never pixel-click pages                           |
| Fallback | `screenshot` / `click` / `drag` | Canvas / snip / unnamed chrome only                                 |

Background focus policy is the default (`GHOST_FOCUS_LOCK`): Ghost prefers posted messages / UIA without stealing the user's mouse.

## 2. Tool surface

**Desktop:** `see`, `act`, `wait`, `assert`, `window`, `key`/`hotkey`, `scroll`, `drag`, `clipboard`, `screenshot`, `screen_info`.

**Browser:** `browser` (`launch` \| `attach` \| `tabs` \| `close`), `tab` (`open` \| `navigate` \| `click` \| `type` \| `text` \| `eval` \| `wait` \| …).

**Aliases** (old prompts): `list_windows`, `focus_window`, `find_control`, `click_control`, `set_value`, `launch`, `click`, `type`.

Read-only tools (`see`, `assert`, `wait`, `screenshot`, `list_windows`, `find_control`, …) skip approval; mutating `act` / `window` / `key` / `browser` / `tab` / … require it (session allow still clears the class).

## 3. HUD + stop

On the first `plugin_computer-use__*` tool in a chat, Anya opens the glow + banner. Chat finished or HUD **Stop** calls Ghost emergency stop, clears the window anchor, and closes the HUD.

## 4. Bundled playbook + learned library

`plugin.json` `contributes.agent.skills: ["skills/windows.md"]` is appended when `agent.prompt` is granted. Recipes cover Notepad, Explorer, Paint, Settings, Calculator, dialogs, and CDP browser flow (EN + zh-CN labels).

The **`playbook` tool** (Agent S2 retrieve/write + EchoPath-style steps) stores successful semantic procedures under `%APPDATA%/Anya/computer-use/playbooks/*.json`. Next time: `lookup` before exploring; on a failed `verified` step: `fail` (confidence drop / quarantine), then explore and `save` with the same `id` to patch. An index of local playbooks is injected into the prompt when the plugin is enabled.

Source: [`src-tauri/plugins/computer-use/skills/windows.md`](../src-tauri/plugins/computer-use/skills/windows.md); impl: `core/plugins/computer/playbook/`.

## 5. Code map

| Piece                                  | Path                                                             |
| -------------------------------------- | ---------------------------------------------------------------- |
| Manifest, host schemas, playbook       | `src-tauri/plugins/computer-use/`                                |
| Ghost bridge + ops + learned playbooks | `core/plugins/computer/{ghost_bridge,ghost_ops,playbook,mod}.rs` |
| Dependency                             | `Cargo.toml` → `ghost-session` (Windows `cfg`, pinned git tag)   |
| Agent tool names                       | `plugin_computer-use__*`                                         |

Pin upstream Ghost releases; bump the tag deliberately and note breaking tool changes. Binary size and first-call COM init cost grow with the engine; that is expected for in-process embed.

Do **not** also enable Ghost MCP for the same session — one desktop stack only. For logged-in websites prefer the official [OpenCLI](./opencli.md) plugin; do not drive the same Chrome with Ghost CDP and OpenCLI Bridge in one turn.

Related: [Plugin system](./plugin-system.md) (`contributes.agent.skills`, permission `computer`).
