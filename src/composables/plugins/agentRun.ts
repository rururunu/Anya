import { createWorkspace, getCurrentWorkspace, listWorkspaces } from "@/commands/workspace";
import { pluginAgentSessionId } from "@/services/chat/pluginSession";
import { useChatStore } from "@/stores/chat";
import { useChatSessionsStore } from "@/stores/chatSessions";

export type PluginAgentRunOptions = {
  /** Reuse a plugin-owned session. Omit to mint `plugin:<pluginId>:<uuid>`. */
  sessionId?: string;
  /** Bind tools to this folder (CLI cwd). Does not switch the workbench workspace. */
  cwd?: string;
};

export type PluginAgentRunResult = {
  sessionId: string;
};

function foldPath(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
}

/** Resolve a workspace id for `cwd`, else the workbench current workspace. */
export async function workspaceIdForPluginAgent(cwd?: string): Promise<string | undefined> {
  const root = cwd?.trim();
  if (root) {
    const wanted = foldPath(root);
    try {
      const listed = await listWorkspaces();
      const hit = listed.find(
        (workspace) => foldPath(workspace.root) === wanted || foldPath(workspace.id) === wanted,
      );
      if (hit) return hit.id;
    } catch {
      /* fall through to create */
    }
    try {
      const created = await createWorkspace(root);
      const id = String(created?.id ?? "").trim();
      if (id) return id;
    } catch {
      /* fall through to current */
    }
  }
  try {
    const current = await getCurrentWorkspace();
    return String(current?.id ?? "").trim() || undefined;
  } catch {
    return undefined;
  }
}

/** Run Anya's Rust agent loop on a plugin-owned session (not the open workbench chat). */
export async function runPluginAgent(
  pluginId: string,
  prompt: string,
  options?: PluginAgentRunOptions,
): Promise<PluginAgentRunResult> {
  const trimmed = prompt.trim();
  if (!trimmed) throw new Error("prompt is empty");

  const sessionId = pluginAgentSessionId(pluginId, options?.sessionId);
  const chatStore = useChatStore();
  const sessionsStore = useChatSessionsStore();

  chatStore.markSessionStarted(sessionId);
  if (!sessionsStore.sessions[sessionId]) {
    chatStore.setSessionMessages(sessionId, []);
  }
  chatStore.ensureCompose(sessionId);
  chatStore.setCompose(sessionId, { chatMode: "agent" });

  const workspaceId = await workspaceIdForPluginAgent(options?.cwd);

  const ok = await chatStore.send(trimmed, sessionId, {
    isolate: true,
    workspaceId,
    quickAsk: !workspaceId,
  });
  if (!ok) throw new Error("agent run failed");
  return { sessionId };
}
