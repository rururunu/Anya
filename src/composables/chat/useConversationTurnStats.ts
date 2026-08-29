import type { Ref } from "vue";
import { computed } from "vue";
import type { ChatMessage } from "@/types/chat";
import type { AppLanguage } from "@/types/setting";
import {
  estimateMessageTokens,
  formatTokenCount,
  promptCacheHitPercent,
} from "@/services/chat/tokenEstimate";
import { tr } from "@/services/i18n";
import { useChatStore } from "@/stores/chat";

/** Compact duration: 45.2s or 2m15s */
export function formatTurnDuration(ms: number, language: AppLanguage): string {
  const seconds = Math.max(0, ms) / 1000;
  if (seconds < 60) {
    const rounded = Math.round(seconds * 10) / 10;
    return language === "zh-CN" ? `${rounded} 秒` : `${rounded}s`;
  }
  const whole = Math.round(seconds);
  const minutes = Math.floor(whole / 60);
  const rest = whole % 60;
  return language === "zh-CN" ? `${minutes} 分 ${rest} 秒` : `${minutes}m${rest}s`;
}

function isUserMessage(message: ChatMessage) {
  return String(message.role).toLowerCase() === "user";
}

function isAssistantMessage(message: ChatMessage) {
  const role = String(message.role).toLowerCase();
  return role === "assistant" || role === "agent";
}

function findLatestTurn(messages: ChatMessage[]) {
  let userIndex = -1;
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    if (isUserMessage(messages[index])) {
      userIndex = index;
      break;
    }
  }
  if (userIndex < 0) return null;

  const user = messages[userIndex];
  const assistant = messages.slice(userIndex + 1).find(isAssistantMessage);
  return { user, assistant };
}

export function useConversationTurnStats(options: {
  sessionId: Ref<string>;
  messages: Ref<ChatMessage[]>;
  clock: Ref<number>;
  language: Ref<AppLanguage>;
}) {
  const chatStore = useChatStore();

  const snapshot = computed(() => {
    const sessionId = options.sessionId.value;
    if (!sessionId) return null;

    const turn = findLatestTurn(options.messages.value);
    if (!turn) return null;

    const { user, assistant } = turn;
    const startedAt = user.timestamp;
    const running = assistant?.status === "pending" || assistant?.status === "streaming";
    const endedAt = running ? options.clock.value : assistant?.completedAt;
    const durationMs = startedAt && endedAt && endedAt >= startedAt ? endedAt - startedAt : 0;

    const toolCount =
      assistant?.toolActivities?.filter((activity) => activity.toolName !== "ask_user").length ?? 0;

    const turnMessages = [user, assistant, ...(assistant ? [] : [])].filter(
      (message): message is ChatMessage => Boolean(message),
    );
    const turnTokens = turnMessages.reduce(
      (total, message) => total + estimateMessageTokens(message),
      0,
    );

    const cacheUsage = assistant
      ? chatStore.messageCacheUsage[sessionId]?.[assistant.id]
      : undefined;
    const cacheHit =
      cacheUsage != null
        ? promptCacheHitPercent(cacheUsage.inputTokens, cacheUsage.cacheReadTokens)
        : null;

    const sessionMessages = options.messages.value;
    const sessionTokens = sessionMessages.reduce(
      (total, message) => total + estimateMessageTokens(message),
      0,
    );

    return {
      running,
      durationMs,
      toolCount,
      turnTokens,
      sessionTokens,
      cacheHit,
    };
  });

  const groups = computed(() => {
    const data = snapshot.value;
    const language = options.language.value;
    if (!data || data.durationMs <= 0) return [] as string[];

    const parts: string[] = [];
    parts.push(
      tr(language, data.running ? "turnStatsRunning" : "turnStatsDuration", {
        duration: formatTurnDuration(data.durationMs, language),
      }),
    );

    if (data.toolCount > 0) {
      parts.push(tr(language, "turnStatsTools", { count: data.toolCount }));
    }

    if (data.sessionTokens > 0) {
      parts.push(
        tr(language, "turnStatsSessionTokens", {
          count: formatTokenCount(data.sessionTokens, language),
        }),
      );
    }

    if (data.cacheHit != null) {
      parts.push(tr(language, "tokens.cacheHit", { percent: data.cacheHit }));
    }

    return parts;
  });

  const fullLine = computed(() => groups.value.join(" | "));

  return {
    groups,
    fullLine,
    visible: computed(() => groups.value.length > 0),
  };
}
