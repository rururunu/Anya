# Windows apps playbook (Ghost engine)

Operate with **see → act → verify**. Do **not** screenshot-hunt Start or guess pixels for named controls.

## Learned playbooks

Before guessing through an unfamiliar app workflow, call `playbook` `{op:"lookup", query:"…", app/exe?}`.

- Hit → follow steps; each step: check `precondition` / `see` / `assert`, then act; read **`verified`**.
- Miss → explore normally.
- Novel success → `playbook` `{op:"save", intent, app, exe?, steps:[{kind,name,role,…}], success_assert?}`. Prefer name/role/keys — **never** pixel coords.
- Step fails → `playbook` `{op:"fail", id, step, reason}` then explore; when fixed, `save` with same `id` to patch.
- Replay worked → `playbook` `{op:"success", id, version?}`.

Low-confidence playbooks are quarantined automatically.

## Priority (always)

1. **`window`** — `launch` / `anchor`. `focus` anchors only (`raise:true` to steal focus).
2. **`see` / `find`** — list or locate controls. Chromium + DevTools → automatic CDP (`dispatch=cdp`). Prefer `find` before spraying acts.
3. **`act`** — name/role (background or CDP). `description=` → ground then real click. Read **`verified`** / `route`.
4. **`wait` / `assert`** — scoped to the anchored window.
5. **`key` / `hotkey`** — single keys + Ctrl+C/X/V/A/Z background; other chords auto-raise target.
6. **Canvas** — `drag` (real mouse). `click real=true` for a tap. Posted clicks do not mark Paint.
7. **Browser** — prefer CDP auto-route or OpenCLI; else `browser` + `tab`. Never pixel-click web pages.

**Focus / lock:** Anya unlocks Ghost and defaults to **`background`** (Ghost MCP parity). `policy=background` is **normal** — not a lock. Never ask for `setx GHOST_FOCUS_LOCK`. Always pass `window=` or rely on the session anchor after launch.

## Speed

- After `window` launch/focus, continue in the **same** turn.
- One `see`, then emit **all** remaining `act`/`key` calls together.
- If `verified` is false: `see` again or `wait for=element`, do not spray clicks.
- Save/Open: `key` `ctrl+s` / `ctrl+o`, `act` type into the filename field, `key` `enter`.

## Launch table

| App | `window` launch `exe` / `target` | Also |
| --- | --- | --- |
| Paint 画图 | `mspaint` | |
| Notepad 记事本 | `notepad` | args = file path |
| Word | `winword.exe` | args = document path |
| Explorer 资源管理器 | `explorer` | `key` `win+e` |
| Settings 设置 | `ms-settings:display` | |
| Calculator 计算器 | `calc` | |
| Edge / Chrome | `browser` op=launch which=edge\|chrome | then `tab` |
| Close app | | `key` `alt+f4` |
| Copy/paste/undo | | `ctrl+c` / `ctrl+v` / `ctrl+z` |

## Notepad 记事本

1. `window` `{op:"launch", exe:"notepad"}` (anchors).
2. `act` `{action:"type", name:"Text Editor", text_input:"…"}` or type into the edit role.
3. Save: `key` `ctrl+s`.

No screenshot. Prefer background act (default Ghost policy).

## Explorer

- Address: `key` `ctrl+l`, `act` type path, `key` `enter`. Or launch with folder args.
- Search `ctrl+f`. Parent `alt+up`. New folder `ctrl+shift+n`.

## Paint 画图

- Ribbon tools: `see` then `act` by name (`Pencil`/`铅笔`, `Brushes`/`画笔`, …). Windowless ribbon buttons may briefly raise Paint (Anya auto-unlocks).
- **Brush strokes:** use `drag` with `window=` (or after launch/anchor). Drag uses **real mouse** — Anya briefly raises Paint, strokes, restores background. Do **not** use posted `click` for canvas marks (Paint ignores them).
- One-off canvas tap: `click` with `real=true` and `window=`.
- Prefer straight `from_*` / `to_*` segments; chain several `drag` calls for curves.

## Browser (CDP)

Prefer the **OpenCLI** plugin (`plugin_opencli__*`) for logged-in Chrome and known site adapters. Use Ghost CDP below only when OpenCLI is unavailable or you need a separate automation profile.

1. `browser` `{op:"launch", which:"edge", mode:"windowed"}` **or** `attach` to an already-logged-in profile with `--remote-debugging-port`.
2. `tab` `{op:"open", url:"https://…"}` → keep `target_id`.
3. Drive with `tab` navigate / click / type / text / eval / wait (CSS selectors).
4. Do **not** use desktop `act`/`click` on web content.
5. Do **not** drive the same Chrome with OpenCLI and Ghost CDP in one turn.

## Common dialogs

- Filename: `act` type full path, `key` `enter`. Overwrite: Yes/`是`.
- **UAC**: do not click. Tell the user to approve.
- Wizards: `act` `Next`/`下一步`, `Install`/`安装`, `Finish`/`完成`.

## Stop

HUD **结束** / chat end stops Ghost (emergency stop + clear window anchor).
