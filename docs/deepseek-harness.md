# Native DeepSeek dsh path

[简体中文](./deepseek-harness.zh-CN.md)

DeepSeek models select an independent dsh tool registry at the Rust agent entry point. Other models retain Anya's existing registry and prompts. Each child agent selects its contract from its own resolved model. This is source code implementation; it needs no Anya plugin and does not run or package a Node harness.

Schemas, descriptions, tool guidance, section ordering and the default identity are imported from the local `deepseek-harness` checkout, currently revision `639ed015397290b3745d163aafe02ffee4aa3f84`. Snapshots, provenance and the MIT license live in `src-tauri/prompts/dsh/`; native executors live in `src-tauri/src/core/tools/dsh/`.

## Tools and modes

The core profile contains `read`, `read_image`, `write`, `edit`, `glob`, `grep`, `pwsh` on Windows or `bash`, `job_output`, `job_list`, `job_kill`, `todo_write`, `ask_user_question`, `skill`, `web_search`, `web_fetch`, `subagent`, `present` and `exit_plan_mode`. Tools without a corresponding host capability are omitted. Enabled Anya plugin tools (`plugin_*`), MCP tools (`mcp__*`), and `manage_plugin` retain their own schemas and executors in the DeepSeek tool list; disabled or unavailable tools remain hidden.

Image chat mode passes only Anya's `generate_image` tool to DeepSeek. The dsh adapter preserves its schema and executor, and the DeepSeek prompt includes the selected image size, quality, count and style instructions. Image generation still requires a configured provider under Settings → Image.

### Compared with the standard Agent

These differences follow the current source registries and prompts; they do not by themselves mean a plugin is uninstalled. DeepSeek uses the fixed dsh core contract, while other models use Anya's full built-in registry.

| Capability          | Standard Agent                                                                         | DeepSeek dsh                                                                                                       |
| ------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Files and shell     | Anya's granular file tools, `apply_patch`, commands and task tools                     | Core `read/write/edit/glob/grep/pwsh` tools; common work remains possible without every equivalent named operation |
| Code navigation     | Symbol, LSP, code index, Git and workspace tools                                       | Primarily search, read and shell; no corresponding dedicated tools                                                 |
| Skills and subtasks | Skill installation/management, exploration/review/document tools and parallel subtasks | `skill` loads enabled skills, plus one `subagent` tool; no equivalent management or parallel tool                  |
| Host helpers        | Memory management, chat history operations, theme management and Companion tools       | Not part of the core contract; available plugins, MCP tools and `manage_plugin` are added separately               |
| Plugin extensions   | Plugin prompts and before/after tool hooks                                             | Plugin/MCP tools can run, but plugin prompts and before/after hooks are not applied                                |
| Optional policies   | Multi-model collaboration, minimal coding and Companion-origin prompts                 | These standard Agent policies are not currently injected; image mode injects its own policy                        |

The actual list also depends on mode, permissions and tool availability. Image mode deliberately narrows the list to image generation, so it cannot be used to infer plugin availability in Agent mode.

### File delivery cards

The `present` name, description, input schema and delivery guidance are imported from upstream. Its native Rust executor verifies paths and regular files, with at most eight files per call. Anya's general tool registry also exposes `present`; local cards do not start the Companion gateway. Successful tool declarations produce cards, without scanning closing prose, listing every source edit, or copying file contents.

Structured metadata is persisted with assistant tool activities and restored from history. It remains valid JSON beyond the ordinary tool-output budget, with a separate 1 MiB ceiling. Repeated absolute paths retain the latest declaration, and more than four cards collapse. Child-agent declarations do not spill into the parent reply; the parent should declare final deliveries itself.

Cards use Anya theme tokens, icons, borders and corners, with a narrow-window layout. Raster images use the existing image sidebar; other files open in the default application. A separate action reveals the file in its folder, with retryable errors. Upstream Office/PDF sidebar previews and application-selection menus are not included. Sizes reflect delivery time; opening uses the current file, which may have been moved, deleted or changed.

Host guidance lists the tools exposed for the current turn and requires capability descriptions to include `read_image`. Child agents use a separate context for intermediate work, but their returned results consume main-conversation tokens. Skills come from the current Anya configuration, not a fixed dsh bundle. Workspace and Git status statements must be supported by current session context or tool results. Official snapshots retain their original text; these clarifications supplement the host integration.

- Reads return absolute paths, numbered lines and pagination, with 2000-line/line-length limits and a 50 KiB content budget. The complete observed version is fingerprinted; giant individual lines use bounded buffering.
- Existing files require a prior read. External changes invalidate the observation. Edits use literal matching and require `replace_all` for ambiguous matches. Path approval, Plan restrictions, previews and checkpoints remain active.
- Search invokes ripgrep directly and parses JSON to preserve Windows drive paths and Unicode names. Overflow results spill into workspace files retrievable with `read`.
- Image reads bound input size and decoding memory. Images larger than 2048 pixels are downscaled for the model with coordinate conversion hints for the original.
- Foreground shell deadlines promote commands to background jobs. Output collection is incremental, jobs are session-scoped, and completions produce notices. Anya's command rules and restricted process entry point remain active.
- Todos replace the list and allow one in-progress item by default. Questions retain stable ids and wait for an answer or cancellation. Plan writers unlock only after the user chooses Approve.
- Skills use the dsh catalog and `skill_content` framing. Office skills continue using the bundled Deno, Office Node and LibreOffice Kit.

This ports core tool contracts rather than running the complete official plugin ecosystem. Anya supplies search, interaction UI, skill storage, image upload, child execution and permissions. Child delegation uses the upstream foreground configuration and exposes no background parameter. Enabled Anya MCP and plugin tools, including Anya's computer-use plugin, remain available as host tools; this does not port the upstream harness's optional MCP, LSP, PTY, computer-use, teams, or continuable-agent plugins. This implementation must not be described as a byte-for-byte copy of all official plugins and protocols.

## Permissions and read previews

The native dsh tools use Anya’s shared permission policy, one-shot approval UI and audit log. Read-only cannot be elevated through a tool approval. Workspace-write prompts for unsandboxed shell calls and external paths; Full-access bypasses interactive gates. This does not reproduce the official harness OS process sandbox.

Successful `read` output uses the same code cards and highlighted sidebar as native `read_file`. The sidebar uses the recorded read activity, with snapshot fallback for changed or unavailable files. See [permissions and code preview](./permissions-and-preview.md).

## Requests and diagnostics

Identity, tool guidance, skill catalog, workspace and project rules form a stable prefix; volatile context and retrieval follow history. Tool definitions are fixed throughout an agent turn and bypass Anya plugin argument/result hooks. The dsh path omits legacy completion challenges and implicit shell verification; the model verifies its work using tool guidance.

Near the context ceiling, large tool payloads are clipped in a request copy and remeasured before summarization. Reasoning and tool-call ids survive. Durable original history is preserved.

Retries recover only the pending model round and retain committed tool results. Writable child tasks are not restarted wholesale after failure, avoiding repeated mutations. Partial-stream retries reset temporary content and calls to avoid merging attempts. Authentication errors are not retried; rate-limit backoff honors Retry-After.

The application configuration directory's `logs/deepseek-calls-YYYY-MM-DD.jsonl` records model calls and HTTP/stream attempts: correlation ids, total latency, first SSE/text latency, cache hit ratio, reasoning/completion tokens, retries and outcome. Keys, prompts and tool arguments are excluded. Unreported fields remain null; incomplete failed streams do not fabricate statistics. A start without an ending record may indicate cancellation or process interruption.

Titles and summaries disable thinking. Explicit short ordinary questions can lower a high budget to Low. Code diagnosis, project analysis and execution retain the configured budget. This is a conservative heuristic; production task success and cache performance have not been measured.

Normal cancellation writes an ending record with `outcome: interrupted`, preserving observed event/text latency and retry counts. A process crash can still leave a start-only record. Correlated `reasoning_policy` records include task class, configured/selected effort, policy mode and version.

### Validate effort against real task outcomes

Run the same model and judged task set in both modes. The default is adaptive; set `ANYA_DEEPSEEK_REASONING_POLICY=configured` in the Anya launch environment for a control run retaining configured effort. Remove it for the adaptive run. Record independent acceptance judgments in JSONL, such as `{"request_id":"actual-request-id","success":true}`, or use `call_id` to label one model call. Multi-call requests are judged once per group. HTTP success is not task success.

```powershell
node scripts/deepseek-metrics-report.mjs "actual-config-directory\logs" "outcomes.jsonl"
```

The report groups by model, task class, selected effort and policy mode. It summarizes cache hits, event/SSE/text latency, tokens, retries and separately judged task success. Missing quality labels remain null. Compare identical task sets; the report does not infer quality from transport completion or calculate fees using unverified prices.

## Maintenance and verification

Refresh snapshots from the upstream developer checkout:

```powershell
node --import file:///C:/My/code/deepseek-harness/node_modules/tsx/dist/loader.mjs scripts/sync-dsh-contracts.mjs C:\My\code\deepseek-harness
```

This maintenance command is not an application runtime or build dependency. Review Rust behavior alongside schema changes when upgrading.

Local verification covers observation/stale edit guards, literal matching, Unicode paths, pagination, background promotion/incremental output, Plan review, schema fidelity, prompt prefixes, DeepSeek reasoning/tool continuation and stream retries. No live DeepSeek API call, production cost measurement or new release installer was performed.

Delivery cards use bundled original Material Icon Theme SVGs with a pinned revision and retained MIT license, rather than dsh internal design artwork. Delivery is also available in the general tool set. See [file delivery cards](./file-delivery.md) for behavior and maintenance.
