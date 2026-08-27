/**
 * Stream / work-timeline helpers for the chat Pinia store.
 */

import type { ChatMessage, WorkTimelineItem } from "@/types/chat";

/**
 * Append a text chunk (reasoning or regular content) to the work timeline,
 * merging into the trailing segment when it's the same kind so consecutive
 * deltas don't fragment into one segment per network chunk. Tool activities
 * are inserted separately (see `upsertToolActivity`), so a new segment only
 * starts here once a tool call has broken the run of same-kind text.
 */
export function appendTimelineText(
  timeline: WorkTimelineItem[] | undefined,
  chunk: string,
  kind: "reasoning" | "content",
): WorkTimelineItem[] {
  const next = [...(timeline ?? [])];
  const last = next[next.length - 1];
  if (last?.type === kind) {
    next[next.length - 1] = { ...last, content: last.content + chunk };
  } else {
    next.push({
      type: kind,
      id: `${kind}-${Date.now()}-${next.length}`,
      content: chunk,
    });
  }
  return next;
}

export function findLastMessageIndex(
  messages: ChatMessage[],
  predicate: (message: ChatMessage) => boolean,
) {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    if (predicate(messages[index])) return index;
  }
  return -1;
}
