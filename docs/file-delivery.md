# File delivery cards

[简体中文](./file-delivery.zh-CN.md)

Applies to Anya v0.2.26. Cards deliver final artifacts; source-code change summaries are separate.

## Selection and display

The model calls `present` with a `files` array containing `path` and optional `description` entries. Both general and DeepSeek tool sets support it; the dedicated Image mode still exposes only image generation. Each call accepts up to eight files; select final deliverables. Relative paths resolve against the tool workspace. Files must exist, be regular files and pass path permissions; directories and symbolic links are rejected.

Successful results contain `version: 1` and file metadata: original path, absolute path, name, description and size at delivery. Metadata is limited to 1 MiB. Files are not copied or uploaded to Companion; response paths and all modified files are not automatically turned into cards.

Cards appear after the reply completes and are restored from persisted main-task successful tool activities. Latest metadata wins for the same absolute path, with Windows case/separator normalization. More than four cards can be expanded. Child-task deliveries do not automatically become parent cards; the main model must present final files again.

## Opening and preview

Images use the existing image preview sidebar and have a default-app button. Other files open in the system default application; a folder button reveals the file. Pending actions prevent repeated clicks; failures show an error and support retry.

There is currently no built-in Office/PDF preview sidebar or application picker. Office skills can generate preview artifacts but do not automatically attach them as card previews. History references the original file; opening uses its current state after moves, deletion or edits. Name and size are delivery-time snapshots.

## Icons and maintenance

Cards use Anya theme variables, rounded corners and responsive layout. `DeliveryFileIcon.vue` selects bundled Material Icon Theme SVGs by extension, with a generic document fallback. Extensions determine appearance, not MIME or content validity. Unmodified artwork is pinned to `735a0166cb72f3514a717f7c17905a77374d5df4`.

Source metadata and license are in `src/assets/material-file-icons/`; `public/licenses/material-icon-theme-MIT.txt` ships with the frontend build. Update SVGs, license and source revision together. No VS Code extension or runtime network request is needed.

## Source and validation

- Rust: `src-tauri/src/core/tools/present.rs`, `core/chat/agent_loop/tools.rs`, `core/chat/db/tool_activity.rs`.
- Frontend: `src/services/chat/presentedFiles.ts`, `src/components/chat/PresentedFiles.vue`, `DeliveryFileIcon.vue`; mounted by `MessageList.vue`.
- Tests cover metadata extraction, deduplication and invalid-result filtering, plus opening, reveal, image preview, expansion and error retry. IPC is mocked; these tests do not verify real system application launching.

Related: [DeepSeek tools](./deepseek-harness.md) · [Office workflows](./office.md) · [architecture](./architecture-overview.md)
