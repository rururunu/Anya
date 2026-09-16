import { computed, ref, type Component } from "vue";
import { storeToRefs } from "pinia";
import {
  ListChecks,
  MessageCircle,
  Paintbrush,
  Shield,
  ShieldOff,
  ShieldQuestion,
  Sparkle,
} from "@lucide/vue";
import {
  conversationConsumedTokens,
  formatTokenCount,
  promptCacheHitPercent,
  promptTokenTotal,
} from "@/services/chat/tokenEstimate";
import { tr } from "@/services/i18n";
import { setPlanMode } from "@/services/ipc";
import { getModelDisplayLabel, getModelIcon } from "@/lib/providerIcons";
import {
  findModelEntry,
  getActiveThinkingVariant,
  getThinkingTierOptions,
  isModelEntrySelected,
  localizeThinkingTierLabel,
} from "@/lib/modelThinking";
import {
  effectiveReasoningEffort,
  effortOptionsForControl,
  isReasoningEffort,
  resolveReasoningControl,
} from "@/lib/reasoningControl";
import { useChatStore } from "@/stores/chat";
import { useChatModelStore } from "@/stores/chatModel";
import { useSettingStore } from "@/stores/setting";
import type { ChatModelInfo, ContextUsageSnapshot } from "@/types/chat";
import {
  localizedOptionLabel,
  normalizeChatMode,
  reasoningEffortOptions,
  toolApprovalModeOptions,
  type ChatMode,
  type ToolApprovalMode,
} from "@/types/setting";

function emptyUsage(): ContextUsageSnapshot {
  return {
    usageRatio: 0,
    estimatedTokens: 0,
    contextWindowTokens: 0,
  };
}

function chatModeIcon(mode: ChatMode): Component {
  switch (mode) {
    case "ask":
      return MessageCircle;
    case "plan":
      return ListChecks;
    case "image":
      return Paintbrush;
    default:
      return Sparkle;
  }
}

function approvalIcon(mode: ToolApprovalMode): Component {
  switch (mode) {
    case "ask":
      return ShieldQuestion;
    case "alwaysAllow":
      return ShieldOff;
    default:
      return Shield;
  }
}

/** Read-only composer chrome (mode, model, approval, tokens) for a session. */
export function useComposerFooterDisplay(sessionId: () => string) {
  const chatStore = useChatStore();
  const chatModelStore = useChatModelStore();
  const settingStore = useSettingStore();
  const { language } = storeToRefs(settingStore);

  const compose = computed(() => chatStore.sessionCompose[sessionId()]);
  const chatModel = computed(
    () => compose.value?.chatModel?.trim() || settingStore.chatModel?.trim() || "",
  );
  const chatModelProvider = computed(
    () => compose.value?.chatModelProvider?.trim() || settingStore.chatModelProvider?.trim() || "",
  );
  const chatMode = computed(() =>
    normalizeChatMode(compose.value?.chatMode ?? settingStore.chatMode),
  );
  const toolApprovalMode = computed(
    () => compose.value?.toolApprovalMode ?? settingStore.toolApprovalMode ?? "ask",
  );

  const currentModelEntry = computed(() =>
    findModelEntry(chatModelStore.models, chatModel.value, chatModelProvider.value),
  );

  const chatModeLabel = computed(() => {
    switch (chatMode.value) {
      case "ask":
        return tr(language.value, "chatModeAsk");
      case "plan":
        return tr(language.value, "chatModePlan");
      case "image":
        return tr(language.value, "chatModeImage");
      default:
        return tr(language.value, "chatModeAgent");
    }
  });

  const modelDisplayName = computed(() => {
    const current = chatModel.value;
    if (!current || (chatModelStore.models.length === 0 && !chatModelStore.loading)) {
      return tr(language.value, "chooseModel");
    }
    const match = currentModelEntry.value;
    if (!match && chatModelStore.models.length === 0) {
      return tr(language.value, "chooseModel");
    }
    return getModelDisplayLabel(match ?? { id: current, provider: "", displayName: undefined });
  });

  const modelProviderIcon = computed(() => {
    const entry = currentModelEntry.value;
    return entry ? getModelIcon(entry) : null;
  });

  const reasoningControl = computed(() =>
    resolveReasoningControl({
      modelId: chatModel.value,
      providerId: chatModelProvider.value,
      entry: currentModelEntry.value,
      customProviders: settingStore.customProviders,
    }),
  );

  const showThinkingTier = computed(() => reasoningControl.value.kind !== "none");

  const thinkingTierLabel = computed(() => {
    const control = reasoningControl.value;
    if (control.kind === "effort") {
      const effort = effectiveReasoningEffort(settingStore.reasoningEffort, control);
      const option = reasoningEffortOptions.find((item) => item.value === effort);
      return option ? localizedOptionLabel(option, language.value) : "";
    }
    const entry = currentModelEntry.value;
    if (!entry) return "";
    const active = getActiveThinkingVariant(entry, chatModel.value);
    return active ? localizeThinkingTierLabel(active.label, language.value) : "";
  });

  const approvalModeLabel = computed(() => {
    const option = toolApprovalModeOptions.find((item) => item.value === toolApprovalMode.value);
    return option
      ? localizedOptionLabel(option, language.value)
      : tr(language.value, "toolApprovalMode");
  });

  const conversationTokenCount = computed(() => {
    const id = sessionId();
    return conversationConsumedTokens(
      chatStore.sessions[id] ?? [],
      chatStore.sessionConsumedTokens[id] ?? 0,
    );
  });

  const conversationTokenTitle = computed(() =>
    tr(language.value, "tokens.sessionEstimated", {
      count: new Intl.NumberFormat(language.value).format(conversationTokenCount.value),
    }),
  );

  const sessionCacheUsage = computed(() => chatStore.sessionCacheUsage[sessionId()] ?? undefined);

  const cacheHitPercent = computed(() => {
    const usage = sessionCacheUsage.value;
    if (!usage) return null;
    return promptCacheHitPercent(usage.inputTokens, usage.cacheReadTokens);
  });

  const cacheHitTitle = computed(() => {
    const usage = sessionCacheUsage.value;
    const percent = cacheHitPercent.value;
    if (!usage || percent == null) return "";
    const prompt = promptTokenTotal(usage.inputTokens, usage.cacheReadTokens);
    return tr(language.value, "tokens.cacheHitTitle", {
      percent,
      cached: formatTokenCount(usage.cacheReadTokens, language.value),
      prompt: formatTokenCount(prompt, language.value),
    });
  });

  const contextUsage = computed(() => chatStore.contextUsage[sessionId()] ?? emptyUsage());

  const chatModeOptions = computed(() => [
    { id: "ask", label: tr(language.value, "chatModeAsk"), icon: MessageCircle },
    { id: "agent", label: tr(language.value, "chatModeAgent"), icon: Sparkle },
    { id: "plan", label: tr(language.value, "chatModePlan"), icon: ListChecks },
    { id: "image", label: tr(language.value, "chatModeImage"), icon: Paintbrush },
  ]);
  const approvalOptions = computed(() =>
    toolApprovalModeOptions.map((option) => ({
      id: option.value,
      label: localizedOptionLabel(option, language.value),
      icon: approvalIcon(option.value),
    })),
  );
  const availableModels = computed(() => {
    const models = [...chatModelStore.models];
    const current = chatModel.value;
    if (
      current &&
      models.length > 0 &&
      !models.some((model) => isModelEntrySelected(model, current, chatModelProvider.value))
    ) {
      models.unshift({ id: current, ownedBy: "", provider: chatModelProvider.value });
    }
    return models;
  });
  const thinkingOptions = computed(() => {
    const control = reasoningControl.value;
    if (control.kind === "effort") {
      return effortOptionsForControl(control).map((option) => ({
        id: option.value,
        label: localizedOptionLabel(option, language.value),
      }));
    }
    const entry = currentModelEntry.value;
    if (!entry || control.kind !== "variants") return [];
    return getThinkingTierOptions(entry).map((variant) => ({
      id: variant.id,
      label: localizeThinkingTierLabel(variant.label, language.value),
    }));
  });
  const thinkingSelectedId = computed(() => {
    if (reasoningControl.value.kind === "effort") {
      return effectiveReasoningEffort(settingStore.reasoningEffort, reasoningControl.value);
    }
    return chatModel.value;
  });

  const modelPickerProvider = ref<string | null>(null);
  const pickerIndex = ref(0);

  function patchCompose(
    patch: Partial<{
      chatModel: string;
      chatModelProvider: string;
      chatMode: ChatMode;
      toolApprovalMode: ToolApprovalMode;
    }>,
  ) {
    const id = sessionId();
    if (!id) return;
    chatStore.ensureCompose(id);
    chatStore.setCompose(id, patch);
  }

  function selectChatMode(mode: string) {
    const next = normalizeChatMode(mode);
    if (next === chatMode.value) return;
    patchCompose({ chatMode: next });
    const id = sessionId();
    if (next === "plan" && id) {
      chatStore.setSessionRejectedPlanFingerprint(id, null);
      void setPlanMode(id, true, "manual")
        .then(() => {
          chatStore.setSessionPlanMode(id, true);
          chatStore.setSessionPlanTrigger(id, "manual");
        })
        .catch(() => undefined);
    }
  }

  function selectApprovalMode(mode: string) {
    if (mode !== "ask" && mode !== "auto" && mode !== "alwaysAllow") return;
    if (mode === toolApprovalMode.value) return;
    patchCompose({ toolApprovalMode: mode });
  }

  function selectModel(entry: ChatModelInfo) {
    if (entry.id === chatModel.value && entry.provider === chatModelProvider.value) return;
    patchCompose({ chatModel: entry.id, chatModelProvider: entry.provider });
    modelPickerProvider.value = null;
  }

  function applyThinkingTier(variantId: string) {
    if (reasoningControl.value.kind === "effort") {
      if (!isReasoningEffort(variantId) || variantId === settingStore.reasoningEffort) return;
      void settingStore.update({ reasoningEffort: variantId });
      return;
    }
    if (variantId === chatModel.value) return;
    patchCompose({ chatModel: variantId, chatModelProvider: chatModelProvider.value });
  }

  function ensureModels() {
    if (chatModelStore.models.length === 0 && !chatModelStore.loading) {
      void chatModelStore.fetch();
    }
  }

  return {
    language,
    chatMode,
    chatModeLabel,
    chatModeIcon: computed(() => chatModeIcon(chatMode.value)),
    chatModeOptions,
    modelDisplayName,
    modelProviderIcon,
    chatModel,
    chatModelProvider,
    availableModels,
    modelPickerProvider,
    pickerIndex,
    showThinkingTier,
    thinkingTierLabel,
    thinkingOptions,
    thinkingSelectedId,
    approvalModeLabel,
    approvalIcon: computed(() => approvalIcon(toolApprovalMode.value)),
    approvalOptions,
    toolApprovalMode,
    showApproval: computed(() => chatMode.value !== "ask"),
    conversationTokenCount,
    conversationTokenTitle,
    cacheHitPercent,
    cacheHitTitle,
    contextUsage,
    formatTokenCount,
    chatModelStore,
    selectChatMode,
    selectApprovalMode,
    selectModel,
    applyThinkingTier,
    ensureModels,
  };
}
