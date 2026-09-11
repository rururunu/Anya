import type { AskUserAnswerItem, ToolActivity } from "@/types/chat";

type RawAskUserAnswer = {
  header?: unknown;
  question?: unknown;
  selected?: unknown;
  userSupplement?: unknown;
};

/** Normalize ask-user Q&A rows from a picker payload or persisted tool result. */
export function normalizeAskUserAnswerItems(
  items: RawAskUserAnswer[] | undefined,
): AskUserAnswerItem[] {
  if (!items?.length) return [];
  return items
    .map((item) => ({
      header: String(item.header ?? "").trim() || undefined,
      question: String(item.question ?? "").trim() || undefined,
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
  if (activity.toolName !== "ask_user") return [];
  const fromResult = parseAskUserAnswerItems(activity.result);
  if (fromResult.length) return fromResult;
  return staged ?? [];
}
