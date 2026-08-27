import { computed, nextTick, ref, watch, type ComputedRef, type Ref } from "vue";

import type ChatInputBar from "@/components/chat/ChatInputBar.vue";
import type { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import {
  chatCancel,
  deleteChatSession,
  listChatSessions,
  listCheckpoints,
  branchChatSession,
  setChatSessionArchived,
  setChatSessionWorkspace,
} from "@/services/ipc";
import { estimateMessageTokens } from "@/services/chat/tokenEstimate";
import { useChatStore } from "@/stores/chat";
import {
  clearCurrentWorkspace,
  listWorkspaces,
  switchWorkspace,
  type Workspace,
} from "@/commands/workspace";
import type { CheckpointInfo, ChatMessage, ChatSessionSummary } from "@/types/chat";
import type { WorkbenchLabels } from "./useWorkbenchLabels";

export interface UseWorkbenchSessionsOptions {
  activeSessionId: Ref<string>;
  activeSessionWorkspaceId: Ref<string | null>;
  sessions: Ref<ChatSessionSummary[]>;
  workspaces: Ref<Workspace[]>;
  messages: ComputedRef<ChatMessage[]>;
  labels: WorkbenchLabels["labels"];
  navigationLabels: WorkbenchLabels["navigationLabels"];
  confirmDialogRef: Ref<InstanceType<typeof AppConfirmDialog> | null>;
  inputRef: Ref<InstanceType<typeof ChatInputBar> | null>;
  reviewOpen: Ref<boolean>;
  removePendingInteraction: (sessionId: string, requestId?: string) => boolean;
  clearSessionUnread: (sessionId: string) => void;
}

/**
 * Conversation list, active session/workspace selection, staged-message
 * queue, and checkpoints. Owns everything needed to create, switch to,
 * delete, and send messages within a conversation.
 */
export function useWorkbenchSessions(options: UseWorkbenchSessionsOptions) {
  const {
    activeSessionId,
    activeSessionWorkspaceId,
    sessions,
    workspaces,
    messages,
    labels,
    navigationLabels,
    confirmDialogRef,
    inputRef,
    reviewOpen,
    removePendingInteraction,
    clearSessionUnread,
  } = options;
  const chatStore = useChatStore();

  const sessionsLoading = ref(false);
  const checkpoints = ref<CheckpointInfo[]>([]);
  const pendingStagedEdit = ref<{ sessionId: string; index: number; original: string } | null>(
    null,
  );

  const sessionsWithLiveTokens = computed(() => {
    const base = sessions.value.map((session) => {
      const compose = chatStore.sessionCompose[session.sessionId];
      const workspaceId = session.workspaceId || compose?.draftWorkspaceId || undefined;
      const withWorkspace =
        workspaceId === session.workspaceId ? session : { ...session, workspaceId };
      if (withWorkspace.sessionId !== activeSessionId.value) return withWorkspace;
      return {
        ...withWorkspace,
        estimatedTokens: messages.value.reduce(
          (total, message) => total + estimateMessageTokens(message),
          0,
        ),
      };
    });
    const knownIds = base.map((session) => session.sessionId);
    // Draft-only rows / draft icons — not every keystroke into sessionCompose.
    void chatStore.draftListVersion;
    const draftOnly = chatStore.listDraftOnlySessions(knownIds).map((draft) => ({
      sessionId: draft.sessionId,
      workspaceId: draft.workspaceId,
      preview: draft.preview,
      messageCount: 0,
      turnCount: 0,
      estimatedTokens: 0,
      updatedAt: draft.updatedAt,
    }));
    return [...draftOnly, ...base];
  });

  const draftSessionIds = computed(() => {
    void chatStore.draftListVersion;
    const ids: string[] = [];
    for (const session of sessionsWithLiveTokens.value) {
      if (chatStore.sessionHasDraft(session.sessionId)) {
        ids.push(session.sessionId);
      }
    }
    return ids;
  });
  const quickAskSessions = computed(() =>
    sessionsWithLiveTokens.value.filter((session) => !session.workspaceId),
  );
  const sessionsByWorkspace = computed(() => {
    const result = new Map<string, ChatSessionSummary[]>();
    for (const session of sessionsWithLiveTokens.value) {
      if (!session.workspaceId) continue;
      const items = result.get(session.workspaceId) ?? [];
      items.push(session);
      result.set(session.workspaceId, items);
    }
    return result;
  });
  const hasConversationMessages = computed(() =>
    messages.value.some((message) => message.role === "user" || message.role === "assistant"),
  );
  const sending = computed(
    () =>
      Boolean(chatStore.sending[activeSessionId.value]) ||
      chatStore.hasActiveAssistantResponse(activeSessionId.value),
  );
  const runningSessionIds = computed(() => {
    const ids = new Set(
      Object.entries(chatStore.sending)
        .filter(([, busy]) => Boolean(busy))
        .map(([sessionId]) => sessionId),
    );
    for (const [sessionId, messages] of Object.entries(chatStore.sessions)) {
      if (
        messages.some(
          (message) =>
            String(message.role).toLowerCase() === "assistant" &&
            (message.status === "pending" || message.status === "streaming"),
        )
      ) {
        ids.add(sessionId);
      }
    }
    return [...ids];
  });
  const stagedMessages = computed(() => chatStore.stagedMessages[activeSessionId.value] ?? []);
  const contextNotice = computed(() => chatStore.contextNotices[activeSessionId.value] ?? "");
  const activeAssistantMessageId = computed(
    () =>
      [...messages.value]
        .reverse()
        .find(
          (message) =>
            String(message.role).toLowerCase() === "assistant" &&
            (message.status === "pending" || message.status === "streaming"),
        )?.id ?? "",
  );

  function createSessionId() {
    return `session-${Date.now()}`;
  }

  function sessionsForWorkspace(workspaceId: string) {
    return sessionsByWorkspace.value.get(workspaceId) ?? [];
  }

  async function refreshSessions() {
    sessionsLoading.value = true;
    try {
      const [chatResponse, workspaceResponse] = await Promise.all([
        listChatSessions(),
        listWorkspaces().catch(() => []),
      ]);
      sessions.value = chatResponse.sessions;
      workspaces.value = workspaceResponse;
      if (chatResponse && chatResponse.sessions) {
        chatStore.setStartedSessionIds(chatResponse.sessions.map((session) => session.sessionId));
      }
    } catch (error) {
      console.error("list_chat_sessions failed:", error);
    } finally {
      sessionsLoading.value = false;
    }
  }

  function createConversation(workspaceId: string | null) {
    const sessionId = createSessionId();
    chatStore.setSessionMessages(sessionId, []);
    chatStore.ensureCompose(sessionId);
    chatStore.setComposeDraft(sessionId, "", { workspaceId });
    chatStore.setOverlayDraftSession(sessionId);
    activeSessionId.value = sessionId;
    activeSessionWorkspaceId.value = workspaceId;
    checkpoints.value = [];
    reviewOpen.value = false;
    void nextTick(() => inputRef.value?.focusInput());
  }

  async function createQuickConversation() {
    await clearCurrentWorkspace();
    createConversation(null);
  }

  async function refreshCheckpoints() {
    if (!activeSessionId.value) {
      checkpoints.value = [];
      return;
    }
    try {
      checkpoints.value = await listCheckpoints(activeSessionId.value);
    } catch {
      checkpoints.value = [];
    }
  }

  async function selectConversation(sessionId: string) {
    cancelStagedEdit();
    clearSessionUnread(sessionId);
    const summary = sessions.value.find((session) => session.sessionId === sessionId);
    const compose = chatStore.ensureCompose(sessionId);
    const workspaceId = summary?.workspaceId ?? compose.draftWorkspaceId ?? null;
    const previousWorkspaceId = activeSessionWorkspaceId.value;
    if (workspaceId && compose.draftWorkspaceId !== workspaceId) {
      chatStore.setComposeDraft(sessionId, compose.draft ?? "", { workspaceId });
    }

    // Switch the visible conversation immediately so the UI is not blocked on IPC.
    activeSessionWorkspaceId.value = workspaceId;
    activeSessionId.value = sessionId;
    chatStore.setOverlayDraftSession(sessionId);
    void nextTick(() => inputRef.value?.focusInput());

    // Same workspace: skip the round-trip (and workspaces-changed fan-out).
    if (workspaceId !== previousWorkspaceId) {
      if (workspaceId) {
        await switchWorkspace(workspaceId);
      } else {
        await clearCurrentWorkspace();
      }
    }

    const cached = chatStore.sessions[sessionId];
    const hasCachedMessages = Array.isArray(cached);
    if (hasCachedMessages) {
      // Show memory immediately; refresh from disk/DB in the background.
      void chatStore.loadHistory(sessionId);
    } else {
      await chatStore.loadHistory(sessionId);
    }
    void refreshCheckpoints();
  }

  async function leaveConversation(sessionId: string) {
    pendingStagedEdit.value = null;
    chatStore.removeCompose(sessionId);
    delete chatStore.sessions[sessionId];
    clearSessionUnread(sessionId);
    removePendingInteraction(sessionId);
    await refreshSessions();
    if (activeSessionId.value === sessionId) {
      const next = sessionsWithLiveTokens.value.find(
        (session) => session.sessionId !== sessionId,
      )?.sessionId;
      if (next) await selectConversation(next);
      else await createQuickConversation();
    }
  }

  async function moveSessionToWorkspace(sessionId: string, workspaceId: string) {
    if (!sessionId || !workspaceId) return;
    const summary = sessionsWithLiveTokens.value.find((session) => session.sessionId === sessionId);
    const sourceWorkspaceId = summary?.workspaceId ?? null;
    if (sourceWorkspaceId === workspaceId) return;

    const compose = chatStore.ensureCompose(sessionId);
    const previousWorkspaceId = compose.draftWorkspaceId ?? sourceWorkspaceId;
    chatStore.setComposeDraft(sessionId, compose.draft ?? "", { workspaceId });
    sessions.value = sessions.value.map((session) =>
      session.sessionId === sessionId ? { ...session, workspaceId } : session,
    );

    try {
      await setChatSessionWorkspace(sessionId, workspaceId);
    } catch (error) {
      console.error("set_chat_session_workspace failed:", error);
      chatStore.setComposeDraft(sessionId, compose.draft ?? "", {
        workspaceId: previousWorkspaceId,
      });
      await refreshSessions();
      return;
    }

    await refreshSessions();
    if (activeSessionId.value === sessionId) {
      activeSessionWorkspaceId.value = workspaceId;
      await switchWorkspace(workspaceId);
    }
  }

  async function archiveConversation(sessionId: string) {
    const known = sessions.value.some((session) => session.sessionId === sessionId);
    if (known) {
      try {
        await setChatSessionArchived(sessionId, true);
      } catch (error) {
        console.warn("set_chat_session_archived skipped:", error);
      }
    }
    await leaveConversation(sessionId);
  }

  async function branchConversation(sessionId: string, messageId?: string) {
    if (!sessionId) return;
    try {
      const summary = await branchChatSession(sessionId, messageId);
      const sourceCompose = chatStore.sessionCompose[sessionId];
      if (sourceCompose) {
        chatStore.setCompose(summary.sessionId, {
          chatModel: sourceCompose.chatModel,
          chatModelProvider: sourceCompose.chatModelProvider,
          chatMode: sourceCompose.chatMode === "plan" ? "agent" : sourceCompose.chatMode,
          toolApprovalMode: sourceCompose.toolApprovalMode,
          imageGen: sourceCompose.imageGen,
        });
      } else {
        chatStore.ensureCompose(summary.sessionId);
      }
      chatStore.setComposeDraft(summary.sessionId, "", {
        workspaceId: summary.workspaceId ?? null,
      });
      await refreshSessions();
      await selectConversation(summary.sessionId);
    } catch (error) {
      console.error("branch_chat_session failed:", error);
    }
  }

  async function removeConversation(sessionId: string) {
    const confirmed = await confirmDialogRef.value?.ask({
      title: labels.value.deleteConversation,
      description: labels.value.deleteConfirm,
      confirmLabel: navigationLabels.value.confirmDelete,
      cancelLabel: navigationLabels.value.cancel,
    });
    if (!confirmed) return;
    try {
      await deleteChatSession(sessionId);
    } catch (error) {
      // Draft-only sessions may not exist on the backend yet.
      console.warn("delete_chat_session skipped:", error);
    }
    await leaveConversation(sessionId);
  }

  async function guideStaged(index: number) {
    await chatStore.guideStagedMessage(activeSessionId.value, index);
  }

  function startStagedEdit(index: number) {
    // 若正在编辑另一条，先把上一条原文案放回队列，避免覆盖丢失。
    cancelStagedEdit();
    const sessionId = activeSessionId.value;
    const message = chatStore.stagedMessages[sessionId]?.[index];
    if (!message) {
      return;
    }
    // 先移除队列中的原文案并回填输入框；提交或取消时再放回队列，避免丢消息。
    pendingStagedEdit.value = { sessionId, index, original: message };
    chatStore.removeStagedMessage(sessionId, index);
    inputRef.value?.setMessage(message);
    void nextTick(() => inputRef.value?.focusInput());
  }

  function cancelStagedEdit() {
    const pending = pendingStagedEdit.value;
    if (!pending) {
      return;
    }
    pendingStagedEdit.value = null;
    // 编辑未提交：把原文案放回队列原位。
    chatStore.insertStagedMessage(pending.sessionId, pending.index, pending.original);
  }

  function removeStaged(index: number) {
    chatStore.removeStagedMessage(activeSessionId.value, index);
  }

  async function submitMessage(text: string) {
    const trimmed = text.trim();
    if (!trimmed) {
      cancelStagedEdit();
      return;
    }
    const pending = pendingStagedEdit.value;
    if (pending) {
      // 编辑暂存消息：改完的内容放回队列原位，不直接发送（AI 仍在执行中）。
      pendingStagedEdit.value = null;
      chatStore.insertStagedMessage(pending.sessionId, pending.index, trimmed);
      await refreshSessions();
      return;
    }
    let sessionId = activeSessionId.value;
    if (!sessionId) {
      await createQuickConversation();
      sessionId = activeSessionId.value;
    }
    if (!sessionId) return;
    await chatStore.send(trimmed, sessionId, {
      workspaceId: activeSessionWorkspaceId.value ?? undefined,
      quickAsk: !activeSessionWorkspaceId.value,
    });
    await refreshSessions();
    await refreshCheckpoints();
  }

  async function pauseResponse() {
    const messageId = activeAssistantMessageId.value;
    const sessionId = activeSessionId.value;
    if (!messageId) return;
    chatStore.clearSending(sessionId);
    try {
      await chatCancel({ messageId });
    } catch (error) {
      console.error("chat_cancel failed:", error);
      chatStore.settleInterruptedSession(sessionId);
    }
    // 停止后也继续发送暂存队列（chat-finished cancelled 会再触发一次，flushStaged 自身有防重入）。
    void chatStore.flushStaged(sessionId);
  }

  async function handleRewound(payload: { text: string }) {
    await chatStore.loadHistory(activeSessionId.value);
    await refreshCheckpoints();
    if (payload.text) inputRef.value?.setMessage(payload.text);
  }

  watch(
    () =>
      messages.value
        .map((message) => `${message.id}:${message.status}:${message.toolActivities?.length ?? 0}`)
        .join("|"),
    () => void refreshCheckpoints(),
  );

  return {
    sessionsLoading,
    checkpoints,
    pendingStagedEdit,
    sessionsWithLiveTokens,
    draftSessionIds,
    quickAskSessions,
    sessionsByWorkspace,
    hasConversationMessages,
    sending,
    runningSessionIds,
    stagedMessages,
    contextNotice,
    activeAssistantMessageId,
    createSessionId,
    sessionsForWorkspace,
    refreshSessions,
    createConversation,
    createQuickConversation,
    refreshCheckpoints,
    selectConversation,
    moveSessionToWorkspace,
    archiveConversation,
    branchConversation,
    removeConversation,
    guideStaged,
    startStagedEdit,
    cancelStagedEdit,
    removeStaged,
    submitMessage,
    pauseResponse,
    handleRewound,
  };
}
