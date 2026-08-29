import { SUBAGENT_TOOLS } from "@/services/chat/subagentTools";
import {
  SUBAGENT_SESSION_MARKER,
  buildSubagentSessionId,
  findSubagentIdForEntry,
} from "@/services/chat/subagentSession";
import { tr } from "@/services/i18n";
import type { AppLanguage } from "@/types/setting";
import type { ChatMessage, MessageStatus, ToolActivity } from "@/types/chat";

export type SubagentPanelEntry = {
  entryId: string;
  parentActivityId: string;
  taskIndex: number;
  title: string;
  task: string;
  status: "running" | "done" | "error";
  model?: string;
  children: ToolActivity[];
};

/** Stable UI session id for a sub-agent row (parent activity + optional parallel index). */
export function buildSubagentPanelSessionId(parentSessionId: string, entryId: string): string {
  return `${parentSessionId}${SUBAGENT_SESSION_MARKER}panel-${entryId.replace(/:/g, "--")}`;
}

export function entryIdFromPanelSession(sessionId: string): string | null {
  const marker = `${SUBAGENT_SESSION_MARKER}panel-`;
  const index = sessionId.indexOf(marker);
  if (index < 0) return null;
  return sessionId.slice(index + marker.length).replace(/--/g, ":");
}

export function isSubagentPanelSessionId(sessionId: string): boolean {
  return sessionId.includes(`${SUBAGENT_SESSION_MARKER}panel-`);
}

export function resolvePanelSessionId(
  parentSessionId: string,
  activities: ToolActivity[],
  entryId: string,
): string {
  const subagentId = findSubagentIdForEntry(activities, entryId);
  if (subagentId) {
    return buildSubagentSessionId(parentSessionId, subagentId);
  }
  return buildSubagentPanelSessionId(parentSessionId, entryId);
}

export function listSubagentEntries(
  activities: ToolActivity[],
  language: AppLanguage,
): SubagentPanelEntry[] {
  const entries: SubagentPanelEntry[] = [];
  for (const parent of activities.filter((activity) => SUBAGENT_TOOLS.has(activity.toolName))) {
    entries.push(...entriesForParent(parent, activities, language, entries.length));
  }
  return entries;
}

export function findSubagentEntry(
  activities: ToolActivity[],
  entryId: string,
  language: AppLanguage,
): SubagentPanelEntry | null {
  const entries = listSubagentEntries(activities, language);
  const direct = entries.find((entry) => entry.entryId === entryId);
  if (direct) return direct;
  const byParent = entries.find((entry) => entry.parentActivityId === entryId);
  if (byParent) return byParent;
  return entries.find((entry) => entry.entryId === `${entryId}:0`) ?? null;
}

function entriesForParent(
  parent: ToolActivity,
  allActivities: ToolActivity[],
  language: AppLanguage,
  offset: number,
): SubagentPanelEntry[] {
  const labels = taskLabels(parent);
  const groups = childGroups(parent.id, allActivities);
  const models = taskModels(parent);
  const count = Math.max(1, labels.length, groups.length);
  return Array.from({ length: count }, (_, index) => {
    const entryId = `${parent.id}:${index}`;
    const children = groups[index] ?? [];
    const status = aggregateStatus(parent.status, children);
    const task = labels[index] ?? labels[0] ?? fallbackTask(parent, language);
    return {
      entryId,
      parentActivityId: parent.id,
      taskIndex: index,
      title: shortTaskTitle(task, offset + index, language),
      task,
      status,
      model: models[index] ?? models[0],
      children,
    };
  });
}

export function buildSubagentViewMessage(
  entry: SubagentPanelEntry,
  parent: ToolActivity,
  allActivities: ToolActivity[],
): ChatMessage {
  const childIds = new Set(entry.children.map((activity) => activity.id));
  const scopedActivities = allActivities
    .filter(
      (activity) =>
        childIds.has(activity.id) ||
        entry.children.some((child) => activity.parentActivityId === child.id),
    )
    .map((activity) =>
      childIds.has(activity.id) ? { ...activity, parentActivityId: undefined } : activity,
    );
  return {
    id: `subagent-${entry.entryId}`,
    sessionId: "subagent",
    role: "assistant",
    content: entry.status === "running" ? "" : completionFor(parent, entry.taskIndex),
    toolActivities: scopedActivities,
    status: messageStatus(entry.status),
    timestamp: Date.now(),
  };
}

function childGroups(parentActivityId: string, activities: ToolActivity[]) {
  const groups = new Map<string, ToolActivity[]>();
  for (const activity of activities) {
    if (activity.parentActivityId !== parentActivityId) continue;
    const key = activity.subagentId ?? "default";
    const group = groups.get(key) ?? [];
    group.push(activity);
    groups.set(key, group);
  }
  return [...groups.values()];
}

function aggregateStatus(
  parentStatus: ToolActivity["status"],
  children: ToolActivity[],
): SubagentPanelEntry["status"] {
  if (children.some((child) => child.status === "error")) return "error";
  if (children.some((child) => child.status === "running") || parentStatus === "running") {
    return "running";
  }
  return parentStatus === "error" ? "error" : "done";
}

function taskLabels(activity: ToolActivity) {
  const args = activity.arguments ?? {};
  if (Array.isArray(args.tasks)) {
    return args.tasks
      .map((value) =>
        typeof value === "object" && value != null
          ? String((value as Record<string, unknown>).prompt ?? "").trim()
          : "",
      )
      .filter(Boolean);
  }
  for (const value of [args.description, args.task, args.prompt]) {
    if (typeof value === "string" && value.trim()) return [value.trim()];
  }
  return activity.detail?.trim() ? [activity.detail.trim()] : [];
}

function taskModels(activity: ToolActivity) {
  const args = activity.arguments ?? {};
  if (Array.isArray(args.tasks)) {
    return args.tasks.map((value) =>
      typeof value === "object" && value != null
        ? String((value as Record<string, unknown>).model ?? "").trim() || undefined
        : undefined,
    );
  }
  return typeof args.model === "string" && args.model.trim() ? [args.model.trim()] : [];
}

function fallbackTask(activity: ToolActivity, language: AppLanguage) {
  return activity.title || tr(language, "subagent.executeTask");
}

function shortTaskTitle(prompt: string, index: number, language: AppLanguage) {
  const lines = prompt
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  const heading = lines.find((line) => /^#{1,6}\s+/.test(line));
  const source = heading ?? lines[0] ?? "";
  const cleaned = source
    .replace(/^#{1,6}\s+/, "")
    .replace(/^(?:任务|task|assignment)\s*[:：-]\s*/i, "")
    .replace(/[`*_~]/g, "")
    .trim();
  const prefix = `${tr(language, "subagent.title")} ${index + 1}`;
  const title = cleaned ? `${prefix} · ${cleaned}` : prefix;
  return title.length > 72 ? `${title.slice(0, 71)}...` : title;
}

function completionFor(parent: ToolActivity, taskIndex: number) {
  const result = parent.result?.trim() ?? "";
  if (parent.toolName !== "run_parallel_subagents" || !result) return result;
  const sections = result.split(/^### Task \d+\s*$/gm);
  return sections[taskIndex + 1]?.trim() ?? "";
}

function messageStatus(status: SubagentPanelEntry["status"]): MessageStatus {
  return status === "running" ? "streaming" : status === "error" ? "error" : "done";
}
