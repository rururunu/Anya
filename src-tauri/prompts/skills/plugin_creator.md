---
name: plugin_creator
description: >
  Anya user-plugin spec. Load this for create/fix/debug AND for any question
  about surfaces, UI entry, ctx.*, manage_plugin, AppData plugin folders,
  theming, blank settings, or fs.pick. Never grep Anya src/ or src-tauri for
  those answers — this file plus describe_contract is the contract.
---

# Plugin Creator

Create and iterate Anya user plugins with `manage_plugin` only. Never edit Anya Vue/Rust, the install directory, or settings, and never add Cargo/exe/cdylib to a plugin.

## When to use

Load this skill (do not guess, do not open Anya's loader) whenever the task is about **user plugins**, including questions:

- "做个插件" / sidebar / terminal / 背景视频 / 输入框按钮 / 猫耳朵 / mascot
- blank settings, `mount() rendered nothing`, theming, `ctx.fs.pick`, `surfaces`, UI 入口, `activate.js`
- "插件目录不在允许范围内" / where plugins live / how Enable bundles UI

Triggers are questions as well as edits. **If you are unsure of a rule, read the matching table below or call `describe_contract` once. Do not `Grep`/`read_file` `src/` or `src-tauri/`.**

## When NOT to use

| Request | Do this instead |
| --- | --- |
| Change Anya's own UI/colors | `manage_custom_theme` |
| Fix a bug in Anya product source | edit the workspace, not `manage_plugin` |
| Only asking how plugins work | answer from **this file**; still do not create files |

## Two directories

The current workspace is Anya's **product source**. User plugins are **not** in it.

| Tree | Location | How to touch it |
| --- | --- | --- |
| Anya source | Workspace `src/`, `src-tauri/` (e.g. `C:\My\code\AltAltAi`) | Readable as the open project. **Not** the plugin SDK. Opening `bundle.rs` / `sdk.ts` / `slotRegistry.ts` to "learn surfaces" is wasted turns. |
| User plugins | `%APPDATA%\Anya\plugins\<id>\` (debug build: `Anya Debug\plugins\<id>`) | **Outside the workspace.** `read_file` / `Grep` / `list_folder` / `run_shell` on that path wait on a permission prompt for minutes and look stuck. Use `manage_plugin` `list` → `list_files` → `get_file` / `put_file`. |

If a tool says a plugin path is not allowed: that is expected. Switch to `manage_plugin`. Do **not** fall back to grepping `src-tauri` because "that one is allowed".

## If you are unsure (lookup — do not open the loader)

| Question | Answer |
| --- | --- |
| Where are plugin files? | AppData table above. `manage_plugin list` then `get_file`. |
| What is `surfaces`? | Table **Surfaces**. Launchers ≠ pane. 主视图 is not `"views"`. |
| 主视图 / 中间 / 像「插件」页 | `surfaces: ["nav"]` **without** `"views"`, or `ctx.workbench.setView`. Never `["nav","views"]`. |
| 工作区视图 / 右侧 review | `surfaces` includes `"views"` (terminal). |
| 自己的页面 + Agent 底座 | 主视图 + **自己的 UI** + `ctx.agent.run(prompt, { sessionId })`. Do **not** `send` / `mount` — those wrap the open workbench chat. 资料/成品 = columns **inside** that page. |
| 自己的页面，但要 Anya 对话壳 | 主视图 + `ctx.agent.mount(el)` (aliases: `ctx.conversation.mount`). `send` / `sessionId` on `ctx.agent`. |
| 对话页上的资料区 | **Do not** `conversation.addMaterials` or ask for a new anchor. Compose `workbench.main`. |
| Blank / default puzzle icon | You omitted `ui/icon.svg` or `plugin.json` `"icon"`. Draw a 48×48 SVG that reads at 16px; do not use Lucide names as the only mark. |
| Which file do I edit? | `ui/src/activate.js`. Never `ui/.anya/activate.js`. |
| What does the workbench actually load? | Enable/reload writes `ui/.anya/activate.js` from the source in **UI entry**. |
| File picker? | `ctx.fs.pick` (permission `fs.pick`). Not a hidden RPC. |
| Settings / 详情? | Home page is automatic. `about` file + `ctx.home.setSettingsView(mount)`. |
| Blank "设置" / empty-mount error? | `mount` built a node and forgot `el.append(...)`. |
| Native `<select>` unreadable? | Don't use it. Button row + `--peek-*` tokens. |
| Persist config on workbench? | Workbench `ctx` has **no** `storage`. `localStorage` (or isolated-window `AnyaPlugin.storageGet/Set`). |
| Video/mascot backdrop? | `ctx.assets.register("workbench.backdrop" \| "mascot.idle", { kind, source })`. Anya plays it. Do not inject `<video>` into Anya DOM. |

Live ids: one `manage_plugin describe_contract` (`anchors`, `assetKeys`, `tabChrome`, `agent`, `icon`, `files`, `ui`, `layout`). Then stop.

**Frozen host.** New UI may only fill **listed** anchors / asset keys / `ctx.agent`. If a hole is missing, compose columns inside `workbench.main`. Do **not** `Grep` `src-tauri`, do not patch Anya source, do not invent anchors.

## Immediate execution

When the user asks to make or change a plugin:

1. If `load_skill` is available, you already have this playbook — do not also `run_skill` it as a subagent.
2. **Existing plugin:** `list` → `list_files` → `get_file` on the sources you will change. **Only** `manage_plugin` — never generic `read_file`/`Grep`/`run_shell` on AppData or the home directory.
3. **New plugin:** `describe_contract` once if you need ids. Then `create` with `id`, `name`, `description`, **and `files:[{path,contents}]` for every source including `ui/icon.svg` and `plugin.json` `"icon": "ui/icon.svg"`**. One tool call. `create` enables and loads the live workbench — do not `open` a window.
4. **Never ask the user to Enable, Reload, or restart Anya.** You own that: `create` auto-enables; `put_file` auto-reloads when already enabled; otherwise call `enable` / `reload` yourself.
5. If live load failed (safe mode / bundle error), fix and `enable`/`reload` again. Use `errors` only when the reload signal reported a fault.

### Do not

- Search AppData / the profile for plugin files with generic tools.
- Grep Anya `src/` / `src-tauri/` to verify the SDK — this file + `describe_contract`.
- Search other installed plugins for a hidden file API — `ctx.fs.pick` is the call.
- Patch Anya source to "make a plugin".
- Tell the user to Enable / Reload / restart Anya — that is your job via `manage_plugin`.
- Ship a plugin with only the Puzzle/letter default icon. Always write `ui/icon.svg`.
- Copy Alacritty / `node-pty` / a native sidecar. Interactive terminal = `pty.*` + xterm in `activate.js`.
- Edit `ui/.anya/activate.js` (generated). Write `ui/src/activate.js`.

## UI entry

| Path | Role |
| --- | --- |
| `ui/src/activate.js` (or `.ts`) | **You write this.** `export async function activate(ctx)`. |
| `ui/src/main.js` / `main.ts` | Alternate source names if `activate.*` is absent. |
| `plugin.json` `ui.entry` | Fallback path (scaffold default `ui/activate.js`). |
| `ui/activate.js` | Scaffold copy of the source. Prefer editing `ui/src/activate.js`. |
| `ui/.anya/activate.js` | **Generated** on Enable/reload (esbuild bundle). Workbench loads this when present. Never edit. |
| `ui/index.html` | Isolated plugin **window** only (`contributes.window`). Not the workbench `activate`. |
| `host/main.ts` | Deno host for tools/hooks. Not loaded in the workbench page. |

Resolve order for the bundle input (first file that exists): `ui/src/activate.ts` → `ui/src/activate.js` → `ui/src/main.ts` → `ui/src/main.js` → `ui.entry`.

## Surfaces

`sidebar.addTab({ surfaces })` chooses **where the button is**. It does **not** by itself pick the center chat column.

**Pane (where `mount(el)` actually shows):**

| User said | API | Where the panel is |
| --- | --- | --- |
| 主视图 / 中间 / 占掉对话 / 像点「插件」 | `surfaces: ["nav"]` and **do not** include `"views"`. Optional extra: `"header"`. | **Center** — replaces the conversation (same region as 插件 / 连接手机). Click the left nav item to open. |
| 主视图, open immediately on Enable | `ctx.workbench.setView({ id, title, mount })` + `"contributes": { "view": true }` | Same center region, exclusive, no extra click. |
| 工作区视图 / 右侧 / 和终端一样 | `surfaces: ["views"]` (default if omitted) | **Right review strip**, conversation stays. |
| Left shortcut **and** right review | `surfaces: ["nav", "views"]` | Nav is only a shortcut that **still opens the right review pane**. This is **not** 主视图. |
| 自己的页面，调用 Agent 流程 | 主视图 + `ctx.agent.run(prompt, { sessionId })` | Plugin owns the page. `run` calls the **same Rust agent loop** on a `plugin:<id>:…` session. Hidden from the conversation sidebar. Pass a stable `sessionId` to keep history. Read `ctx.stores.chat.sessions[sessionId]`. Do **not** `send` or `mount`. |
| 自己的页面，但仍用 Anya 对话壳 | 主视图 + `ctx.agent.mount(el)` | Plugin owns the page (compose 资料 \| Agent \| 成品 yourself). `mount` **embeds** Anya's live conversation into `el`. Aliases: `ctx.conversation.mount` / `send`. |
| Extra region the host does not list | Compose it inside `workbench.main` | **Never** add a host anchor or edit Anya `src/`. |
| 标题栏纯动作 / 只跑函数、不打开面板 | `surfaces: ["header"]` + `onClick`, **do not** include `"nav"` or `"views"` | **No pane**. Host draws the icon; click runs `onClick`. |

**Launcher-only keys** (button location):

| `surfaces` | Button |
| --- | --- |
| omitted | `["views"]` — tab on the review strip (and the pane is there). |
| `"views"` | Review-strip tab. |
| `"nav"` | Left nav, next to 插件 / 连接手机. Pane = center **unless** `"views"` is also set. |
| `"header"` | Conversation header icon. With `nav`/`views`, still opens that pane. **Only** `"header"` (no nav, no views) = chrome **action**: runs `onClick`, does not open a pane. |
| `[]` | No launcher (backdrop/mascot). |

Do **not** map 主视图 → `"views"`. `"views"` is the right-hand 工作区视图 (terminal lives there).

Header-only action (click runs a function, no pane):

```js
ctx.sidebar.addTab({
  id: "open-terminal",
  title: ctx.i18n.t("title", "Open in Terminal", { "zh-CN": "在终端中打开" }),
  icon: `anya-plugin://localhost/${ctx.pluginId}/ui/icon.svg`,
  surfaces: ["header"],
  onClick() {
    return ctx.workspace.openInTerminal();
  },
});
```

`plugin.json`: `"contributes": { "sidebar": true }`, `"icon": "ui/icon.svg"`. Do not add `"views"` or `"nav"`.

主视图 launcher (click like 插件):

```js
ctx.sidebar.addTab({
  id: "main",
  title: "Bid writer",
  icon: `anya-plugin://localhost/${ctx.pluginId}/ui/icon.svg`,
  surfaces: ["nav"],
  mount(el) { el.textContent = "hello"; return () => { el.textContent = ""; }; },
});
```

`plugin.json`: `"contributes": { "sidebar": true }`, `"icon": "ui/icon.svg"`. Do not add `"views"`.

`content: { header(el) {…}, views(el) {…} }` overrides `mount` per **launcher** surface; missing keys fall back to `mount`. There is no `content.main` — center content is either nav-without-views or `workbench.setView`.

**Settings do not go in chrome.** Every installed plugin already has a home page (Plugins list → the plugin): header + 详情 (`about` file) + 设置 (`ctx.home.setSettingsView`). Never `addTab` only to expose settings.

`plugin.json` `contributes` must match what `activate` actually calls: `sidebar` / `view` / `composer` / `window` / `agent`. Scaffold defaults `window: true` — override in the same `create` call.

## Plugin roles

A plugin is **not** always a sidebar. Set `"role"` in `plugin.json` (`ui` | `service` | `agent`). Omit to infer from `contributes`.

| role | What it is | Needs |
| --- | --- | --- |
| `ui` | Workbench chrome (sidebar, window, composer, header) | `ui/src/activate.js` + `ui.workbench` |
| `service` | Host / OS capability (CLI, daemon) | `host/main.ts`; UI optional |
| `agent` | Tools the **chat agent** should call | `host/main.ts` `describe()` tools. **No UI.** Do not add sidebar/window/`ui.workbench`. After Enable, tools are in the chat schema and a system note tells the agent to use them. Users can type `#plugin:<id>` (e.g. `#plugin:computer-use`) like `#skill` / `#mcp` to prefer those tools for the turn. |

Official: `terminal` = `ui`; `computer-use` = `agent`.

Agent-only example (`computer-use` shape): `plugin.json` `"role": "agent"`, `"contributes": { "agent": { "tools": true, "prompt": "…", "skills": ["skills/windows.md"] } }`, permissions `agent.tools` + `agent.prompt` (+ domain permission). No `activate.js`. `skills` markdown is appended to the system prompt so the agent does not explore the UI.

## Layout

- `plugin.json` — `ui.entry`, `"icon": "ui/icon.svg"` (**required**), optional `host.entry`, optional `about`, `contributes`, `permissions`
- `ui/icon.svg` — distinctive 48×48 mark (list, home, nav). Not a Lucide name. Not the letter-circle scaffold.
- `ui/src/activate.js` — workbench UI. Pure JS npm (`lodash`) OK. No `fs`, `node-pty`, `.node`.
- `ui/index.html` — isolated window only
- `host/main.ts` — Deno host. `net` → `import "npm:…"` (Anya user-dir Deno cache)
- `skills/*.md` — optional; list in `contributes.agent.skills`. Appended when `agent.prompt` is granted.

## Icon

Every plugin ships its own mark. `create` `files` **must** include `ui/icon.svg`.

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48">
  <rect width="48" height="48" rx="12" fill="#5b7cfa"/>
  <!-- shapes that still read at 16px: cat ears, document, film strip… -->
</svg>
```

`plugin.json`: `"icon": "ui/icon.svg"`.

`addTab({ icon })` may omit `icon` (host uses the file) or pass `anya-plugin://localhost/${ctx.pluginId}/ui/icon.svg`. Do **not** pass `"sparkles"` / `"terminal"` as the only icon unless the plugin is actually a terminal.

## Minimal sidebar tab

```js
export async function activate(ctx) {
  ctx.sidebar.addTab({
    id: "main",
    title: ctx.i18n.t("title", "My tool", { "zh-CN": "我的工具" }),
    icon: `anya-plugin://localhost/${ctx.pluginId}/ui/icon.svg`,
    surfaces: ["views"],
    mount(el) {
      el.textContent = "hello";
      return () => { el.textContent = ""; };
    },
  });
}
export function deactivate() {}
```

`plugin.json`: `"contributes": { "sidebar": true }`, `"icon": "ui/icon.svg"`, `"permissions": ["ui.workbench","storage"]`. Include `ui/icon.svg` in the same `create` `files`.

`mount` **must return a cleanup**. Do not open a new PTY every time the tab is shown.

## Backdrop / mascot (no tab)

```js
export async function activate(ctx) {
  const source = `anya-plugin://localhost/${ctx.pluginId}/ui/bg.webm`;
  ctx.assets.register("workbench.backdrop", { kind: "video", source });
  ctx.onDeactivate(() => ctx.assets.unregister("workbench.backdrop"));
}
```

Never inject `<video>` into Anya's DOM. Anya's container plays the asset and respects reduced motion.

### Backdrop plugin with a home-page settings tab

No `sidebar.addTab`. Configuration is `ctx.home.setSettingsView`. `mount` **must append real controls to `el`**. Use `ctx.fs.pick`, then `ctx.assets.register` with the returned `url`. Ship `about`.

```js
export async function activate(ctx) {
  const KEY = "workbench.backdrop";
  let source = `anya-plugin://localhost/${ctx.pluginId}/ui/bg.webm`;
  ctx.assets.register(KEY, { kind: "video", source });
  ctx.onDeactivate(() => ctx.assets.unregister(KEY));

  ctx.home.setSettingsView((el) => {
    const status = document.createElement("div");
    status.textContent = source;
    const pick = document.createElement("button");
    pick.textContent = ctx.i18n.t("pick", "Choose local file", { "zh-CN": "选择本地文件" });
    pick.onclick = async () => {
      const files = await ctx.fs.pick({
        filters: [{ name: "Media", extensions: ["webm", "mp4", "mov", "png", "jpg", "webp"] }],
      });
      const file = files?.[0];
      if (!file) return;
      source = file.url;
      status.textContent = file.name;
      ctx.assets.register(KEY, { kind: "video", source });
    };
    el.append(status, pick);
    return () => {
      el.replaceChildren();
    };
  });
}
```

`plugin.json`: `"about": "ui/about.md"`, `"permissions": ["ui.workbench","storage","fs.pick"]`. No `"contributes.sidebar"`.

**Any surface that renders nothing is a bug.** Building nodes on a detached `wrap` and forgetting `el.append(wrap)` is the same bug. Teardown undoes **this** view only — do not `home.clearSettingsView()` / `slots.unmount` from mount teardown (switching 详情/设置 would unregister the view).

## Look and feel

Anya keeps WebView2 `color-scheme: only light` (darkness is CSS variables). `describe_contract.ui` lists tokens.

| Control | Do this |
| --- | --- |
| Choices (毛色 / 模式) | Row of `<button>`s. Selected = `background: var(--peek-surface); color: var(--peek-text)`. |
| Native `<select>` | Avoid. OS-white popup + `color: inherit` is unreadable. If you must: `option { background:#fff; color:#111 }`. |
| Text | `color: var(--peek-text); background: var(--peek-surface); border: 1px solid var(--peek-border)`. |
| Checkbox / range | `accent-color: var(--peek-accent)`. |

Tokens: `--peek-text` `--peek-muted` `--peek-surface` `--peek-bg` `--peek-border` `--peek-accent`. Inline `background:transparent;color:inherit` on `<select>` overrides the host baseline and breaks the dropdown.

## Workbench SDK

- `sidebar.addTab({ id, title, icon, mount(el)?, onClick?, surfaces?, content? })`
- `home.setSettingsView(mount(el))` / `home.clearSettingsView()`
- `workbench.setView({ id, title, mount(el) })` — exclusive; check you won
- `agent.run(prompt, { sessionId?, cwd? })` — same Rust agent loop on a plugin-owned session (`plugin:<pluginId>:…`). Does **not** write the open workbench chat. Omit `sessionId` to mint one; pass a stable id to keep history. `cwd` binds tools to that folder (does not switch the workbench workspace). Returns `{ sessionId }`. Permission `ask_anya`.
- `agent.mount(el)` / `unmount()` — embed the live agent conversation into the plugin's own page; returns cleanup
- `agent.send(text)` — inject into the current workbench conversation (same path as the composer)
- `agent.sessionId()` — current workbench session id (`mount`/`send`, not `run`)
- `workspace.openInTerminal(id?)` — OS terminal at a workspace (Windows: `wt.exe -d <cwd>`). Header-only tabs use this from `onClick`.
- `conversation.*` — aliases of `agent` (`addMaterials` is compat-only; do not use for new plugins)
- `composer.addAccessory({ id, mount(el) })`
- `ctx.slots.mount(anchorId, view)` → `false` if an exclusive anchor is taken
- `ctx.assets.register(key, { kind, source })` → `false` if another plugin already owns that exclusive key (same as `slots.mount`)
- `ctx.bus.publish` / `subscribe` (max 200 subscriptions / plugin)
- `ctx.i18n.t(key, fallback, dict?)`
- `ctx.host.rpc` / `ctx.host.on` — `on` returns an unsubscribe; call it from mount cleanup
- `ctx.fs.pick({ multiple?, directory?, filters?: [{ name, extensions }] })` → `{ path, name, url }[] | null`
- Isolated window: `AnyaPlugin.pick` / `storageGet` / `storageSet`
- `onDeactivate(fn)`
- Workbench `ctx.storage` **does not exist**. Persist with `localStorage` (or the isolated-window storage APIs).

Same-heap UI can read `stores.chat` / `stores.setting`. Warn the user.

## Files

| Need | Do this |
| --- | --- |
| User picks a local file | `ctx.fs.pick({ filters })` + permission `fs.pick`. Use **`url`** as asset/`<img>`/`<video>` source. Cancel → `null`. |
| Isolated window | `AnyaPlugin.pick(same options)` |
| File shipped in the plugin | `anya-plugin://localhost/${ctx.pluginId}/ui/…` |
| Workspace from `host/main.ts` | permission `fs.workspace`, then `Deno.readTextFile` / `writeTextFile`. **Not** a `ctx` method. |
| Folder pick | `ctx.fs.pick({ directory: true })` — `path`/`name` only; **`url` is empty**. |
| Share / install a plugin pack | `manage_plugin import` (`path` to zip or folder, optional `overwrite`) / `export` (`id`, optional dest `path`). Does not enable. Cannot overwrite official plugins (`terminal`, `computer-use`). Users can also Import/Export in the plugins list. |

Not provided: `ctx.fs.read` / `write` / `stat`; native path from `<input type="file">`; any other host RPC for files.

Picked files are copied into `data/picked/` (max 256 MiB). `url` is a stable `anya-plugin://` snapshot.

### PTY (needs `pty` permission)

```
pty.open  { cols, rows, cwd? } → { session }
pty.write { session, data }
pty.resize { session, cols, rows }
pty.kill  { session }
host.on("pty.data", ev => { /* ev.session, ev.data */ })
```

Open **once** in `mount`, kill in the returned cleanup / `onDeactivate`.

### Computer use (needs `computer` permission)

Anya implements these — do not shell out. Prefer `launch` → `key` → `click_control` / `set_value` (UIA) → pixel click only for canvas. Screenshot defaults to the **foreground window** and appends an interactive control list. Pixel click/move/scroll x/y are in the **last screenshot** (origin top-left).

```
computer.screenshot   { scope?, title? } → { content }  // JPEG + UIA overlay
computer.screenInfo   {}
computer.listWindows  {}
computer.focusWindow  { title? , id? }
computer.findControl  { name?, id?, windowTitle? }
computer.clickControl { name?, id?, index? }
computer.setValue     { value, name?, id?, index? }
computer.launch       { target, args?, ms? }
computer.click        { x, y, button?, count?, modifiers? }
computer.drag         { x,y,x2,y2 | path | cx,cy,r }
computer.move         { x, y }
computer.scroll       { x, y, dy?, dx? }
computer.type         { text }
computer.key          { keys }           // enter, tab, win+e, ctrl+c
```

Workbench: `ctx.host.rpc("computer.screenshot")` if the plugin has UI. Official `computer-use` is `role: agent` — **no UI**; after Enable the chat agent calls `plugin_computer-use__launch` / `click_control` / `screenshot`. Windows only. Playbook: `contributes.agent.skills`.

## Agent tools / hooks

`agent.tools` → `host/main.ts` `describe` → `{ tools, hooks }`. Names are `plugin_<id>__<name>`. Cannot overlay core tools, approval, sandbox, or Plan. Hooks may **deny** a call; they cannot skip Anya gates.

## Permissions

`storage` `ask_anya` `pty` `run` `fs.workspace` `fs.pick` `net` `agent.tools` `agent.hooks` `agent.prompt` `ui.workbench` `computer`

Optional `capabilities` (do not invent ids — `describe_contract`):

```json
"capabilities": [
  { "id": "net.listen", "scope": { "host": "127.0.0.1", "portRange": [40010, 40010] } }
]
```

Known: `net.listen`, `net.connect` (loopback, ports 1024–65535), `fs.scope` (`{ "root": "<relative path>" }`).

## Uninstall / safe mode

Uninstall stops the host, unhooks tools, and **deletes the plugin folder**. Official plugins (`terminal`, `computer-use`) cannot be uninstalled. Safe mode skips **all** plugins at startup. Do not fight that by writing Anya source.
