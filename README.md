# Anya

<p align="center">
  <img src="src-tauri/icons/icon.png" alt="Anya" width="120" height="120" />
</p>

<h1 align="center">Anya</h1>

<p align="center"><strong>A desktop agent you can summon anytime.</strong></p>

<p align="center">
  Press a shortcut, and Anya is there — ready to help with documents, code, and everyday work.<br />
  DeepSeek is first-class; more providers plug in when you need them.
</p>

<p align="center">
  <a href="./README.md">English</a>
  &nbsp;·&nbsp;
  <a href="./README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <img alt="platform" src="https://img.shields.io/badge/Windows-10%20%2F%2011-0078D4?style=flat-square" />
  <img alt="release" src="https://img.shields.io/badge/version-v0.2.8-4D6BFE?style=flat-square" />
  <img alt="license" src="https://img.shields.io/badge/license-Unlicense-3DA639?style=flat-square" />
  <img alt="stack" src="https://img.shields.io/badge/Tauri%202%20%2B%20Vue%203%20%2B%20Rust-black?style=flat-square" />
</p>

---

## At a glance

|                 |                                                                              |
| --------------- | ---------------------------------------------------------------------------- |
| **Overlay**     | Double-tap <kbd>Alt</kbd> from any app. Ask, attach context, keep going.     |
| **Workbench**   | Full desktop UI for pinned chats, project workspaces, and review.            |
| **Agent**       | Ask / Agent / Plan; tools, Skills, MCP, Office; complex tasks may auto-plan. |
| **Local-first** | Keys, history, and settings stay on your machine by default.                 |

**Docs:** [Architecture](./docs/architecture-overview.md) · [Releases](./docs/release.md) · [Index](./docs/README.md)

---

## Overlay — ask from anywhere

Double-tap <kbd>Alt</kbd> to show or hide the floating window. Ask a question, follow up in place, and switch Agent / model / approval under the composer.

<p align="center">
  <img src="./docs/image/Alt%2BAlt.png" alt="Anya floating overlay" width="560" />
</p>

Anya can pick up the current text selection or Explorer selection. Paste or drag images and files into the input when you need richer context.

<p align="center">
  <img src="./docs/image/select_text_recognition.webp" alt="Selected text brought into Anya" width="800" />
</p>

<p align="center">
  <img src="./docs/image/select_image_recognition.webp" alt="Selected image attached in Anya" width="800" />
</p>

Summoning outside an IDE starts a **Quick Ask** session — not bound to a workspace, so history does not land in the wrong project. Bind a folder only when you choose one in the overlay (or with `/work`), or when you trigger from an IDE that is actually in the foreground.

Need more room? Use **Open conversation in workbench** on the overlay to move that same session into the full desktop UI — progress, tools, and history continue there.

### IDE context plugins

Companion plugins push active file, workspace, language, and selection to the local Anya app (best-effort; the editor keeps working if Anya is not running).

- [Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=Anya.anya-ide-context)
- [IntelliJ Platform](https://plugins.jetbrains.com/plugin/33163-anya-ide-context)

---

## Workbench — every session in one place

The workbench is the full desktop surface: Quick Ask threads from the overlay sit beside pinned chats and project workspaces.

<p align="center">
  <img src="./docs/image/workspace.png" alt="Anya workbench" width="900" />
</p>

| Area           | What it is for                                                                                                          |
| -------------- | ----------------------------------------------------------------------------------------------------------------------- |
| **Pinned**     | Keep important threads at the top.                                                                                      |
| **Workspaces** | Bind chats to a project folder so Agent edits stay in context.                                                          |
| **Quick Ask**  | Temporary overlay sessions — continue, start new, or keep a long run here while you still summon the overlay elsewhere. |

### Review changes

When Agent edits files, Anya shows a per-file summary and a focused Diff view.

<p align="center">
  <img src="./docs/image/workspace-diff.png" alt="Diff review in Anya" width="900" />
</p>

- Task list and verification stay on the conversation timeline.
- Open **Review** for side-by-side or unified diffs.
- Undo covers changes Anya applied in the current session (checkpoints).

### Settings

Configure models, providers, agent behavior, and extensions from the embedded settings page.

<p align="center">
  <img src="./docs/image/workspace-settings.png" alt="Anya settings" width="900" />
</p>

Common controls include default chat model, vision / multimodal fallback, reasoning effort and language, tool approval mode, Agent display density, and context-window budget.

---

## Capabilities

### Ask / Agent / Plan

| Mode      | Intent                              | Typical tools / constraints                                                        |
| --------- | ----------------------------------- | ---------------------------------------------------------------------------------- |
| **Ask**   | Read-only investigation             | Files, search, LSP, other read-only tools                                          |
| **Agent** | Default; change the world carefully | Files, PowerShell, Git, Skills, MCP, sub-agents; complex tasks may auto-enter Plan |
| **Plan**  | Agree on steps before writes        | Write tools locked; `update_tasks` + end-of-message approval card                  |

Ask withholds write / shell / git. Agent enables them under your approval policy. Plan (manual or auto) blocks writes via the plan gate until you approve at the end of the assistant reply. All three share the same `AgentRunner` loop — policy lives in tool exposure, approval, and the plan gate, not a second orchestrator.

### Timeline

Assistant turns interleave **reasoning**, **reply text**, and **tool activity** in chronological order — live and after reload. Long thinking no longer hides the work that happened mid-thought.

### Integrations

| Integration            | Role                                                                                               |
| ---------------------- | -------------------------------------------------------------------------------------------------- |
| **Microsoft Office**   | Context and `word_*` / `excel_*` / `ppt_*` tools when Word, Excel, or PowerPoint is running (COM)  |
| **Skills**             | Built-in and vendor playbooks (docx, pandoc, research, review, bid tech, …); can run as sub-agents |
| **MCP**                | Stdio and remote MCP servers                                                                       |
| **LSP**                | Diagnostics when configured                                                                        |
| **Pinned-image badge** | Optional PixPin / Snipaste badge to open a chat with that image                                    |
| **Sub-agents**         | Split larger work while progress stays visible on the main thread                                  |
| **Memory**             | Local memory tools; optional mem0 cloud sync                                                       |
| **Web search**         | Serper or Tavily when an API key is set                                                            |

### Model providers

| Provider     | How you connect                                                                              |
| ------------ | -------------------------------------------------------------------------------------------- |
| **DeepSeek** | API key — recommended default                                                                |
| **Gemini**   | Google sign-in (Antigravity OAuth)                                                           |
| **Custom**   | OpenAI-compatible Base URL + key; presets for MiMo, Zhipu GLM, Volcengine Ark, MiniMax, Kimi |

For image input with a text-only primary model, set a vision model or enable multimodal split analysis in Settings. The composer shows session token estimates and context usage; you can change model and thinking level while tools run.

---

## Install and get started

1. Download the MSI from [Releases](../../releases) and install.
2. Open **Settings** from the tray icon and connect a model provider.
3. Double-tap <kbd>Alt</kbd>, ask a question, press <kbd>Enter</kbd> — or move the session to the workbench when you need the full UI.

| Shortcut                                            | Action                                             |
| --------------------------------------------------- | -------------------------------------------------- |
| Double-tap <kbd>Alt</kbd>                           | Show or hide the overlay                           |
| <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Space</kbd> | Fallback summon shortcut                           |
| <kbd>Enter</kbd>                                    | Send                                               |
| <kbd>/</kbd>                                        | Slash commands                                     |
| <kbd>Esc</kbd>                                      | Clear input; may close the window in some contexts |

---

## Data and privacy

API keys, OAuth tokens, settings, and chat history stay on your machine by default. Context capture is local; the message and attached context leave the device only when you send them to your configured provider.

Web search, MCP, and mem0 cloud sync send data to those services — enable them only if you accept their policies.

Crash recovery uses a local SQLite journal so interrupted streaming turns can settle on next launch instead of leaving the UI stuck “executing”.

---

## Architecture (summary)

Anya is a single-process **Tauri 2** app: WebView2 (Vue 3 + Pinia) for presentation, and a Rust host for OS integration, chat domain logic, model I/O, and tools.

```mermaid
flowchart TB
  subgraph Surfaces["Window surfaces"]
    WB[Workbench]
    OV[Overlay]
    ST[Settings]
    PV[Image preview]
  end

  subgraph Host["Anya.exe — Rust host"]
    CMD[commands / EventBus]
    CHAT[ChatService · StreamManager · AgentRunner]
    TOOLS[ToolRegistry · plan gate · Skills · MCP · Office]
    STORE[(SQLite + journal)]
  end

  subgraph External["External"]
    LLM[Model providers]
    IDE[IDE plugins]
    OFFICE[Word / Excel / PPT]
  end

  WB & OV & ST & PV <-->|IPC invoke + events| CMD
  CMD --> CHAT
  CHAT --> TOOLS
  CHAT --> STORE
  CHAT -->|HTTPS stream| LLM
  IDE -->|context push| Host
  TOOLS -->|COM| OFFICE
```

Primary path:

```text
invoke("chat")
  → ChatService::send
  → StreamManager / AgentRuntime
  → AgentRunner::run
  → AIProvider::stream + ToolRegistry
  → EventBus → UI
  → ConversationManager persists messages (+ work_timeline)
```

`AgentRunner` owns the model↔tools loop. `AgentRuntime` owns run lifecycle (cancel, soft-inject, debug). Ask / Agent / Plan share that loop; Plan gates writes via `plan_mode` until end-of-message approval. Do not add a second chat loop beside `AgentRunner`.

Full diagrams: [Architecture overview](./docs/architecture-overview.md) · [简体中文](./docs/architecture-overview.zh-CN.md)

---

## Run from source

Requires Node.js 18+, pnpm, Rust stable, VS C++ Build Tools, and WebView2.

```bash
pnpm install
pnpm tauri:dev
```

```bash
pnpm check          # typecheck + lint + frontend tests
cd src-tauri && cargo test --lib
pnpm tauri:build
```

The installer lands at `src-tauri/target/release/bundle/msi/Anya_0.2.8_x64.msi`.

For signing, `latest.json`, and GitHub Releases, see [Releases and remote updates](./docs/release.md).

---

## License

This repository is dedicated to the public domain under the [Unlicense](./LICENSE).
