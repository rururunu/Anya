# Computer use

<p align="center">
  <a href="./computer-use.md">English</a>
  &nbsp;·&nbsp;
  <a href="./computer-use.zh-CN.md">简体中文</a>
</p>

Official **agent** plugin `computer-use` (Windows, **off until Enable**). No workbench UI — grant `computer`, then ask Anya in chat, or type `#plugin:computer-use`. Mutating tools use the existing approval picker; **Allow for session** avoids a confirm on every click.

It is not a screenshot-only VLM. Screenshot is the eye; UI Automation is the skeleton; `launch` / keyboard are the fast path.

## 1. Hybrid pipeline

The agent is instructed (plugin prompt + bundled playbook) to climb this ladder. Lower rungs are faster and hit more reliably:

| Level | How                                      | Typical tools                                                                     |
| ----- | ---------------------------------------- | --------------------------------------------------------------------------------- |
| 1     | ShellExecute — open an app, file, or URI | `launch` (`mspaint`, `notepad`, `C:\doc.docx`, `ms-settings:display`)             |
| 2     | App / OS shortcuts                       | `key` (`ctrl+s`, `win+e`, `alt+f4`)                                               |
| 3     | UIA semantics                            | `click_control` (InvokePattern, else clickable point), `set_value` (ValuePattern) |
| 4     | Last-resort pixels                       | `click` / `drag` / `scroll` on canvas, snip region, or unnamed chrome             |

Do not hunt the Start menu. Do not screenshot after every click. One screenshot to aim, then emit the remaining calls in the **same** model turn (mutating tools run in order).

## 2. Dual input: JPEG + UIA tree

`screenshot` captures the **foreground window** by default (`scope: desktop` for the virtual screen). The tool result is:

1. A JPEG (downscaled to a 1280px edge). Click/move/scroll `x,y` are pixels in **this** image, origin top-left.
2. Up to 40 interactive UIA controls: name, AutomationId, **image-space** rect. `click_control` `index` / `name` / `id` uses that list.

`find_control` searches Name **or** AutomationId when the overlay is truncated. Coordinates for named clicks come from UIA bounding boxes / `GetClickablePoint`, not from the model guessing pixels.

JPEG → screen mapping: `screen = image / scale + window origin`, where `scale = image_w / window_w`. Capture, UIA rects, and SendInput all use **physical** pixels. The result also reports window **DPI** so a 125%/150%/4K display is not treated as CSS pixels.

After `click` / `click_control`, Anya probes `ElementFromPoint` and the foreground title (no JPEG recapture). If the foreground looks like Anya, the tool result says so — focus the target app instead of clicking again.

## 3. Bundled Windows playbook

`plugin.json` `contributes.agent.skills: ["skills/windows.md"]` is appended to the system prompt when `agent.prompt` is granted (no extra `load_skill` turn). Recipes cover Paint, Notepad, Explorer, Settings, Calculator, Snipping, Task Manager, and common dialogs (EN + zh-CN labels).

Source of truth: [`src-tauri/plugins/computer-use/skills/windows.md`](../src-tauri/plugins/computer-use/skills/windows.md). Official plugins are copied into AppData on launch.

## 4. Code map

| Piece                            | Path                                                                                   |
| -------------------------------- | -------------------------------------------------------------------------------------- |
| Manifest, host schemas, playbook | `src-tauri/plugins/computer-use/`                                                      |
| Dispatch + tools                 | `core/plugins/computer/` (`ops.rs`, `win/{capture,uia,snapshot,launch,observe,input}`) |
| Prompt assembly (`skills` files) | `core/plugins/prompt.rs`                                                               |
| Agent tool names                 | `plugin_computer-use__*`                                                               |

`computer.*` host RPC is the same primitive set for a UI plugin with `computer` granted. The official plugin is `role: agent` — the chat agent calls the tools directly.

Related: [Plugin system](./plugin-system.md) (`contributes.agent.skills`, permission `computer`).
