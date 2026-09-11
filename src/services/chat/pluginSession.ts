export const PLUGIN_AGENT_SESSION_PREFIX = "plugin:";

/** True for sessions owned by `ctx.agent.run` (hidden from the conversation sidebar). */
export function isPluginAgentSessionId(sessionId: string): boolean {
  return sessionId.startsWith(PLUGIN_AGENT_SESSION_PREFIX);
}

/** Mint or normalize a plugin-owned agent session id. */
export function pluginAgentSessionId(pluginId: string, suffix?: string): string {
  const id = pluginId.trim();
  const extra = suffix?.trim();
  if (extra && isPluginAgentSessionId(extra)) return extra;
  if (extra) return `${PLUGIN_AGENT_SESSION_PREFIX}${id}:${extra}`;
  const rand =
    typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
      ? crypto.randomUUID()
      : `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
  return `${PLUGIN_AGENT_SESSION_PREFIX}${id}:${rand}`;
}
