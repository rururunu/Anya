import { nextTick, onMounted, onUnmounted, watch, type Ref } from "vue";
import type { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type ChatInputBar from "@/components/chat/ChatInputBar.vue";
import {
  listenAskUser,
  listenChatFinished,
  listenChatSessionTitleUpdated,
  listenChatStarted,
  listenInteractionResolved,
  listenPathPermission,
  listenToolApproval,
  setWindowSessionView,
} from "@/services/ipc";
import { tr } from "@/services/i18n";
import { useAppStore } from "@/stores/app";
import { useChatStore } from "@/stores/chat";
import { useSettingStore } from "@/stores/setting";
import { useUpdaterStore } from "@/stores/updater";
import type { Workspace } from "@/commands/workspace";
import type { CategoryId } from "@/types/setting";
import type { ChatSessionSummary } from "@/types/chat";
import type { PendingInteraction, WorkspacePointerDrag } from "./types";

export interface UseWorkbenchLifecycleOptions {
  appWindow: WebviewWindow;
  activeSessionId: Ref<string>;
  sessions: Ref<ChatSessionSummary[]>;
  workspaces: Ref<Workspace[]>;
  activeSessionWorkspaceId: Ref<string | null>;
  initializing: Ref<boolean>;
  inputRef: Ref<InstanceType<typeof ChatInputBar> | null>;
  settingsOpen: Ref<boolean>;
  openSettings: (category?: CategoryId) => void;
  syncMaximizedState: () => Promise<void>;
  refreshSessions: () => Promise<void>;
  selectConversation: (sessionId: string) => Promise<void>;
  createQuickConversation: () => Promise<void>;
  refreshCheckpoints: () => Promise<void>;
  clearSessionUnread: (sessionId: string) => void;
  markSessionUnread: (sessionId: string) => void;
  isWorkbenchClosed: () => Promise<boolean>;
  showActionableWindowsNotification: (
    sessionId: string,
    title: string,
    body: string,
    persistent?: boolean,
    requestId?: string,
  ) => Promise<void>;
  notifyWhenNotViewed: (
    sessionId: string,
    title: string,
    body: string,
    requestId?: string,
  ) => Promise<boolean>;
  dismissNotificationForInteraction: (requestId?: string, sessionId?: string) => Promise<void>;
  pendingInteractions: Ref<Record<string, PendingInteraction>>;
  setPendingInteraction: (sessionId: string, interaction: PendingInteraction) => void;
  removePendingInteraction: (sessionId: string, requestId?: string) => boolean;
  sessionDisplayName: (sessionId: string) => string;
  updateReviewWidth: () => void;
  handleWorkbenchHotkey: (event: KeyboardEvent) => void;
  workspacePointerDrag: Ref<WorkspacePointerDrag | null>;
  clearWorkspaceLongPress: (drag: WorkspacePointerDrag) => void;
  moveWorkspacePointerDrag: (event: PointerEvent) => void;
  finishWorkspacePointerDrag: (event: PointerEvent) => void;
  cancelWorkspacePointerDrag: (event: PointerEvent) => void;
}

/**
 * Boot sequence (restore last session, fade out the loader) and every
 * window / IPC event listener the workbench keeps alive for its lifetime.
 * Registered here so `Main.vue` only has to call this once.
 */
export function useWorkbenchLifecycle(options: UseWorkbenchLifecycleOptions) {
  const {
    appWindow,
    activeSessionId,
    sessions,
    workspaces,
    activeSessionWorkspaceId,
    initializing,
    inputRef,
    settingsOpen,
    openSettings,
    syncMaximizedState,
    refreshSessions,
    selectConversation,
    createQuickConversation,
    refreshCheckpoints,
    clearSessionUnread,
    markSessionUnread,
    isWorkbenchClosed,
    showActionableWindowsNotification,
    notifyWhenNotViewed,
    dismissNotificationForInteraction,
    pendingInteractions,
    setPendingInteraction,
    removePendingInteraction,
    sessionDisplayName,
    updateReviewWidth,
    handleWorkbenchHotkey,
    workspacePointerDrag,
    clearWorkspaceLongPress,
    moveWorkspacePointerDrag,
    finishWorkspacePointerDrag,
    cancelWorkspacePointerDrag,
  } = options;

  const appStore = useAppStore();
  const chatStore = useChatStore();
  const settingStore = useSettingStore();
  const unlisteners: UnlistenFn[] = [];
  let pendingWorkbenchSessionId = "";

  watch(
    [activeSessionId, settingsOpen],
    ([sessionId, showingSettings]) =>
      void setWindowSessionView(showingSettings ? undefined : sessionId),
    { immediate: true },
  );

  onMounted(async () => {
    await syncMaximizedState();
    unlisteners.push(await appWindow.onResized(() => void syncMaximizedState()));
    unlisteners.push(
      await appWindow.onFocusChanged(({ payload: focused }) => {
        if (focused && !settingsOpen.value) clearSessionUnread(activeSessionId.value);
      }),
    );
    unlisteners.push(
      await listen("open-workbench-settings", () => {
        openSettings(appStore.settingsCategory || "ai");
      }),
    );
    unlisteners.push(
      await listen<string>("workbench-open-session", async (event) => {
        pendingWorkbenchSessionId = event.payload;
        settingsOpen.value = false;
        if (initializing.value) return;
        await refreshSessions();
        await selectConversation(event.payload);
      }),
    );

    try {
      await refreshSessions();
      if (pendingWorkbenchSessionId) await selectConversation(pendingWorkbenchSessionId);
      else if (sessions.value[0]) await selectConversation(sessions.value[0].sessionId);
      else await createQuickConversation();
    } finally {
      // Let the first content frame paint under the loader before fading it out.
      await nextTick();
      await new Promise<void>((resolve) => {
        requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
      });
      initializing.value = false;
    }

    if (pendingWorkbenchSessionId && activeSessionId.value !== pendingWorkbenchSessionId) {
      await refreshSessions();
      await selectConversation(pendingWorkbenchSessionId);
    }

    unlisteners.push(
      await listenChatFinished((payload) => {
        if (!payload.sessionId || payload.finishReason === "cancelled") return;
        if (payload.sessionId === activeSessionId.value) void refreshCheckpoints();
        void (async () => {
          if (!(await isWorkbenchClosed())) {
            clearSessionUnread(payload.sessionId);
            return;
          }
          markSessionUnread(payload.sessionId);
          await showActionableWindowsNotification(
            payload.sessionId,
            tr(settingStore.language, "notification.taskCompleted"),
            sessionDisplayName(payload.sessionId),
          );
        })();
      }),
    );
    unlisteners.push(
      await listenChatSessionTitleUpdated(() => {
        void refreshSessions();
      }),
    );
    unlisteners.push(
      await listenChatStarted(() => {
        void refreshSessions();
      }),
    );
    unlisteners.push(
      await listen("history-updated", async () => {
        await refreshSessions();
        if (
          activeSessionId.value &&
          !sessions.value.some((session) => session.sessionId === activeSessionId.value)
        ) {
          const next = sessions.value[0]?.sessionId;
          if (next) await selectConversation(next);
          else await createQuickConversation();
        }
      }),
    );
    unlisteners.push(
      await listen("workspaces-changed", async () => {
        await refreshSessions();
        if (
          activeSessionWorkspaceId.value &&
          !workspaces.value.some((workspace) => workspace.id === activeSessionWorkspaceId.value)
        ) {
          activeSessionWorkspaceId.value = null;
        }
      }),
    );
    unlisteners.push(
      await listenAskUser((payload) => {
        const sessionId = payload.sessionId || activeSessionId.value;
        setPendingInteraction(sessionId, {
          kind: "ask_user",
          value: { requestId: payload.requestId, questions: payload.questions },
        });
        void notifyWhenNotViewed(
          sessionId,
          tr(settingStore.language, "notification.needsInput"),
          payload.questions[0]?.question || sessionDisplayName(sessionId),
          payload.requestId,
        );
      }),
    );
    unlisteners.push(
      await listenPathPermission((payload) => {
        const sessionId = payload.sessionId || activeSessionId.value;
        setPendingInteraction(sessionId, { kind: "path_permission", value: payload });
        void notifyWhenNotViewed(
          sessionId,
          tr(settingStore.language, "notification.pathPermission"),
          payload.path,
          payload.requestId,
        );
      }),
    );
    unlisteners.push(
      await listenToolApproval((payload) => {
        const sessionId = payload.sessionId || activeSessionId.value;
        setPendingInteraction(sessionId, { kind: "tool_approval", value: payload });
        chatStore.attachToolApprovalPreview(
          sessionId,
          payload.toolName,
          payload.preview ?? null,
          activeSessionId.value,
        );
        void notifyWhenNotViewed(
          sessionId,
          tr(settingStore.language, "notification.approval"),
          payload.title || payload.toolName,
          payload.requestId,
        );
      }),
    );
    unlisteners.push(
      await listenInteractionResolved((payload) => {
        const matchedSessionId = Object.entries(pendingInteractions.value).find(
          ([, interaction]) => interaction.value.requestId === payload.requestId,
        )?.[0];
        void dismissNotificationForInteraction(payload.requestId, matchedSessionId);
        if (!matchedSessionId) return;
        removePendingInteraction(matchedSessionId, payload.requestId);
        if (matchedSessionId === activeSessionId.value)
          void nextTick(() => inputRef.value?.focusInput());
      }),
    );
    unlisteners.push(
      await appWindow.listen("workbench-opened", () => {
        void refreshSessions();
        void inputRef.value?.focusInput();
      }),
    );

    globalThis.addEventListener("resize", updateReviewWidth);
    globalThis.addEventListener("keydown", handleWorkbenchHotkey);
    globalThis.addEventListener("pointermove", moveWorkspacePointerDrag);
    globalThis.addEventListener("pointerup", finishWorkspacePointerDrag);
    globalThis.addEventListener("pointercancel", cancelWorkspacePointerDrag);
    updateReviewWidth();
    useUpdaterStore().startPolling();
  });

  onUnmounted(() => {
    useUpdaterStore().stopPolling();
    if (workspacePointerDrag.value) clearWorkspaceLongPress(workspacePointerDrag.value);
    for (const unlisten of unlisteners) unlisten();
    globalThis.removeEventListener("resize", updateReviewWidth);
    globalThis.removeEventListener("keydown", handleWorkbenchHotkey);
    globalThis.removeEventListener("pointermove", moveWorkspacePointerDrag);
    globalThis.removeEventListener("pointerup", finishWorkspacePointerDrag);
    globalThis.removeEventListener("pointercancel", cancelWorkspacePointerDrag);
  });
}
