/**
 * History merge / settle helpers for the chat Pinia store.
 * Pure functions only — no Pinia state.
 */

import type { ChatMessage, MessageCacheUsage, SessionCacheUsage } from "@/types/chat";

/** Cheap equality fingerprint so loadHistory can skip no-op setSessionMessages. */
export function messagesHistoryFingerprint(messages: ChatMessage[]): string {
  let out = `${messages.length}`;
  for (const message of messages) {
    out += `|${message.id}:${message.status}:${message.content.length}:${message.reasoning?.length ?? 0}:${message.toolActivities?.length ?? 0}:${message.workTimeline?.length ?? 0}:${message.completedAt ?? ""}:${message.activityStatus ?? ""}`;
  }
  return out;
}

export function cacheUsagesFromHistory(
  usages: MessageCacheUsage[] | undefined,
): Record<string, SessionCacheUsage> {
  const next: Record<string, SessionCacheUsage> = {};
  for (const usage of usages ?? []) {
    if (!usage.messageId) continue;
    next[usage.messageId] = {
      inputTokens: usage.inputTokens,
      cacheReadTokens: usage.cacheReadTokens,
    };
  }
  return next;
}

/** Mark crash-orphaned in-flight rows so the UI is not stuck "executing". */
export function settleInterruptedMessages(messages: ChatMessage[]): ChatMessage[] {
  return messages.map((message) => {
    const statusStuck = message.status === "pending" || message.status === "streaming";
    const toolsStuck = message.toolActivities?.some((activity) => activity.status === "running");
    if (!statusStuck && !toolsStuck) {
      return message;
    }
    return {
      ...message,
      status: statusStuck ? "cancelled" : message.status,
      completedAt: message.completedAt ?? Date.now(),
      activityStatus: undefined,
      estimatedTokens: undefined,
      toolActivities: message.toolActivities?.map((activity) =>
        activity.status === "running"
          ? {
              ...activity,
              status: "error" as const,
              success: false,
              result: activity.result?.trim() ? activity.result : "interrupted",
            }
          : activity,
      ),
    };
  });
}

/** Merge a persisted history snapshot into an actively streaming session.
 * The database can lag behind realtime events, so it must never replace newer
 * optimistic/streaming messages that only exist in memory yet. */
export function mergeActiveHistory(persisted: ChatMessage[], live: ChatMessage[]): ChatMessage[] {
  const liveById = new Map(live.map((message) => [message.id, message]));
  const persistedIds = new Set(persisted.map((message) => message.id));
  const merged = persisted.map((stored) => {
    const current = liveById.get(stored.id);
    if (!current) {
      return stored;
    }

    const currentIsActive = current.status === "pending" || current.status === "streaming";
    const currentHasNewerContent =
      current.content.length > stored.content.length ||
      (current.reasoning?.length ?? 0) > (stored.reasoning?.length ?? 0) ||
      (current.toolActivities?.length ?? 0) > (stored.toolActivities?.length ?? 0) ||
      (current.workTimeline?.length ?? 0) > (stored.workTimeline?.length ?? 0);

    return currentIsActive || currentHasNewerContent ? current : stored;
  });

  // Realtime events may have created messages that the history query has not
  // persisted yet. Keep them in their existing order at the end of the turn.
  for (const message of live) {
    if (!persistedIds.has(message.id)) {
      merged.push(message);
    }
  }
  return merged;
}
