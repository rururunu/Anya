import { computed, ref, type ComputedRef, type Ref } from "vue";

import { formatSessionPreview } from "@/services/chat/sessionPreview";
import { isSubagentSessionId } from "@/services/chat/subagentSession";
import { tr } from "@/services/i18n";
import { useChatStore } from "@/stores/chat";
import { useSubagentSessionStore } from "@/stores/subagentSessions";
import { useSettingStore } from "@/stores/setting";
import type { Workspace } from "@/commands/workspace";
import type { CapturedContext, ChatSessionSummary } from "@/types/chat";
import type { CategoryId } from "@/types/setting";
import type { WorkbenchLabels } from "./useWorkbenchLabels";

export type WorkbenchExtensionView = "plugins" | "phone";

export interface UseWorkbenchNavigationOptions {
  settingsOpen: Ref<boolean>;
  reviewOpen: Ref<boolean>;
  closeSettings: () => void;
  openSettingsPanel: (category?: CategoryId) => void;
  activeSessionId: Ref<string>;
  activeSessionWorkspaceId: Ref<string | null>;
  sessions: Ref<ChatSessionSummary[]>;
  sessionsWithLiveTokens: ComputedRef<ChatSessionSummary[]>;
  hasConversationMessages: ComputedRef<boolean>;
  labels: WorkbenchLabels["labels"];
  navigationLabels: WorkbenchLabels["navigationLabels"];
  selectConversation: (sessionId: string) => Promise<void>;
  consumeSuppressedSessionClick: (sessionId: string) => boolean;
  createQuickConversation: () => Promise<void>;
  createWorkspaceConversation: (workspace: Workspace) => Promise<void>;
  createConversation: (workspaceId: string | null) => void;
  branchConversation: (sessionId: string, messageId: string) => Promise<void>;
}

/**
 * Workbench view routing: extension panels, conversation selection, title bar context, and settings entry.
 */
export function useWorkbenchNavigation(options: UseWorkbenchNavigationOptions) {
  const chatStore = useChatStore();
  const subagentSessionStore = useSubagentSessionStore();
  const settingStore = useSettingStore();

  const extensionView = ref<WorkbenchExtensionView | null>(null);

  function openSettings(category?: CategoryId) {
    extensionView.value = null;
    options.openSettingsPanel(category);
  }

  function openExtensionView(view: WorkbenchExtensionView) {
    options.closeSettings();
    extensionView.value = extensionView.value === view ? null : view;
  }

  async function handleSelectConversation(sessionId: string) {
    if (options.consumeSuppressedSessionClick(sessionId)) return;
    extensionView.value = null;
    await options.selectConversation(sessionId);
  }

  function handleCreateQuickConversation() {
    extensionView.value = null;
    return options.createQuickConversation();
  }

  function handleCreateWorkspaceConversation(workspace: Workspace) {
    extensionView.value = null;
    return options.createWorkspaceConversation(workspace);
  }

  function handleBranchMessage(messageId: string) {
    void options.branchConversation(options.activeSessionId.value, messageId);
  }

  function handleShowContext(context: CapturedContext) {
    extensionView.value = null;
    let sessionId = options.activeSessionId.value;
    if (!sessionId) {
      options.createConversation(options.activeSessionWorkspaceId.value);
      sessionId = options.activeSessionId.value;
    }
    if (!sessionId) return;
    chatStore.upsertMessage({
      id: `local-context-${Date.now()}`,
      sessionId,
      role: "assistant",
      content: "",
      environmentContext: context,
      status: "done",
      timestamp: Date.now(),
    });
  }

  const showConversationHeader = computed(
    () => !options.settingsOpen.value && !options.reviewOpen.value && !extensionView.value,
  );

  const activeTitle = computed(() => {
    if (options.settingsOpen.value) return options.labels.value.settings;
    if (extensionView.value === "plugins") return options.navigationLabels.value.plugins;
    if (extensionView.value === "phone") return options.navigationLabels.value.connectPhone;
    if (isSubagentSessionId(options.activeSessionId.value)) {
      const badge = tr(settingStore.language, "subagent.badge");
      const preview =
        subagentSessionStore.records[options.activeSessionId.value]?.preview ||
        options.sessionsWithLiveTokens.value.find(
          (session) => session.sessionId === options.activeSessionId.value,
        )?.preview ||
        "";
      const title = formatSessionPreview(preview) || options.labels.value.untitled;
      return title.startsWith(badge) ? title : `${badge} ${title}`;
    }
    if (!options.hasConversationMessages.value) return options.labels.value.untitled;
    const preview =
      options.sessions.value.find((session) => session.sessionId === options.activeSessionId.value)
        ?.preview || "";
    return formatSessionPreview(preview) || options.labels.value.untitled;
  });

  return {
    extensionView,
    openSettings,
    openExtensionView,
    handleSelectConversation,
    handleCreateQuickConversation,
    handleCreateWorkspaceConversation,
    handleBranchMessage,
    handleShowContext,
    showConversationHeader,
    activeTitle,
  };
}
