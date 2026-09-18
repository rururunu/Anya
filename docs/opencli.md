# OpenCLI plugin

<p align="center">
  <a href="./opencli.md">English</a>
  &nbsp;·&nbsp;
  <a href="./opencli.zh-CN.md">简体中文</a>
</p>

Official **agent** plugin `opencli` (default **off**). Wraps the local [OpenCLI](https://github.com/jackwener/OpenCLI) CLI so Anya can run site adapters and drive a **logged-in Chrome** via Browser Bridge — complementary to [Computer Use](./computer-use.md) (Ghost desktop + CDP).

## Prerequisites

1. Node.js ≥ 20
2. Install CLI via the plugin **Settings** one-click button, or `npm i -g @jackwener/opencli`
3. OpenCLI Chrome extension (Browser Bridge)
4. `opencli doctor` green (also from Settings)

Optional: set `OPENCLI_BIN` to a full path if `opencli` is not on `PATH`.

## Tools

| Tool      | Approval | Notes                                |
| --------- | -------- | ------------------------------------ |
| `doctor`  | no       | Setup check                          |
| `list`    | no       | Adapters / commands (`--json`)       |
| `run`     | yes      | Site adapters via `argv` / `command` |
| `browser` | yes      | `opencli browser <session> …`        |

Agent names: `plugin_opencli__*`.

## Routing

- Logged-in web / known sites → OpenCLI
- Desktop apps, Paint, Explorer, UIA → Computer Use
- Same Chrome profile: **one** stack per turn (do not mix Ghost CDP and OpenCLI Bridge)

## Code map

| Piece                          | Path                         |
| ------------------------------ | ---------------------------- |
| Manifest, host schemas, skills | `src-tauri/plugins/opencli/` |
| Runner                         | `core/plugins/opencli.rs`    |
| Permission                     | `opencli`                    |

Related: [Plugin system](./plugin-system.md), [Computer use](./computer-use.md).
