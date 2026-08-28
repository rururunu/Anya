import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export interface Workspace {
  id: string;
  name: string;
  root: string;
  description?: string;
  source?: string | null;
  createdAt: string;
  lastUsedAt: string;
  pinned: boolean;
  archived?: boolean;
  sortOrder: number;
}

export function workspaceSourceLabel(source?: string | null): string {
  switch (source?.trim().toLowerCase()) {
    case "vscode":
    case "visual studio code":
      return "VS Code";
    case "idea":
    case "intellij":
    case "intellij idea":
      return "IntelliJ IDEA";
    default:
      return source?.trim() ?? "";
  }
}

export function listWorkspaces(): Promise<Workspace[]> {
  return invoke("list_workspaces");
}

export function listArchivedWorkspaces(): Promise<Workspace[]> {
  return invoke("list_archived_workspaces");
}

export function getCurrentWorkspace(): Promise<Workspace | null> {
  return invoke("get_current_workspace");
}

export function listWorkspaceFiles(): Promise<string[]> {
  return invoke("list_workspace_files");
}

export function createWorkspace(root: string): Promise<Workspace> {
  return invoke("create_workspace", { root });
}

export function switchWorkspace(id: string): Promise<Workspace> {
  return invoke("switch_workspace", { id });
}

export function isWorkspaceNotFoundError(error: unknown): boolean {
  return String(error).toLowerCase().includes("workspace not found");
}

/** Switch workspace; returns false when the id is missing instead of throwing. */
export async function trySwitchWorkspace(id: string): Promise<boolean> {
  try {
    await switchWorkspace(id);
    return true;
  } catch (error) {
    if (!isWorkspaceNotFoundError(error)) throw error;
    try {
      await clearCurrentWorkspace();
    } catch {
      // Best-effort: drop a stale current-workspace pointer.
    }
    return false;
  }
}

export function clearCurrentWorkspace(): Promise<void> {
  return invoke("clear_current_workspace");
}

export function deleteWorkspace(id: string): Promise<void> {
  return invoke("delete_workspace", { id });
}

export function updateWorkspace(
  id: string,
  name: string,
  description?: string | null,
): Promise<Workspace> {
  return invoke("update_workspace", { id, name, description: description ?? null });
}

export function openWorkspaceFolder(id: string): Promise<void> {
  return invoke("open_workspace_folder", { id });
}

export function openWorkspaceInTerminal(id: string): Promise<void> {
  return invoke("open_workspace_in_terminal", { id });
}

export function setWorkspacePinned(id: string, pinned: boolean): Promise<void> {
  return invoke("set_workspace_pinned", { id, pinned });
}

export function setWorkspaceArchived(id: string, archived: boolean): Promise<void> {
  return invoke("set_workspace_archived", { id, archived });
}

export function reorderWorkspaces(ids: string[]): Promise<void> {
  return invoke("reorder_workspaces", { ids });
}

export async function selectWorkspaceFolder(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false });
  return typeof selected === "string" ? selected : null;
}
