import { computed, ref, type Ref } from "vue";

import type { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import {
  clearCurrentWorkspace,
  createWorkspace,
  deleteWorkspace,
  listWorkspaces,
  openWorkspaceFolder as openWorkspaceFolderCommand,
  openWorkspaceInTerminal as openWorkspaceInTerminalCommand,
  reorderWorkspaces,
  selectWorkspaceFolder,
  setWorkspacePinned,
  setWorkspaceArchived,
  switchWorkspace,
  type Workspace,
} from "@/commands/workspace";
import { formatSessionPreview } from "@/services/chat/sessionPreview";
import type { ChatSessionSummary } from "@/types/chat";
import type { WorkspaceDropPosition, SessionPointerDrag, WorkspacePointerDrag } from "./types";
import type { WorkbenchLabels } from "./useWorkbenchLabels";

export interface UseWorkbenchWorkspacesOptions {
  workspaces: Ref<Workspace[]>;
  activeSessionWorkspaceId: Ref<string | null>;
  navigationLabels: WorkbenchLabels["navigationLabels"];
  labels: WorkbenchLabels["labels"];
  confirmDialogRef: Ref<InstanceType<typeof AppConfirmDialog> | null>;
  editWorkspaceDialogRef: Ref<{ edit: (workspace: Workspace) => Promise<boolean> } | null>;
  refreshSessions: () => Promise<void>;
  createConversation: (workspaceId: string | null) => void;
  moveSessionToWorkspace: (sessionId: string, workspaceId: string) => Promise<void>;
}

/**
 * Workspace list UI: pinned/regular navigation sections, collapsed state,
 * the per-row action menu, pointer drag to reorder workspaces, and pointer
 * drag to move conversations between workspaces.
 */
export function useWorkbenchWorkspaces(options: UseWorkbenchWorkspacesOptions) {
  const {
    workspaces,
    activeSessionWorkspaceId,
    navigationLabels,
    labels,
    confirmDialogRef,
    editWorkspaceDialogRef,
    refreshSessions,
    createConversation,
    moveSessionToWorkspace,
  } = options;

  const collapsedWorkspaceIds = ref(new Set<string>());
  const collapsedNavigationSections = ref(new Set<string>());
  const workspaceMenuId = ref("");
  const draggedWorkspaceId = ref("");
  const dragOverWorkspaceId = ref("");
  const workspaceDropPosition = ref<WorkspaceDropPosition | null>(null);
  const workspacePointerDrag = ref<WorkspacePointerDrag | null>(null);
  const sessionPointerDrag = ref<SessionPointerDrag | null>(null);
  const draggedSessionId = ref("");
  const sessionDropWorkspaceId = ref("");
  const suppressedWorkspaceClickId = ref("");
  const suppressedSessionClickId = ref("");
  const sessionDragGhost = computed(() => {
    const drag = sessionPointerDrag.value;
    if (!drag?.dragging) return null;
    return { x: drag.x, y: drag.y, title: drag.preview };
  });

  const pinnedWorkspaces = computed(() => workspaces.value.filter((workspace) => workspace.pinned));
  const regularWorkspaces = computed(() =>
    workspaces.value.filter((workspace) => !workspace.pinned),
  );
  const workspaceNavigationSections = computed(() => [
    ...(pinnedWorkspaces.value.length > 0
      ? [{ id: "pinned", label: navigationLabels.value.pinned, items: pinnedWorkspaces.value }]
      : []),
    { id: "workspaces", label: navigationLabels.value.workspaces, items: regularWorkspaces.value },
  ]);

  function toggleNavigationSection(id: string) {
    const next = new Set(collapsedNavigationSections.value);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    collapsedNavigationSections.value = next;
  }

  function toggleWorkspaceGroup(id: string) {
    const next = new Set(collapsedWorkspaceIds.value);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    collapsedWorkspaceIds.value = next;
  }

  function toggleWorkspaceMenu(id: string) {
    workspaceMenuId.value = workspaceMenuId.value === id ? "" : id;
  }

  function handleWorkspaceClick(workspace: Workspace) {
    if (suppressedWorkspaceClickId.value === workspace.id) {
      suppressedWorkspaceClickId.value = "";
      return;
    }
    toggleWorkspaceGroup(workspace.id);
  }

  function suppressWorkspaceClick(workspaceId: string) {
    suppressedWorkspaceClickId.value = workspaceId;
    globalThis.setTimeout(() => {
      if (suppressedWorkspaceClickId.value === workspaceId) suppressedWorkspaceClickId.value = "";
    }, 300);
  }

  function suppressSessionClick(sessionId: string) {
    suppressedSessionClickId.value = sessionId;
    globalThis.setTimeout(() => {
      if (suppressedSessionClickId.value === sessionId) suppressedSessionClickId.value = "";
    }, 300);
  }

  function consumeSuppressedSessionClick(sessionId: string) {
    if (suppressedSessionClickId.value !== sessionId) return false;
    suppressedSessionClickId.value = "";
    return true;
  }

  function expandWorkspaceGroup(id: string) {
    if (!collapsedWorkspaceIds.value.has(id)) return;
    const next = new Set(collapsedWorkspaceIds.value);
    next.delete(id);
    collapsedWorkspaceIds.value = next;
  }

  function startWorkspacePointerDrag(event: PointerEvent, workspace: Workspace) {
    if (event.button !== 0 || !(event.target instanceof Element)) return;
    if (event.target.closest(".workspace-actions, .workspace-menu")) return;
    if (sessionPointerDrag.value) return;

    const row =
      event.currentTarget instanceof HTMLElement
        ? event.currentTarget
        : event.target.closest<HTMLElement>(".workspace-row");
    row?.setPointerCapture(event.pointerId);
    workspacePointerDrag.value = {
      pointerId: event.pointerId,
      sourceId: workspace.id,
      startX: event.clientX,
      startY: event.clientY,
      dragging: false,
    };
  }

  function startSessionPointerDrag(event: PointerEvent, session: ChatSessionSummary) {
    if (event.button !== 0 || !(event.target instanceof Element)) return;
    if (event.target.closest(".session-action")) return;
    if (workspacePointerDrag.value?.dragging) return;

    const row =
      event.currentTarget instanceof HTMLElement
        ? event.currentTarget
        : event.target.closest<HTMLElement>(".session-row");
    row?.setPointerCapture(event.pointerId);
    workspacePointerDrag.value = null;
    sessionPointerDrag.value = {
      pointerId: event.pointerId,
      sessionId: session.sessionId,
      sourceWorkspaceId: session.workspaceId ?? null,
      preview: formatSessionPreview(session.preview || "") || labels.value.untitled,
      startX: event.clientX,
      startY: event.clientY,
      x: event.clientX,
      y: event.clientY,
      dragging: false,
    };
  }

  function moveSessionPointerDrag(event: PointerEvent) {
    const drag = sessionPointerDrag.value;
    if (!drag || drag.pointerId !== event.pointerId) return;
    if (!drag.dragging) {
      const distance = Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY);
      if (distance < 6) return;
      drag.dragging = true;
      draggedSessionId.value = drag.sessionId;
      workspaceMenuId.value = "";
      document.getSelection()?.removeAllRanges();
    }

    event.preventDefault();
    drag.x = event.clientX;
    drag.y = event.clientY;
    const targetId =
      document
        .elementFromPoint(event.clientX, event.clientY)
        ?.closest<HTMLElement>("[data-workspace-id]")?.dataset.workspaceId ?? "";
    sessionDropWorkspaceId.value = targetId && targetId !== drag.sourceWorkspaceId ? targetId : "";
  }

  async function finishSessionPointerDrag(event: PointerEvent) {
    const drag = sessionPointerDrag.value;
    if (!drag || drag.pointerId !== event.pointerId) return;
    const targetId = sessionDropWorkspaceId.value;
    const didDrag = drag.dragging;
    sessionPointerDrag.value = null;
    draggedSessionId.value = "";
    sessionDropWorkspaceId.value = "";
    if (!didDrag) return;

    event.preventDefault();
    suppressSessionClick(drag.sessionId);
    if (!targetId) return;
    const target = workspaces.value.find((workspace) => workspace.id === targetId);
    if (!target) return;
    const confirmed = await confirmDialogRef.value?.ask({
      title: navigationLabels.value.moveConversationTitle,
      description: navigationLabels.value.moveConversationDescription,
      detailLabel: target.name,
      confirmLabel: navigationLabels.value.continue,
      cancelLabel: navigationLabels.value.cancel,
      tone: "default",
    });
    if (!confirmed) return;
    expandWorkspaceGroup(targetId);
    await moveSessionToWorkspace(drag.sessionId, targetId);
  }

  function cancelSessionPointerDrag(event: PointerEvent) {
    const drag = sessionPointerDrag.value;
    if (!drag || drag.pointerId !== event.pointerId) return;
    sessionPointerDrag.value = null;
    draggedSessionId.value = "";
    sessionDropWorkspaceId.value = "";
  }

  function moveWorkspacePointerDrag(event: PointerEvent) {
    if (sessionPointerDrag.value) {
      moveSessionPointerDrag(event);
      return;
    }
    const drag = workspacePointerDrag.value;
    if (!drag || drag.pointerId !== event.pointerId) return;

    if (!drag.dragging) {
      const distance = Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY);
      if (distance < 6) return;
      drag.dragging = true;
      draggedWorkspaceId.value = drag.sourceId;
      workspaceMenuId.value = "";
      document.getSelection()?.removeAllRanges();
    }

    event.preventDefault();
    const targetElement = document
      .elementFromPoint(event.clientX, event.clientY)
      ?.closest<HTMLElement>("[data-workspace-id]");
    const targetId = targetElement?.dataset.workspaceId ?? "";
    const source = workspaces.value.find((workspace) => workspace.id === drag.sourceId);
    const target = workspaces.value.find((workspace) => workspace.id === targetId);
    const validTarget =
      source && target && source.id !== target.id && source.pinned === target.pinned;
    dragOverWorkspaceId.value = validTarget ? target.id : "";
    if (validTarget && targetElement) {
      const targetRow = targetElement.querySelector<HTMLElement>(":scope > .workspace-row");
      const bounds = targetRow?.getBoundingClientRect();
      workspaceDropPosition.value =
        bounds && event.clientY >= bounds.top + bounds.height / 2 ? "after" : "before";
    } else {
      workspaceDropPosition.value = null;
    }
  }

  function finishWorkspacePointerDrag(event: PointerEvent) {
    if (sessionPointerDrag.value) {
      void finishSessionPointerDrag(event);
      return;
    }
    const drag = workspacePointerDrag.value;
    if (!drag || drag.pointerId !== event.pointerId) return;

    const targetId = dragOverWorkspaceId.value;
    const dropPosition = workspaceDropPosition.value;
    workspacePointerDrag.value = null;
    draggedWorkspaceId.value = "";
    dragOverWorkspaceId.value = "";
    workspaceDropPosition.value = null;
    if (!drag.dragging) return;

    event.preventDefault();
    suppressWorkspaceClick(drag.sourceId);
    if (targetId && dropPosition) void reorderWorkspaceItems(drag.sourceId, targetId, dropPosition);
  }

  function cancelWorkspacePointerDrag(event: PointerEvent) {
    if (sessionPointerDrag.value) {
      cancelSessionPointerDrag(event);
      return;
    }
    const drag = workspacePointerDrag.value;
    if (!drag || drag.pointerId !== event.pointerId) return;
    workspacePointerDrag.value = null;
    draggedWorkspaceId.value = "";
    dragOverWorkspaceId.value = "";
    workspaceDropPosition.value = null;
  }

  async function reorderWorkspaceItems(
    sourceId: string,
    targetId: string,
    dropPosition: WorkspaceDropPosition,
  ) {
    if (!sourceId || sourceId === targetId) return;
    const sourceIndex = workspaces.value.findIndex((workspace) => workspace.id === sourceId);
    const targetIndex = workspaces.value.findIndex((workspace) => workspace.id === targetId);
    const source = workspaces.value[sourceIndex];
    const target = workspaces.value[targetIndex];
    if (!source || !target || source.pinned !== target.pinned) return;

    const next = workspaces.value.filter((workspace) => workspace.id !== sourceId);
    const adjustedTargetIndex = next.findIndex((workspace) => workspace.id === targetId);
    const insertionIndex = adjustedTargetIndex + (dropPosition === "after" ? 1 : 0);
    next.splice(insertionIndex, 0, source);
    workspaces.value = next;
    try {
      await reorderWorkspaces(next.map((workspace) => workspace.id));
    } catch (error) {
      console.error("reorder workspaces failed:", error);
      workspaces.value = await listWorkspaces();
    }
  }

  async function toggleWorkspacePinned(workspace: Workspace) {
    workspaceMenuId.value = "";
    try {
      await setWorkspacePinned(workspace.id, !workspace.pinned);
      workspaces.value = await listWorkspaces();
    } catch (error) {
      console.error("set workspace pinned failed:", error);
    }
  }

  async function editWorkspace(workspace: Workspace) {
    workspaceMenuId.value = "";
    const saved = await editWorkspaceDialogRef.value?.edit(workspace);
    if (saved) {
      workspaces.value = await listWorkspaces();
    }
  }

  async function addWorkspace() {
    const root = await selectWorkspaceFolder();
    if (!root) return;
    await createWorkspace(root);
    await refreshSessions();
  }

  async function createWorkspaceConversation(workspace: Workspace) {
    await switchWorkspace(workspace.id);
    createConversation(workspace.id);
    await refreshSessions();
  }

  async function openWorkspaceFolder(workspace: Workspace) {
    workspaceMenuId.value = "";
    try {
      await openWorkspaceFolderCommand(workspace.id);
    } catch (error) {
      console.error("open workspace folder failed:", error);
    }
  }

  async function openWorkspaceInTerminal(workspace: Workspace) {
    workspaceMenuId.value = "";
    try {
      await openWorkspaceInTerminalCommand(workspace.id);
    } catch (error) {
      console.error("open workspace in terminal failed:", error);
    }
  }

  async function archiveWorkspace(workspace: Workspace) {
    workspaceMenuId.value = "";
    await setWorkspaceArchived(workspace.id, true);
    if (activeSessionWorkspaceId.value === workspace.id) {
      await clearCurrentWorkspace();
      activeSessionWorkspaceId.value = null;
      createConversation(null);
    }
    await refreshSessions();
  }

  async function removeWorkspace(workspace: Workspace) {
    workspaceMenuId.value = "";
    const confirmed = await confirmDialogRef.value?.ask({
      title: navigationLabels.value.deleteWorkspace,
      description: navigationLabels.value.deleteWorkspaceConfirm,
      confirmLabel: navigationLabels.value.confirmDelete,
      cancelLabel: navigationLabels.value.cancel,
    });
    if (!confirmed) return;
    await deleteWorkspace(workspace.id);
    if (activeSessionWorkspaceId.value === workspace.id) {
      await clearCurrentWorkspace();
      activeSessionWorkspaceId.value = null;
    }
    await refreshSessions();
  }

  return {
    collapsedWorkspaceIds,
    collapsedNavigationSections,
    workspaceMenuId,
    draggedWorkspaceId,
    dragOverWorkspaceId,
    workspaceDropPosition,
    workspacePointerDrag,
    draggedSessionId,
    sessionDropWorkspaceId,
    sessionDragGhost,
    pinnedWorkspaces,
    regularWorkspaces,
    workspaceNavigationSections,
    toggleNavigationSection,
    toggleWorkspaceGroup,
    toggleWorkspaceMenu,
    handleWorkspaceClick,
    startWorkspacePointerDrag,
    startSessionPointerDrag,
    consumeSuppressedSessionClick,
    moveWorkspacePointerDrag,
    finishWorkspacePointerDrag,
    cancelWorkspacePointerDrag,
    toggleWorkspacePinned,
    editWorkspace,
    addWorkspace,
    createWorkspaceConversation,
    openWorkspaceFolder,
    openWorkspaceInTerminal,
    archiveWorkspace,
    removeWorkspace,
  };
}
