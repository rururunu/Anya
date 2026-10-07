<p align="center">
  <img src="./public/mascot/mascot-icon.svg" alt="Anya" width="96" height="96" />
</p>

<h1 align="center">Anya</h1>

<p align="center"><strong>Your Windows AI assistant, one shortcut away</strong></p>
<p align="center">Questions · Code · Documents</p>

<p align="center">
  English &nbsp;·&nbsp; <a href="./README.zh-CN.md">简体中文</a>
  &nbsp;·&nbsp; <a href="../../releases">Download</a>
  &nbsp;·&nbsp; <a href="./docs/README.md">Documentation</a>
</p>

<p align="center"><sub>Windows 10 / 11 &nbsp;·&nbsp; v0.2.27 &nbsp;·&nbsp; MIT</sub></p>

---

## What it does

| Feature                | What you can do                                                                                      |
| ---------------------- | ---------------------------------------------------------------------------------------------------- |
| **Ask anywhere**       | Double-tap Alt to open the overlay with files or images.                                             |
| **Work on projects**   | Organize workspaces and chats, run tasks, and review changes.                                        |
| **Personalize Anya**   | Set up a local profile and avatar, review usage activity, and import or export profile information.  |
| **Handle documents**   | Generate, read, convert, and render Word / Excel / PowerPoint files; deliver them as cards.          |
| **Choose your model**  | Configure DeepSeek, OpenAI-compatible, or Anthropic-compatible providers.                            |
| **Extend your tools**  | Use skills, MCP, and plugins; manually enable computer use and OpenCLI.                              |
| **Connect your phone** | Chat, approve actions, and transfer files with [Companion](https://github.com/rururunu/AnyaAndroid). |

DeepSeek models use natively integrated dsh tools and prompts, with cache usage, response timing, and retry diagnostics. Other models use Anya's general tool set. See [DeepSeek integration](./docs/deepseek-harness.md).

## Get started

Alt+Alt captures selected text before the overlay takes focus and displays a compact inline reference tag. Chrome / Edge users can optionally install the [Selection Bridge](./browser-extension/README.md) for webpage selection and source URLs.

The **Profile** settings page shows local usage insights and lets you transfer your profile, settings, and API keys. Treat exported files as sensitive. A factory reset is available from the profile actions.

The chat composer keeps model and thinking controls on the right; on narrow windows, controls collapse to icons. Editing a sent message uses the same controls. When an update is available, open it from the workbench sidebar or **Settings → About** to review the release notes before installing.

1. Download and install the MSI from [Releases](../../releases).
2. Open **Settings**, add a model provider, and configure its credentials.
3. Start a chat in the workbench, or double-tap **Alt** to open the overlay.

Fallback shortcut: <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Space</kbd>.

| Mode  | Purpose                                                     |
| ----- | ----------------------------------------------------------- |
| Ask   | Read-only questions and research                            |
| Agent | Complete tasks with tools under your approval policy        |
| Plan  | Prepare a plan before approved write operations             |
| Image | Generate images with a separately configured image provider |

Office layout, conversion, and formula results need checks for each artifact. Some search and extension features require additional setup. See [Office workflows](./docs/office.md) and [plugins](./docs/plugin-system.md).

## Data and privacy

Settings, credentials, and chat history stay on your machine by default. Sending a message sends its text and attached context to your configured model provider. Search, remote MCP, cloud sync, and phone connections also send relevant data to their services or devices when enabled.

Custom background images are stored as local files; runtime settings keep their paths. Readable images are included when you export a portable profile.

## Development

Requires Node.js 18+, pnpm, Rust stable, Visual Studio C++ Build Tools, and WebView2.

```bash
pnpm install
pnpm tauri:dev
```

<details>
<summary>Check and build commands</summary>

```bash
pnpm check
cd src-tauri
cargo test --lib
cd ..
pnpm tauri:build
```

</details>

Built with **Tauri 2 + Vue 3 + TypeScript + Rust**.

## Documentation

- [Documentation index](./docs/README.md)
- [Architecture](./docs/architecture-overview.md)
- [DeepSeek integration](./docs/deepseek-harness.md)
- [Office workflows](./docs/office.md)
- [File delivery](./docs/file-delivery.md)
- [Browser Selection Bridge](./browser-extension/README.md)
- [Plugin development](./docs/plugin-system.md)
- [Releases and updates](./docs/release.md)

IDE context plugins: [VS Code](https://marketplace.visualstudio.com/items?itemName=Anya.anya-ide-context) · [IntelliJ Platform](https://plugins.jetbrains.com/plugin/33163-anya-ide-context).

## License and acknowledgments

Licensed under [MIT](./LICENSE). Third-party notices are in [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md).

Thanks to [Ghost](https://github.com/NORTHTEKDevs/ghost), [OpenCLI](https://github.com/jackwener/OpenCLI), and the open-source tools and artwork used in Anya.
