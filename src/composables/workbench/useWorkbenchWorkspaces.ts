import { computed, ref, type Ref } from "vue";

import type { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import {
  clearCurrentWorkspace,
  createWorkspace,
  deleteWorkspace,
  listWorkspaces,
  openWorkspaceFolder as openWorkspaceFolderCommand,
  reorderWorkspaces,
  selectWorkspaceFolder,
  setWorkspacePinned,
  setWorkspaceArchived,
  switchWorkspace,
  type Workspace,
} from "@/commands/workspace";
import type { WorkspaceDropPosition, WorkspacePointerDrag } from "./types";
import type { WorkbenchLabels } from "./useWorkbenchLabels";

export interface UseWorkbenchWorkspacesOptions {
  workspaces: Ref<Workspace[]>;
  activeSessionWorkspaceId: Ref<string | null>;
  navigationLabels: WorkbenchLabels["navigationLabels"];
  confirmDialogRef: Ref<InstanceType<typeof AppConfirmDialog> | null>;
  refreshSessions: () => Promise<void>;
  createConversation: (workspaceId: string | null) => void;
}

/**
 * Workspace list UI: pinned/regular navigation sections, collapsed state,
 * the per-row action menu, and pointer-based drag-to-reorder.
 */
export function useWorkbenchWorkspaces(options: UseWorkbenchWorkspacesOptions) {
  const {
    workspaces,
    activeSessionWorkspaceId,
    navigationLabels,
    confirmDialogRef,
    refreshSessions,
    createConversation,
  } = options;

  const collapsedWorkspaceIds = ref(new Set<string>());
  const collapsedNavigationSections = ref(new Set<string>());
  const workspaceMenuId = ref("");
  const draggedWorkspaceId = ref("");
  const dragOverWorkspaceId = ref("");
  const workspaceDropPosition = ref<WorkspaceDropPosition | null>(null);
  const workspacePointerDrag = ref<WorkspacePointerDrag | null>(null);
  const suppressedWorkspaceClickId = ref("");

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

  function clearWorkspaceLongPress(drag: WorkspacePointerDrag) {
    if (drag.longPressTimer) globalThis.clearTimeout(drag.longPressTimer);
    drag.longPressTimer = null;
  }

  function suppressWorkspaceClick(workspaceId: string) {
    suppressedWorkspaceClickId.value = workspaceId;
    globalThis.setTimeout(() => {
      if (suppressedWorkspaceClickId.value === workspaceId) suppressedWorkspaceClickId.value = "";
    }, 300);
  }

  function startWorkspacePointerDrag(event: PointerEvent, workspace: Workspace) {
    if (event.button !== 0 || !(event.target instanceof Element)) return;
    if (event.target.closest(".workspace-actions, .workspace-menu")) return;

    if (event.currentTarget instanceof HTMLElement) {
      event.currentTarget.setPointerCapture(event.pointerId);
    }
    const drag: WorkspacePointerDrag = {
      pointerId: event.pointerId,
      sourceId: workspace.id,
      startX: event.clientX,
      startY: event.clientY,
      dragging: false,
      cancelled: false,
      longPressTimer: null,
    };
    drag.longPressTimer = globalThis.setTimeout(() => {
      const current = workspacePointerDrag.value;
      if (!current || current.pointerId !== drag.pointerId || current.sourceId !== drag.sourceId)
        return;
      current.dragging = true;
      draggedWorkspaceId.value = current.sourceId;
      workspaceMenuId.value = "";
      document.getSelection()?.removeAllRanges();
    }, 260);
    workspacePointerDrag.value = drag;
  }

  function moveWorkspacePointerDrag(event: PointerEvent) {
    const drag = workspacePointerDrag.value;
    if (!drag || drag.pointerId !== event.pointerId) return;
    if (drag.cancelled) return;

    if (!drag.dragging) {
      const distance = Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY);
      if (distance < 6) return;
      clearWorkspaceLongPress(drag);
      drag.cancelled = true;
      return;
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
    const drag = workspacePointerDrag.value;
    if (!drag || drag.pointerId !== event.pointerId) return;

    const targetId = dragOverWorkspaceId.value;
    const dropPosition = workspaceDropPosition.value;
    clearWorkspaceLongPress(drag);
    workspacePointerDrag.value = null;
    draggedWorkspaceId.value = "";
    dragOverWorkspaceId.value = "";
    workspaceDropPosition.value = null;
    if (!drag.dragging) {
      if (drag.cancelled) suppressWorkspaceClick(drag.sourceId);
      return;
    }

    event.preventDefault();
    suppressWorkspaceClick(drag.sourceId);
    if (targetId && dropPosition) void reorderWorkspaceItems(drag.sourceId, targetId, dropPosition);
  }

  function cancelWorkspacePointerDrag(event: PointerEvent) {
    const drag = workspacePointerDrag.value;
    if (!drag || drag.pointerId !== event.pointerId) return;
    clearWorkspaceLongPress(drag);
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
    pinnedWorkspaces,
    regularWorkspaces,
    workspaceNavigationSections,
    toggleNavigationSection,
    toggleWorkspaceGroup,
    toggleWorkspaceMenu,
    handleWorkspaceClick,
    startWorkspacePointerDrag,
    moveWorkspacePointerDrag,
    finishWorkspacePointerDrag,
    cancelWorkspacePointerDrag,
    toggleWorkspacePinned,
    addWorkspace,
    createWorkspaceConversation,
    openWorkspaceFolder,
    archiveWorkspace,
    removeWorkspace,
    clearWorkspaceLongPress,
  };
}
