import { getCurrentWorkspace, openWorkspaceInTerminal } from "@/commands/workspace";
import { activePluginSessionId } from "@/composables/plugins/conversationSend";
import { useChatStore } from "@/stores/chat";
import { useChatSessionsStore } from "@/stores/chatSessions";

export type PluginWorkspaceSurface = {
  /** Open the OS terminal at a workspace. Omit `id` to use the current conversation workspace. */
  openInTerminal: (id?: string) => Promise<void>;
};

export function createPluginWorkspaceSurface(): PluginWorkspaceSurface {
  return {
    openInTerminal: (id) => openPluginWorkspaceInTerminal(id),
  };
}

/** Open the OS terminal at a workspace root (same IPC as the workbench context menu). */
export async function openPluginWorkspaceInTerminal(id?: string): Promise<void> {
  const workspaceId = await resolvePluginWorkspaceId(id);
  await openWorkspaceInTerminal(workspaceId);
}

/** Resolve a workspace id: explicit argument, then the live session, then Anya's current workspace. */
export async function resolvePluginWorkspaceId(id?: string): Promise<string> {
  const explicit = id?.trim();
  if (explicit) return explicit;

  const sessionId = activePluginSessionId();
  if (sessionId) {
    try {
      const draft = String(
        useChatStore().sessionCompose?.[sessionId]?.draftWorkspaceId ?? "",
      ).trim();
      if (draft) return draft;
    } catch {
      /* pinia may be unavailable outside the workbench */
    }
    try {
      const row = useChatSessionsStore().summaries.find((item) => item.sessionId === sessionId);
      const fromSession = String(row?.workspaceId ?? "").trim();
      if (fromSession) return fromSession;
    } catch {
      /* list next */
    }
  }

  const current = await getCurrentWorkspace();
  const currentId = String(current?.id ?? "").trim();
  if (currentId) return currentId;
  throw new Error("No active workspace");
}
