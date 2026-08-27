import type { AskUserSession, PathPermissionSession } from "@/components/chat/ChatInputBar.vue";
import type { ToolApprovalSession } from "@/types/chat";

/** Which panel is shown in the review sidebar. */
export type ReviewView = "diff" | "agents" | "runtime" | "image";

/** Side of the drop target a dragged workspace row is hovering over. */
export type WorkspaceDropPosition = "before" | "after";

/** A single unresolved interaction (ask-user / permission / approval) for a session. */
export type PendingInteraction =
  | { kind: "ask_user"; value: AskUserSession }
  | { kind: "path_permission"; value: PathPermissionSession }
  | { kind: "tool_approval"; value: ToolApprovalSession };

/** Tracks a pointer-based drag used to reorder workspaces. */
export type WorkspacePointerDrag = {
  pointerId: number;
  sourceId: string;
  startX: number;
  startY: number;
  dragging: boolean;
};

/** Tracks a pointer-based drag used to move a conversation between workspaces. */
export type SessionPointerDrag = {
  pointerId: number;
  sessionId: string;
  sourceWorkspaceId: string | null;
  preview: string;
  startX: number;
  startY: number;
  x: number;
  y: number;
  dragging: boolean;
};

/** Floating preview that follows the pointer while a conversation is dragged. */
export type SessionDragGhost = {
  x: number;
  y: number;
  title: string;
};
