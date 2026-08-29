import type { ToolActivity } from "@/types/chat";
import { SUBAGENT_TOOLS } from "@/services/chat/subagentTools";

const SUBAGENT_SESSION_MARKER = "-sub-";

export { SUBAGENT_SESSION_MARKER };

export function isSubagentSessionId(sessionId: string): boolean {
  return sessionId.includes(SUBAGENT_SESSION_MARKER);
}

export function parentSessionId(sessionId: string): string {
  const index = sessionId.indexOf(SUBAGENT_SESSION_MARKER);
  return index > 0 ? sessionId.slice(0, index) : "";
}

export function subagentIdFromSession(sessionId: string): string {
  const index = sessionId.indexOf(SUBAGENT_SESSION_MARKER);
  return index >= 0 ? sessionId.slice(index + SUBAGENT_SESSION_MARKER.length) : "";
}

export function buildSubagentSessionId(parentSessionId: string, subagentId: string): string {
  return `${parentSessionId}${SUBAGENT_SESSION_MARKER}${subagentId}`;
}

export function rootSessionId(sessionId: string): string {
  return parentSessionId(sessionId) || sessionId;
}

/** Map a tool-activity entry id (`activityId` or `activityId:index`) to a sub-agent id. */
export function findSubagentIdForEntry(activities: ToolActivity[], entryId: string): string | null {
  const colon = entryId.indexOf(":");
  if (colon > 0) {
    const activityId = entryId.slice(0, colon);
    const index = Number(entryId.slice(colon + 1));
    if (!Number.isFinite(index) || index < 0) return null;

    const parent = activities.find((activity) => activity.id === activityId);
    if (!parent) return null;

    const groups = groupChildActivitiesBySubagentId(
      activities.filter((activity) => activity.parentActivityId === activityId),
    );
    const group = groups[index];
    const subagentId = group?.[0]?.subagentId;
    return subagentId?.trim() || null;
  }

  const parent = activities.find((activity) => activity.id === entryId);
  if (!parent || !SUBAGENT_TOOLS.has(parent.toolName)) return null;
  const children = activities.filter((activity) => activity.parentActivityId === entryId);
  const subagentId = children.find((child) => child.subagentId?.trim())?.subagentId;
  return subagentId?.trim() || null;
}

function groupChildActivitiesBySubagentId(children: ToolActivity[]): ToolActivity[][] {
  const groups = new Map<string, ToolActivity[]>();
  for (const child of children) {
    const key = child.subagentId ?? "default";
    const group = groups.get(key) ?? [];
    group.push(child);
    groups.set(key, group);
  }
  return [...groups.values()];
}

export function resolveSubagentSessionId(
  rootSessionId: string,
  activities: ToolActivity[],
  entryId: string,
  _knownSessionIds?: Iterable<string>,
): string | null {
  const subagentId = findSubagentIdForEntry(activities, entryId);
  if (subagentId) {
    return buildSubagentSessionId(rootSessionId, subagentId);
  }
  return null;
}
