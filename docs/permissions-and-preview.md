# Permissions and code preview

[简体中文](./permissions-and-preview.zh-CN.md) · Applies to Anya v0.2.29.

## Permission scopes

Chat mode (Ask / Agent / Plan / Image) and permission scope are separate. Plan approval and tool-specific restrictions remain additional checks.

| UI scope        | Persisted value | Behavior                                                                                       |
| --------------- | --------------- | ---------------------------------------------------------------------------------------------- |
| Read-only       | `ask`           | Reject file mutations and arbitrary shell execution; user approval cannot elevate this scope.  |
| Workspace-write | `auto`          | Allow workspace file edits; gate external paths and approve each unsandboxed shell invocation. |
| Full-access     | `alwaysAllow`   | Bypass interactive tool/path gates; explicit safety restrictions remain.                       |

Legacy settings retain their serialized values. These scopes express Anya policy, not dsh's OS-enforced `read-only` / `workspace-write` / `danger-full-access` sandbox. Existing ancestor paths are resolved to check symlink and junction boundaries.

Approvals offer Allow once and Reject. There are no session-wide or permanent tool grants. `tool_approval.rs` and `path_permission.rs` share cancellation-aware waiting through `approval_wait.rs`; tool/path requests expire after ten minutes. `ToolError` distinguishes policy denial from explicit user rejection: only the latter produces the user-rejected stop message. The agent receives the current permission policy on each loop.

Policy decisions, approval requests and outcomes are written to SQLite `chat_permission_events`. Audit writes complete before execution; failures deny execution.

## Workbench and overlay recovery

`usePendingInteractions.ts` maintains ordered queues per session. Question, tool and path events append rather than replace the displayed head. Backend sequence numbers order request kinds. Resolved request IDs suppress stale events and snapshots.

`get_pending_interactions` reads a session snapshot; an empty session ID requests all sessions. Startup, focus and session changes reconcile snapshots. Backend terminal paths emit `interaction-resolved`, including cancellation and timeout, so other windows remove completed cards. Failed submissions restore the request.

`ApprovalRequestPanel.vue` supplies shared operation details and vertical choices with arrow-key navigation and Enter confirmation. `useSharedReviewState.ts` synchronizes image tabs, selected image and subagent viewing state per root session through `anya.shared-review.v1.*`. Window size and layout stay local. This is not a complete mirror of every sidebar state; code-read selection is local to the window where the file is clicked.

## Read cards and highlighted sidebar

`readCodePreview.ts` extracts contiguous numbered text from successful `read` and `read_file` results, retaining the actual returned range and indentation. Binary, image and document formats keep their existing presentations. `FileReadCard.vue` renders a filename, range and expandable code editor. CodeMirror provides syntax highlighting, original line numbers and horizontal scrolling for long lines.

Clicking the filename or preview button opens `ReadCodeSidebar.vue` in the workbench or overlay. `preview_read_code` accepts a successful recorded activity ID, never an arbitrary UI-supplied path. It resolves relative paths through the recorded session or parent workspace, without falling back to the currently selected workspace. Disk reading runs in a blocking worker and accepts regular UTF-8 files up to 1 MiB, rejecting NUL-containing content.

The sidebar compares the captured range with the current file. Matching content displays the complete file, scrolls to the read range and highlights those lines. Changed or unavailable files display the captured excerpt with a notice. The comparison covers the read range; it does not assert that unrelated file content is unchanged. Switching conversations clears code-read selection. Previewing is read-only and does not append content to model context.

## Verification boundaries

Focused tests cover request queue recovery, shared review state, read-output parsing and card clicks. Frontend type checking and targeted linting, Rust tool tests and Rust compilation were run during implementation. These checks do not establish native dual-window visual correctness, live DeepSeek task quality or production sandbox behavior.
