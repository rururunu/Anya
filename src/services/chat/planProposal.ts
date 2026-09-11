import type { ChatMessage, TaskItem, ToolActivity } from "@/types/chat";

const PLAN_TASK_TOOLS = new Set(["update_tasks", "todo_write"]);
const MUTATION_KINDS = new Set(["edit", "create", "delete", "move"]);

export type SavedPlanProposal = {
  title?: string;
  path?: string;
  content: string;
};

/** Last save_plan payload on this assistant turn, if any. */
export function savePlanFromMessage(message: ChatMessage): SavedPlanProposal | null {
  const activities = message.toolActivities ?? [];
  for (let i = activities.length - 1; i >= 0; i -= 1) {
    const activity = activities[i];
    if (activity?.toolName !== "save_plan") continue;
    const content = String(activity.arguments?.content ?? "").trim();
    if (!content) continue;
    const title = String(activity.arguments?.title ?? "").trim() || undefined;
    const path = String(activity.arguments?.path ?? "").trim() || undefined;
    return { title, path, content };
  }
  return null;
}

/** First markdown heading in a plan proposal. */
export function extractPlanTitle(content: string): string | undefined {
  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed.startsWith("#")) continue;
    const title = trimmed.replace(/^#+\s*/, "").trim();
    if (title) return title;
  }
  return undefined;
}

/** First prose paragraph, truncated for the Created Plan card. */
export function extractPlanSummary(content: string, maxLen = 160): string {
  const parts: string[] = [];
  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) {
      if (parts.length) break;
      continue;
    }
    if (isMarkdownStructure(trimmed)) {
      if (parts.length) break;
      continue;
    }
    parts.push(trimmed.replace(/\s+/g, " "));
    if (parts.join(" ").length >= 80) break;
  }
  const text = parts.join(" ");
  if (!text) return "";
  if (text.length <= maxLen) return text;
  return `${text.slice(0, Math.max(0, maxLen - 1)).trim()}…`;
}

function isMarkdownStructure(line: string): boolean {
  return (
    line.startsWith("#") ||
    line.startsWith("```") ||
    line.startsWith("|") ||
    line.startsWith(">") ||
    line.startsWith("- ") ||
    line.startsWith("* ") ||
    line.startsWith("• ") ||
    /^\d+[.)]\s/.test(line)
  );
}

export function tasksFromMessage(message: ChatMessage): TaskItem[] {
  const activities = message.toolActivities ?? [];
  for (let i = activities.length - 1; i >= 0; i -= 1) {
    const activity = activities[i];
    if (!activity || !PLAN_TASK_TOOLS.has(activity.toolName)) continue;
    const raw = activity.arguments?.tasks;
    if (!Array.isArray(raw)) continue;
    const tasks = raw.flatMap((value) => {
      if (!value || typeof value !== "object") return [];
      const item = value as Record<string, unknown>;
      const content = String(item.content ?? "").trim();
      if (!content) return [];
      return [
        {
          content,
          status: String(item.status ?? "pending"),
          activeForm:
            typeof item.activeForm === "string"
              ? item.activeForm
              : typeof item.active_form === "string"
                ? item.active_form
                : undefined,
          level: typeof item.level === "number" ? item.level : undefined,
        } satisfies TaskItem,
      ];
    });
    if (tasks.length) return tasks;
  }
  return [];
}

/** Last checklist still present after history is truncated (rewind). */
export function tasksFromHistory(messages: ChatMessage[]): TaskItem[] {
  for (let i = messages.length - 1; i >= 0; i -= 1) {
    const tasks = tasksFromMessage(messages[i]!);
    if (tasks.length) return tasks;
  }
  return [];
}

/** Last save_plan payload still present after history is truncated (rewind). */
export function planFromHistory(messages: ChatMessage[]): SavedPlanProposal | null {
  for (let i = messages.length - 1; i >= 0; i -= 1) {
    const plan = savePlanFromMessage(messages[i]!);
    if (plan) return plan;
  }
  return null;
}

/** Approve-and-execute canned prompt that unlocks writers for one turn. */
export function isPlanExecutePrompt(content: string): boolean {
  const text = content.trim();
  return text.startsWith("计划已批准") || text.startsWith("Plan approved");
}

/** True when this turn already started implementing (not still proposing). */
function messageHasStartedWork(message: ChatMessage): boolean {
  const hadMutations = (message.toolActivities ?? []).some((activity) => {
    if (activity.success === false || activity.status === "error") return false;
    return MUTATION_KINDS.has(String(activity.kind ?? "").toLowerCase());
  });
  if (hadMutations) return true;
  return tasksFromMessage(message).some((task) => {
    const status = String(task.status ?? "pending")
      .trim()
      .toLowerCase();
    return (
      status === "completed" ||
      status === "done" ||
      status === "complete" ||
      status === "in_progress" ||
      status === "active" ||
      status === "running" ||
      status === "cancelled" ||
      status === "canceled"
    );
  });
}

/** Planning turn: checklist without file mutations yet. */
export function isPendingPlanTurn(message: ChatMessage): boolean {
  const tasks = tasksFromMessage(message);
  if (!tasks.length) return false;
  if (messageHasStartedWork(message)) return false;
  return tasks.some((task) => {
    const status = String(task.status ?? "pending").toLowerCase();
    return status === "pending" || status === "";
  });
}

/** Planning turn that should render the Created Plan card in the assistant body. */
export function isCreatedPlanMessage(message: ChatMessage): boolean {
  if (messageHasStartedWork(message)) return false;
  if (savePlanFromMessage(message)) return true;
  return isPendingPlanTurn(message);
}

export function planCardCopy(
  message: ChatMessage,
  fallbackTitle: string,
): { title: string; summary: string } {
  const saved = savePlanFromMessage(message);
  if (saved) {
    return {
      title: saved.title || extractPlanTitle(saved.content) || fallbackTitle,
      summary: extractPlanSummary(saved.content),
    };
  }
  const tasks = tasksFromMessage(message);
  const first = tasks[0]?.content?.trim();
  return {
    title: first && first.length <= 48 ? first : fallbackTitle,
    summary: "",
  };
}

/** Hide save_plan / plan checklists from the work timeline; the Created Plan card owns them. */
export function shouldHidePlanChromeActivity(
  activity: ToolActivity,
  message: ChatMessage,
): boolean {
  if (activity.toolName === "save_plan") return true;
  if (!PLAN_TASK_TOOLS.has(activity.toolName)) return false;
  return Boolean(savePlanFromMessage(message)) || tasksFromMessage(message).length > 0;
}
