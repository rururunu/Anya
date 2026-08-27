/**
 * Context-window usage ring for ChatInputBar.
 * Reflects committed conversation state only — never the composer draft.
 */

import { computed, ref, watch, type Ref } from "vue";
import { useDebounceFn } from "@vueuse/core";
import { storeToRefs } from "pinia";
import { getContextUsage } from "@/services/ipc";
import { estimateMessageTokens } from "@/services/chat/tokenEstimate";
import { tr } from "@/services/i18n";
import { useChatStore } from "@/stores/chat";
import { useSettingStore } from "@/stores/setting";
import type { ContextUsageSnapshot, CapturedContext } from "@/types/chat";

function emptyContextUsage(): ContextUsageSnapshot {
  return {
    usageRatio: 0,
    estimatedTokens: 0,
    contextWindowTokens: 64_000,
    systemPromptTokens: 0,
    toolsTokens: 0,
    messageTokens: 0,
  };
}

export function useContextUsage(options: {
  sessionId: Ref<string> | (() => string);
  capturedContext:
    Ref<CapturedContext | null | undefined> | (() => CapturedContext | null | undefined);
  chatModel: Ref<string>;
}) {
  const chatStore = useChatStore();
  const settingStore = useSettingStore();
  const { language } = storeToRefs(settingStore);

  const sessionIdOf = () =>
    typeof options.sessionId === "function" ? options.sessionId() : options.sessionId.value;
  const capturedContextOf = () =>
    typeof options.capturedContext === "function"
      ? options.capturedContext()
      : options.capturedContext.value;

  const contextUsage = ref<ContextUsageSnapshot>(emptyContextUsage());
  let contextUsageRequestId = 0;

  const conversationTokenCount = computed(() => {
    const sessionMessages = chatStore.sessions[sessionIdOf()] ?? [];
    let total = 0;
    for (const item of sessionMessages) {
      total += estimateMessageTokens(item);
    }
    return total;
  });

  const conversationTokenTitle = computed(() =>
    tr(language.value, "tokens.sessionEstimated", {
      count: new Intl.NumberFormat(language.value).format(conversationTokenCount.value),
    }),
  );

  const sessionCacheUsage = computed(() => chatStore.sessionCacheUsage[sessionIdOf()] ?? undefined);

  const runContextUsageLoad = async (requestId: number) => {
    const sessionId = sessionIdOf();
    const hasConversation = (chatStore.sessions[sessionId] ?? []).some(
      (item) => item.role === "user" || item.role === "assistant",
    );

    if (!hasConversation) {
      contextUsage.value = emptyContextUsage();
      if (sessionId) {
        chatStore.setContextUsage(sessionId, contextUsage.value);
      }
      return;
    }

    try {
      const response = await getContextUsage({
        sessionId: sessionId || undefined,
        context: capturedContextOf() ?? undefined,
        modelId: options.chatModel.value.trim() || undefined,
      });
      if (requestId !== contextUsageRequestId || sessionId !== sessionIdOf()) {
        return;
      }
      contextUsage.value = {
        usageRatio: response.usageRatio,
        estimatedTokens: response.estimatedTokens,
        contextWindowTokens: response.contextWindowTokens,
        systemPromptTokens: response.systemPromptTokens ?? 0,
        environmentTokens: response.environmentTokens ?? 0,
        toolsTokens: response.toolsTokens ?? 0,
        rulesTokens: response.rulesTokens ?? 0,
        memoriesTokens: response.memoriesTokens ?? 0,
        skillsTokens: response.skillsTokens ?? 0,
        mcpTokens: response.mcpTokens ?? 0,
        subagentTokens: response.subagentTokens ?? 0,
        summarizedTokens: response.summarizedTokens ?? 0,
        messageTokens: response.messageTokens ?? 0,
      };
      if (sessionId) {
        chatStore.setContextUsage(sessionId, contextUsage.value);
      }
    } catch (error) {
      console.error("Failed to load context usage:", error);
    }
  };

  const loadContextUsage = useDebounceFn(runContextUsageLoad, 180);

  function refreshContextUsage() {
    contextUsageRequestId += 1;
    return loadContextUsage(contextUsageRequestId);
  }

  function sessionMessagesFingerprint(sessionId: string) {
    const messages = chatStore.sessions[sessionId] ?? [];
    let chars = 0;
    for (const item of messages) {
      chars += item.content.length + (item.reasoning?.length ?? 0);
    }
    return `${messages.length}:${chars}`;
  }

  watch(
    () =>
      [
        sessionIdOf(),
        capturedContextOf(),
        settingStore.largeContextEnabled,
        sessionIdOf() ? sessionMessagesFingerprint(sessionIdOf()) : "",
      ] as const,
    () => {
      void refreshContextUsage();
    },
  );

  watch(
    () => (sessionIdOf() ? chatStore.contextUsage[sessionIdOf()] : undefined),
    (usage) => {
      if (!usage) {
        return;
      }
      contextUsage.value = usage;
    },
  );

  watch(options.chatModel, () => {
    void refreshContextUsage();
  });

  return {
    contextUsage,
    conversationTokenCount,
    conversationTokenTitle,
    sessionCacheUsage,
    refreshContextUsage,
  };
}
