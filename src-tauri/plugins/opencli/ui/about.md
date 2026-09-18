# OpenCLI

Agent plugin. After you **Enable** it (grant `opencli`), ask Anya in chat to use site adapters or drive your **logged-in Chrome** through [OpenCLI](https://github.com/jackwener/OpenCLI) (Apache-2.0).

**Before enable**, confirm you have:

1. Node.js ≥ 20 — or use **Settings → 一键安装 CLI** inside this plugin page  
2. The OpenCLI **Browser Bridge** Chrome extension  
3. A green `opencli doctor` (also available in Settings)

The agent can:

1. **`doctor` / `list`** — check setup and discover adapters  
2. **`run`** — deterministic site commands (Bilibili, Zhihu, Hacker News, …)  
3. **`browser`** — open / state / click / type / fill / extract on a named session  

Prefer OpenCLI for logged-in web tasks. Prefer **Computer Use** for desktop UI. Do not drive the same Chrome with both stacks in one turn.

Mutating tools (`run`, `browser`) use Anya's tool-approval picker. The plugin ships **disabled**.
