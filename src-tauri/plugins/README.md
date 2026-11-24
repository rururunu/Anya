# Official plugins

Source of truth for plugins Anya copies into the user plugins directory on
launch (`%APPDATA%\Anya\plugins\` / `Anya Debug\plugins\`).

| Folder | Id | What it is |
| --- | --- | --- |
| [`opencli/`](./opencli/) | `opencli` | Official agent plugin. Copied on launch. **Off until Enable.** |
| [`examples/`](./examples/) | — | Reference plugins, not auto-enabled. |

`terminal/` and `computer-use/` stay in this tree, but those ids are reserved.
Anya does not copy or load them. The workbench terminal and Windows computer use
are built into the app. See [computer use](../../docs/computer-use.md).

Edit `opencli/` here, then restart Anya. `ensure_bundled_plugins` overwrites the
AppData copy from this source.

The built-in terminal inherits the Windows user environment. It does not prepend
plugin `bin/` directories. A plugin-owned PTY session still does.

Pin-window AI badge (PixPin / Snipaste) is not a plugin. Optional local API while Anya is running:

```text
POST http://127.0.0.1:18480/api/ask/image
{"image":"data:image/png;base64,..."}
```
