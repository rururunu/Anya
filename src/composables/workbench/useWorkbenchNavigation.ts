import { computed, ref, type ComputedRef, type Ref } from "vue";

import { formatSessionPreview } from "@/services/chat/sessionPreview";
import { isSubagentSessionId } from "@/services/chat/subagentSession";
import { tr } from "@/services/i18n";
import { useSubagentSessionStore } from "@/stores/subagentSessions";
import { useSettingStore } from "@/stores/setting";
import type { Workspace } from "@/commands/workspace";
import type { ChatSessionSummary } from "@/types/chat";
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
  sessionsWithLiveTokens: ComputedRef<ChatSessionSummary[]>;
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
    const preview =
      options.sessionsWithLiveTokens.value.find(
        (session) => session.sessionId === options.activeSessionId.value,
      )?.preview || "";
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
    showConversationHeader,
    activeTitle,
  };
}
