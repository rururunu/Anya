import type { AskUserAnswerItem, ToolActivity } from "@/types/chat";

type RawAskUserAnswer = {
  header?: unknown;
  question?: unknown;
  selected?: unknown;
  userSupplement?: unknown;
  kind?: unknown;
};

export const PLAN_SWITCH_KIND = "planSwitch";
export const PLAN_SWITCH_HEADER = "切换到计划";
export const PLAN_SWITCH_ACCEPT_LABEL = "切换到计划";
export const PLAN_SWITCH_DECLINE_LABEL = "继续用 Agent";

/** ask_user and request_plan_mode both wait on the same picker. */
export function isAskUserTool(name: string | undefined): boolean {
  return name === "ask_user" || name === "request_plan_mode";
}

/** Plan-mode switch ask (timeline tool or picker question). */
export function isPlanSwitchTool(name: string | undefined): boolean {
  return name === "request_plan_mode";
}

/** True when this picker question is the Agent → Plan switch. */
export function isPlanSwitchQuestion(
  question?: { kind?: string; header?: string } | null,
): boolean {
  if (!question) return false;
  if (question.kind === PLAN_SWITCH_KIND) return true;
  return (question.header ?? "").trim() === PLAN_SWITCH_HEADER;
}

/** True when recorded answers came from a plan-switch picker. */
export function isPlanSwitchAsk(items?: AskUserAnswerItem[]): boolean {
  return Boolean(items?.some((item) => isPlanSwitchQuestion(item)));
}

/** Normalize ask-user Q&A rows from a picker payload or persisted tool result. */
export function normalizeAskUserAnswerItems(
  items: RawAskUserAnswer[] | undefined,
): AskUserAnswerItem[] {
  if (!items?.length) return [];
  return items
    .map((item) => ({
      header: String(item.header ?? "").trim() || undefined,
      question: String(item.question ?? "").trim() || undefined,
      kind: String(item.kind ?? "").trim() || undefined,
      selected: Array.isArray(item.selected)
        ? item.selected.map((value) => String(value).trim()).filter(Boolean)
        : [],
      userSupplement: Boolean(item.userSupplement),
    }))
    .filter((item) => item.userSupplement || item.selected.length > 0);
}

/** Parse the JSON payload emitted when the user finishes an ask-user picker. */
export function parseAskUserAnswerItems(raw: string | undefined): AskUserAnswerItem[] {
  if (!raw?.trim()) return [];
  try {
    const parsed = JSON.parse(raw) as { answers?: RawAskUserAnswer[] };
    return normalizeAskUserAnswerItems(parsed.answers);
  } catch {
    return [];
  }
}

/** Answers for one ask_user activity: persisted result, else in-flight staged rows. */
export function askUserAnswersForActivity(
  activity: ToolActivity,
  staged?: AskUserAnswerItem[],
): AskUserAnswerItem[] {
  if (!isAskUserTool(activity.toolName)) return [];
  const fromResult = parseAskUserAnswerItems(activity.result);
  if (fromResult.length) return fromResult;
  return staged ?? [];
}
