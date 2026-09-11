# Windows apps playbook

Operate with the hybrid pipeline. Do **not** screenshot-tour Start or learn an app from pixels.

## Priority (always)

1. **`launch`** — app, file, or URI (`mspaint`, `notepad`, `C:\doc.docx`, `ms-settings:display`). One call. Do not Win+R unless launch fails.
2. **`key`** — `ctrl+s`, `ctrl+z`, `alt+f4`, `enter`, `esc`, `tab`, `win+e`, `win+i`.
3. **UIA** — `screenshot` returns a control list. `click_control` `index` / `name` / `id` (Invoke). `set_value` for named edits. `find_control` if the list truncated.
4. **Pixels** — `click` / `drag` only for canvas, unmarked regions, or snip rectangles.

`find_control` / `click_control`: try **both** Chinese and English labels (locale is often zh-CN).

## Speed

- After `launch`, continue in the **same** turn (it already waits ~200ms). No hover. No screenshot between steps.
- Screenshot **once** when you need canvas coordinates or the UIA list. Then emit **all** remaining tools in that turn.
- If a click result says foreground looks like Anya, `focus_window` the target — do not keep pixel-clicking.
- Save/Open: `ctrl+s` / `ctrl+o`, `type` the full path, `enter`. Or `set_value` on the filename box.

## Launch table

| App | `launch` target | Also |
| --- | --- | --- |
| Paint 画图 | `mspaint` | |
| Notepad 记事本 | `notepad` | `launch` `{target:"notepad", args:"C:\\a.txt"}` |
| Word | `winword.exe` | args = document path |
| Explorer 资源管理器 | `explorer` | `key` `win+e`; args = folder path |
| Settings 设置 | `ms-settings:` | `ms-settings:display`, `network`, `bluetooth` |
| Calculator 计算器 | `calc` | |
| Snipping 截图 | | `key` `win+shift+s` then one `drag` |
| Task Manager 任务管理器 | `taskmgr` | `key` `ctrl+shift+esc` |
| Cmd / PowerShell | `cmd` / `powershell` | |
| Close app | | `key` `alt+f4` |
| Copy/paste/undo | | `ctrl+c` / `ctrl+v` / `ctrl+z` |

## Paint 画图 (`launch` `mspaint`)

Window title contains `Paint` or `画图`. White **canvas** is below the ribbon — do not click Anya.

- Undo `ctrl+z`. Save `ctrl+s`. New `ctrl+n`.
- Tools: `click_control` `Pencil`/`铅笔`, `Brushes`/`画笔`, `Eraser`/`橡皮擦`, `Shapes`/`形状`, `Text`/`文本`, `Fill`/`填充` from the UIA list. Screenshot **once** for canvas bounds, then **all** `drag`s.
- `drag` `x,y`+`x2,y2` = straight stroke. Circles/arcs: `cx,cy,r`. Free curves: `path` with 8+ points.
- Shape tools: one bounding-box `drag`, not a brush circle.
- Color: `click_control` a swatch (`Colors`/`颜色`) then draw.

## Notepad 记事本 (`launch` `notepad`)

Caret is in the editor. **`type`.** No click, no screenshot. Save `ctrl+s`. Find `ctrl+f`.

## Explorer (`launch` `explorer` or `win+e`)

- Address: `ctrl+l`, `type` path, `enter`. Or `launch` `{target:"explorer", args:"C:\\Users"}`.
- Search `ctrl+f`. Parent `alt+up`. New folder `ctrl+shift+n`. Rename `f2`.

## Settings (`launch` `ms-settings:…` or `win+i`)

Do not click Settings home tiles.

## Calculator (`launch` `calc`)

Type digits and operators. Do not click the keypad.

## Common dialogs

- Filename box: `set_value` or `type` a full path, `enter`. Overwrite: `enter` on Yes/`是`.
- **UAC**: do not click. Tell the user to approve.
- Wizards: `click_control` `Next`/`下一步`, `Install`/`安装`, `Finish`/`完成`.
