import { defineStore } from "pinia";

import { chat, chatHistory } from "@/services/ipc";
import { createLogger } from "@/services/logger";
import { useSettingStore } from "./setting";
import { useChatModelStore } from "./chatModel";
import {
  normalizeChatStarted,
  normalizeMessage,
  normalizeRole,
  resolveSessionId,
  type RawChatStarted,
} from "@/services/chat/normalize";
import { accumulateCacheUsage, estimateMessageTokens } from "@/services/chat/tokenEstimate";
import {
  CONFIGURE_PROVIDER_MARKER,
  isConfigureProviderError,
} from "@/services/chat/ensureDefaultModel";
import { isKnownModelSelection } from "@/lib/modelThinking";
import { tr } from "@/services/i18n";
import {
  defaultImageGenCompose,
  imageGenComposeEqual,
  imageGenPayload,
  normalizeImageGenCompose,
} from "@/services/chat/imageGenMode";
import type {
  AskUserAnswerItem,
  ChatMessage,
  ContextUsageSnapshot,
  FileOfferEvent,
  SessionCacheUsage,
  SharedFileOffer,
  SharedUrlOffer,
  TaskItem,
  ToolActivity,
  ToolPreviewPayload,
  UrlOfferEvent,
} from "@/types/chat";
import { normalizeChatMode } from "@/types/setting";
import {
  composeCache,
  defaultCompose,
  flushPersistComposeCache,
  loadComposeCache,
  loadRejectedPlanFingerprints,
  persistComposeCache,
  persistRejectedPlanFingerprints,
  sanitizeCompose,
  scheduleDraftListBump,
  schedulePersistComposeCache,
  syncComposeToRemote,
  type SessionCompose,
} from "./chatCompose";
import {
  cacheUsagesFromHistory,
  mergeActiveHistory,
  messagesHistoryFingerprint,
  settleInterruptedMessages,
} from "./chatHistory";
import { appendTimelineText, findLastMessageIndex } from "./chatStream";
import { useChatSessionsStore } from "./chatSessions";

function sessionsStore() {
  return useChatSessionsStore();
}

export type { SessionCompose } from "./chatCompose";
export { defaultCompose } from "./chatCompose";
export {
  messagesHistoryFingerprint,
  settleInterruptedMessages,
  mergeActiveHistory,
} from "./chatHistory";

const log = createLogger("chat-store");

/**
 * Thin UI store — 按 session 镜像 AI Runtime 状态。
 */
export const useChatStore = defineStore("chat", {
  state: () => ({
    sending: {} as Record<string, boolean>,
    /** Per-conversation compose settings (model / mode / approval / draft). */
    sessionCompose: {} as Record<string, SessionCompose>,
    /** User messages typed while a turn is executing — held until the guide
     * button is clicked (inject into the running turn) or the turn finishes
     * (auto-send as the next turn). */
    stagedMessages: {} as Record<string, string[]>,
    /** Prevent duplicate finish events from dispatching multiple queued turns. */
    stagedDispatching: {} as Record<string, boolean>,
    contextNotices: {} as Record<string, string | undefined>,
    contextUsage: {} as Record<string, ContextUsageSnapshot | undefined>,
    /** Aggregated DeepSeek prompt-cache snapshot per conversation. */
    sessionCacheUsage: {} as Record<string, SessionCacheUsage | undefined>,
    /** Prompt-cache totals keyed by assistant message id. */
    messageCacheUsage: {} as Record<string, Record<string, SessionCacheUsage>>,
    /** Live in-session task list from update_tasks. */
    sessionTasks: {} as Record<string, TaskItem[]>,
    /** Session plan-mode gate (writer tools blocked until approve). */
    sessionPlanMode: {} as Record<string, boolean>,
    /** How the active plan was entered. Auto plans (agent complexity
     * detection) get the 30s auto-execute window; manual plans always wait. */
    sessionPlanTrigger: {} as Record<string, "auto" | "manual">,
    /** Structure fingerprint of a plan whose auto-execute was rejected. Same
     * checklist stays manual; a new/updated plan clears this and may countdown.
     * Persisted so a restart does not revive the auto-execute countdown. */
    sessionRejectedPlanFingerprint: loadRejectedPlanFingerprints() as Record<string, string>,
  }),
  getters: {
    sessions(): Record<string, ChatMessage[]> {
      return sessionsStore().sessions;
    },
    startedSessionIds(): Record<string, boolean> {
      return sessionsStore().startedSessionIds;
    },
    overlayDraftSessionId(): string {
      return sessionsStore().overlayDraftSessionId;
    },
    draftListVersion(): number {
      return sessionsStore().draftListVersion;
    },
    overlayMessages(): ChatMessage[] {
      return sessionsStore().overlayMessages;
    },
    overlayContextNotice(): string | undefined {
      const sessionId = sessionsStore().overlayDraftSessionId;
      if (!sessionId) {
        return undefined;
      }
      return this.contextNotices[sessionId];
    },
    overlayContextUsage(): ContextUsageSnapshot | undefined {
      const sessionId = sessionsStore().overlayDraftSessionId;
      if (!sessionId) {
        return undefined;
      }
      return this.contextUsage[sessionId];
    },
  },
  actions: {
    setOverlayDraftSession(sessionId: string) {
      sessionsStore().setOverlayDraftSession(sessionId);
    },
    setStartedSessionIds(ids: string[]) {
      sessionsStore().setStartedSessionIds(ids);
    },
    markSessionStarted(sessionId: string) {
      sessionsStore().markSessionStarted(sessionId);
    },
    /** Return the conversation's own compose settings, creating them on first
     * open by inheriting the last used conversation (or app defaults). */
    ensureCompose(sessionId: string): SessionCompose {
      loadComposeCache();
      if (!sessionId) {
        return defaultCompose();
      }
      const settingStore = useSettingStore();
      const existing = this.sessionCompose[sessionId];
      const cached = composeCache.entries[sessionId];

      // A compose record is a per-conversation snapshot. Once created or
      // restored, it must never be recomputed from another conversation.
      let resolved: SessionCompose;
      if (existing) {
        resolved = existing;
      } else if (cached) {
        resolved = sanitizeCompose(cached);
        this.sessionCompose = { ...this.sessionCompose, [sessionId]: resolved };
      } else if (sessionsStore().startedSessionIds[sessionId]) {
        resolved = sanitizeCompose({
          chatModel: settingStore.chatModel ?? "",
          chatModelProvider: settingStore.chatModelProvider ?? "",
          chatMode: settingStore.chatMode ?? "agent",
          toolApprovalMode: settingStore.toolApprovalMode ?? "ask",
          imageGen: defaultImageGenCompose(),
          draft: "",
        });
        this.sessionCompose = { ...this.sessionCompose, [sessionId]: resolved };
        composeCache.entries[sessionId] = resolved;
        persistComposeCache();
      } else {
        const source =
          composeCache.last && composeCache.entries[composeCache.last]
            ? composeCache.entries[composeCache.last]
            : undefined;
        resolved = sanitizeCompose({
          chatModel: source?.chatModel ?? settingStore.chatModel ?? "",
          chatModelProvider: source?.chatModelProvider ?? settingStore.chatModelProvider ?? "",
          // Plan is session-gated live state — never inherit a stale plan chip.
          chatMode: source?.chatMode === "plan" ? "agent" : normalizeChatMode(source?.chatMode),
          toolApprovalMode: source?.toolApprovalMode ?? settingStore.toolApprovalMode ?? "ask",
          imageGen: normalizeImageGenCompose(source?.imageGen),
          draft: "",
        });
        this.sessionCompose = { ...this.sessionCompose, [sessionId]: resolved };
        composeCache.entries[sessionId] = resolved;
        composeCache.last = sessionId;
        persistComposeCache();
      }
      // Keep the gateway mirror warm so Companion can read the desktop model
      // even if this session was never edited after the last app launch.
      void syncComposeToRemote(sessionId, resolved);
      return resolved;
    },
    /** Persist one conversation's option change without touching others. */
    setCompose(
      sessionId: string,
      patch: Partial<
        Pick<
          SessionCompose,
          "chatModel" | "chatModelProvider" | "chatMode" | "toolApprovalMode" | "imageGen"
        >
      >,
    ) {
      if (!sessionId) {
        return;
      }
      const current = this.ensureCompose(sessionId);
      const next = sanitizeCompose({ ...current, ...patch });
      if (
        current.chatModel === next.chatModel &&
        current.chatModelProvider === next.chatModelProvider &&
        current.chatMode === next.chatMode &&
        current.toolApprovalMode === next.toolApprovalMode &&
        imageGenComposeEqual(current.imageGen, next.imageGen)
      ) {
        return;
      }
      this.sessionCompose = { ...this.sessionCompose, [sessionId]: next };
      composeCache.entries[sessionId] = next;
      composeCache.last = sessionId;
      flushPersistComposeCache();
      void syncComposeToRemote(sessionId, next);
    },
    /** Apply compose patch originating from Companion (skip remote echo). */
    applyComposeFromRemote(
      sessionId: string,
      patch: Partial<
        Pick<
          SessionCompose,
          "chatModel" | "chatModelProvider" | "chatMode" | "toolApprovalMode" | "imageGen"
        >
      >,
    ) {
      if (!sessionId) {
        return;
      }
      const current = this.ensureCompose(sessionId);
      const next = sanitizeCompose({ ...current, ...patch });
      if (
        current.chatModel === next.chatModel &&
        current.chatModelProvider === next.chatModelProvider &&
        current.chatMode === next.chatMode &&
        current.toolApprovalMode === next.toolApprovalMode &&
        imageGenComposeEqual(current.imageGen, next.imageGen)
      ) {
        return;
      }
      this.sessionCompose = { ...this.sessionCompose, [sessionId]: next };
      composeCache.entries[sessionId] = next;
      composeCache.last = sessionId;
      flushPersistComposeCache();
    },
    /** Persist the input draft for one conversation (debounced by callers). */
    setComposeDraft(
      sessionId: string,
      draft: string,
      options?: { workspaceId?: string | null; persistImmediate?: boolean },
    ) {
      if (!sessionId) {
        return;
      }
      const current = this.ensureCompose(sessionId);
      const trimmed = draft.trim();
      const hadDraft = Boolean(current.draft?.trim());
      const hasDraft = Boolean(trimmed);
      const prevWorkspaceId = current.draftWorkspaceId ?? null;
      const nextWorkspaceId =
        options && "workspaceId" in options ? (options.workspaceId ?? null) : prevWorkspaceId;
      if (current.draft === draft && prevWorkspaceId === nextWorkspaceId) {
        if (options?.persistImmediate) {
          flushPersistComposeCache();
        }
        return;
      }

      // Mutate in place so replacing sessionCompose does not invalidate the
      // whole sidebar on every keystroke.
      current.draft = draft;
      current.draftUpdatedAt = hasDraft ? Date.now() : undefined;
      if (options && "workspaceId" in options) {
        current.draftWorkspaceId = options.workspaceId ?? null;
      }
      composeCache.entries[sessionId] = { ...current };
      if (options?.persistImmediate) {
        flushPersistComposeCache();
      } else {
        schedulePersistComposeCache(1000);
      }

      const messages = sessionsStore().sessions[sessionId] ?? [];
      const isDraftOnlySession =
        !sessionsStore().startedSessionIds[sessionId] &&
        !messages.some((item) => item.role === "user" || item.role === "assistant");
      // Draft icon / draft-only row membership — immediate on presence flip.
      if (hadDraft !== hasDraft || prevWorkspaceId !== nextWorkspaceId) {
        sessionsStore().bumpDraftListVersion();
      } else if (isDraftOnlySession && hasDraft) {
        // Preview text for brand-new chats: refresh at most ~1/s.
        scheduleDraftListBump(() => {
          sessionsStore().bumpDraftListVersion();
        });
      }
    },
    sessionHasDraft(sessionId: string): boolean {
      return sessionsStore().sessionHasDraft(sessionId, this.sessionCompose);
    },
    listDraftOnlySessions(knownSessionIds: Iterable<string>) {
      return sessionsStore().listDraftOnlySessions(knownSessionIds, this.sessionCompose);
    },
    /** Drop the compose record when a conversation is deleted. */
    removeCompose(sessionId: string) {
      if (!sessionId) {
        return;
      }
      const next = { ...this.sessionCompose };
      delete next[sessionId];
      this.sessionCompose = next;
      delete composeCache.entries[sessionId];
      if (composeCache.last === sessionId) {
        composeCache.last = "";
      }
      flushPersistComposeCache();
      sessionsStore().bumpDraftListVersion();
    },
    setContextNotice(sessionId: string, message: string | undefined) {
      if (!sessionId) {
        return;
      }
      this.contextNotices = {
        ...this.contextNotices,
        [sessionId]: message,
      };
    },
    setSessionTasks(sessionId: string, tasks: TaskItem[]) {
      if (!sessionId) {
        return;
      }
      this.sessionTasks = {
        ...this.sessionTasks,
        [sessionId]: tasks,
      };
    },
    clearSessionTasks(sessionId: string) {
      if (!sessionId || !(sessionId in this.sessionTasks)) {
        return;
      }
      const next = { ...this.sessionTasks };
      delete next[sessionId];
      this.sessionTasks = next;
    },
    setSessionPlanMode(sessionId: string, active: boolean) {
      if (!sessionId) {
        return;
      }
      if (Boolean(this.sessionPlanMode[sessionId]) === active) {
        return;
      }
      this.sessionPlanMode = {
        ...this.sessionPlanMode,
        [sessionId]: active,
      };
    },
    setSessionPlanTrigger(sessionId: string, trigger: "auto" | "manual") {
      if (!sessionId) {
        return;
      }
      if (this.sessionPlanTrigger[sessionId] === trigger) {
        return;
      }
      this.sessionPlanTrigger = {
        ...this.sessionPlanTrigger,
        [sessionId]: trigger,
      };
    },
    setSessionRejectedPlanFingerprint(sessionId: string, fingerprint: string | null) {
      if (!sessionId) {
        return;
      }
      if (!fingerprint) {
        if (!(sessionId in this.sessionRejectedPlanFingerprint)) {
          return;
        }
        const next = { ...this.sessionRejectedPlanFingerprint };
        delete next[sessionId];
        this.sessionRejectedPlanFingerprint = next;
        persistRejectedPlanFingerprints(next);
        return;
      }
      if (this.sessionRejectedPlanFingerprint[sessionId] === fingerprint) {
        return;
      }
      const next = {
        ...this.sessionRejectedPlanFingerprint,
        [sessionId]: fingerprint,
      };
      this.sessionRejectedPlanFingerprint = next;
      persistRejectedPlanFingerprints(next);
    },
    rejectedPlanFingerprint(sessionId: string): string | null {
      if (!sessionId) {
        return null;
      }
      return this.sessionRejectedPlanFingerprint[sessionId] ?? null;
    },
    isPlanModeActive(sessionId: string): boolean {
      return Boolean(sessionId && this.sessionPlanMode[sessionId]);
    },
    setContextUsage(sessionId: string, usage: ContextUsageSnapshot | undefined) {
      if (!sessionId) {
        return;
      }
      this.contextUsage = {
        ...this.contextUsage,
        [sessionId]: usage,
      };
    },
    setSessionCacheUsage(sessionId: string, usage: SessionCacheUsage | undefined) {
      if (!sessionId) {
        return;
      }
      this.sessionCacheUsage = {
        ...this.sessionCacheUsage,
        [sessionId]: usage,
      };
    },
    setMessageCacheUsage(sessionId: string, usages: Record<string, SessionCacheUsage>) {
      if (!sessionId) {
        return;
      }
      this.messageCacheUsage = {
        ...this.messageCacheUsage,
        [sessionId]: usages,
      };
    },
    addPromptCacheUsage(sessionId: string, usage: SessionCacheUsage, messageId?: string) {
      if (!sessionId) {
        return;
      }
      this.sessionCacheUsage = {
        ...this.sessionCacheUsage,
        [sessionId]: accumulateCacheUsage(this.sessionCacheUsage[sessionId], usage),
      };
      if (!messageId) {
        return;
      }
      const current = this.messageCacheUsage[sessionId] ?? {};
      this.messageCacheUsage = {
        ...this.messageCacheUsage,
        [sessionId]: {
          ...current,
          [messageId]: accumulateCacheUsage(current[messageId], usage),
        },
      };
    },
    setSessionMessages(sessionId: string, messages: ChatMessage[]) {
      sessionsStore().setSessionMessages(sessionId, messages);
    },
    resolveOverlaySessionId(preferred?: string) {
      return sessionsStore().resolveOverlaySessionId(preferred);
    },
    upsertMessage(message: ChatMessage) {
      const sessionId = message.sessionId;
      if (!sessionId) {
        return false;
      }

      const normalized: ChatMessage = {
        ...message,
        sessionId,
        role: normalizeRole(message.role),
      };
      const messages = sessionsStore().sessions[sessionId] ?? [];
      const index = messages.findIndex((item) => item.id === normalized.id);

      if (index === -1) {
        this.setSessionMessages(sessionId, [...messages, normalized]);
        return true;
      }

      const next = [...messages];
      next[index] = normalized;
      this.setSessionMessages(sessionId, next);
      return true;
    },
    stageAskUserAnswer(sessionId: string, items: AskUserAnswerItem[]) {
      const normalized = items
        .map((item) => ({
          header: item.header?.trim() || undefined,
          selected: item.selected.map((v) => v.trim()).filter(Boolean),
          userSupplement: Boolean(item.userSupplement),
        }))
        .filter((item) => item.userSupplement || item.selected.length > 0);
      if (normalized.length === 0) {
        return;
      }

      const resolvedSessionId = this.resolveOverlaySessionId(sessionId);
      const messages = sessionsStore().sessions[resolvedSessionId];
      if (!messages) {
        return;
      }

      // 挂在当前轮的 assistant 消息上，渲染在工具卡片之后、AI 正文之前
      let targetIndex = -1;
      for (let i = messages.length - 1; i >= 0; i -= 1) {
        const message = messages[i];
        if (normalizeRole(message.role) !== "assistant") {
          continue;
        }
        if (message.toolActivities?.some((activity) => activity.toolName === "ask_user")) {
          targetIndex = i;
          break;
        }
      }
      if (targetIndex === -1) {
        for (let i = messages.length - 1; i >= 0; i -= 1) {
          if (normalizeRole(messages[i].role) === "assistant") {
            targetIndex = i;
            break;
          }
        }
      }
      if (targetIndex === -1) {
        return;
      }

      const next = [...messages];
      next[targetIndex] = {
        ...next[targetIndex],
        askUserAnswer: normalized,
      };
      this.setSessionMessages(resolvedSessionId, next);
    },
    stageUserMessage(sessionId: string, content: string) {
      const trimmed = content.trim();
      if (!trimmed) {
        return;
      }

      this.upsertMessage({
        id: `local-user-${Date.now()}`,
        sessionId,
        role: "user",
        content: trimmed,
        status: "done",
        timestamp: Date.now(),
      });
    },
    pushStagedMessage(sessionId: string, content: string) {
      const trimmed = content.trim();
      if (!sessionId || !trimmed) {
        return;
      }
      void import("@/commands/remote").then(async ({ remotePushStaged }) => {
        try {
          const messages = await remotePushStaged(sessionId, trimmed);
          this.stagedMessages = {
            ...this.stagedMessages,
            [sessionId]: messages,
          };
        } catch {
          // Offline / command missing: keep a local fallback so typing isn't lost.
          this.stagedMessages = {
            ...this.stagedMessages,
            [sessionId]: [...(this.stagedMessages[sessionId] ?? []), trimmed],
          };
        }
      });
    },
    /** Insert a message back into the queue at (clamped) index, preserving the
     * original order when re-editing a staged message from the input box. */
    insertStagedMessage(sessionId: string, index: number, content: string) {
      const trimmed = content.trim();
      if (!sessionId || !trimmed) {
        return;
      }
      void import("@/commands/remote").then(async ({ remoteInsertStaged }) => {
        try {
          const messages = await remoteInsertStaged(sessionId, index, trimmed);
          this.stagedMessages = {
            ...this.stagedMessages,
            [sessionId]: messages,
          };
        } catch {
          const queue = [...(this.stagedMessages[sessionId] ?? [])];
          const at = Math.max(0, Math.min(index, queue.length));
          queue.splice(at, 0, trimmed);
          this.stagedMessages = {
            ...this.stagedMessages,
            [sessionId]: queue,
          };
        }
      });
    },
    removeStagedMessage(sessionId: string, index: number) {
      void import("@/commands/remote").then(async ({ remoteRemoveStaged }) => {
        try {
          const messages = await remoteRemoveStaged(sessionId, index);
          if (messages.length === 0) {
            this.clearStagedLocal(sessionId);
          } else {
            this.stagedMessages = {
              ...this.stagedMessages,
              [sessionId]: messages,
            };
          }
        } catch {
          const queue = this.stagedMessages[sessionId];
          if (!queue || index < 0 || index >= queue.length) {
            return;
          }
          const next = queue.filter((_, itemIndex) => itemIndex !== index);
          if (next.length === 0) {
            this.clearStagedLocal(sessionId);
          } else {
            this.stagedMessages = {
              ...this.stagedMessages,
              [sessionId]: next,
            };
          }
        }
      });
    },
    clearStagedLocal(sessionId: string) {
      if (!this.stagedMessages[sessionId]) {
        return;
      }
      const next = { ...this.stagedMessages };
      delete next[sessionId];
      this.stagedMessages = next;
    },
    clearStaged(sessionId: string) {
      this.clearStagedLocal(sessionId);
      void import("@/commands/remote").then(({ remoteClearStaged }) => {
        void remoteClearStaged(sessionId).catch(() => undefined);
      });
    },
    applyStagedFromRemote(sessionId: string, messages: string[]) {
      if (!sessionId) return;
      if (!messages.length) {
        this.clearStagedLocal(sessionId);
        return;
      }
      this.stagedMessages = {
        ...this.stagedMessages,
        [sessionId]: [...messages],
      };
    },
    /** Dispatch exactly one queued message. The next one is dispatched by the
     * next chat-finished event, so queued turns never merge into one another. */
    async flushStaged(sessionId: string) {
      if (this.stagedDispatching[sessionId] || this.sending[sessionId]) {
        return;
      }
      // 用户停止后 sending 可能已清，但助手行仍是 pending/streaming；先落定再发队列。
      if (this.hasActiveAssistantResponse(sessionId)) {
        this.settleInterruptedSession(sessionId);
      }
      if (this.hasActiveAssistantResponse(sessionId) || this.sending[sessionId]) {
        return;
      }
      this.stagedDispatching = {
        ...this.stagedDispatching,
        [sessionId]: true,
      };
      try {
        const { remotePopStaged } = await import("@/commands/remote");
        let content: string | null = null;
        try {
          content = await remotePopStaged(sessionId);
        } catch {
          const queue = this.stagedMessages[sessionId];
          if (!queue?.length) return;
          content = queue[0] ?? null;
          this.stagedMessages = {
            ...this.stagedMessages,
            [sessionId]: queue.slice(1),
          };
        }
        if (!content) {
          this.clearStagedLocal(sessionId);
          return;
        }
        const remaining = this.stagedMessages[sessionId] ?? [];
        // Mirror may lag; refresh from remaining after pop when event hasn't arrived yet.
        if (remaining[0] === content) {
          this.stagedMessages = {
            ...this.stagedMessages,
            [sessionId]: remaining.slice(1),
          };
        }
        const sent = await this.send(content, sessionId, { fromQueue: true });
        if (!sent) {
          const { remoteInsertStaged } = await import("@/commands/remote");
          try {
            const messages = await remoteInsertStaged(sessionId, 0, content);
            this.stagedMessages = {
              ...this.stagedMessages,
              [sessionId]: messages,
            };
          } catch {
            this.stagedMessages = {
              ...this.stagedMessages,
              [sessionId]: [content, ...(this.stagedMessages[sessionId] ?? [])],
            };
          }
        }
      } finally {
        const next = { ...this.stagedDispatching };
        delete next[sessionId];
        this.stagedDispatching = next;
      }
    },
    /** Guide a single staged message into the running turn immediately. The
     * message is removed from the queue and soft-injected; the rest of the
     * queue stays put. */
    async guideStagedMessage(sessionId: string, index: number) {
      const { remoteTakeStaged } = await import("@/commands/remote");
      let content: string | null = null;
      try {
        content = await remoteTakeStaged(sessionId, index);
      } catch {
        const queue = this.stagedMessages[sessionId];
        if (!queue || index < 0 || index >= queue.length) {
          return;
        }
        content = queue[index] ?? null;
        this.removeStagedMessage(sessionId, index);
      }
      if (!content) return;
      await this.send(content, sessionId, { fromQueue: true });
    },
    stageSoftInject(sessionId: string, content: string) {
      const trimmed = content.trim();
      if (!trimmed) return;
      const token = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      const messages = sessionsStore().sessions[sessionId] ?? [];
      // Place the inject after the active assistant so MessageList can fold it
      // into that turn (not as a new unanswered user bubble).
      this.setSessionMessages(sessionId, [
        ...messages,
        {
          id: `local-user-${token}`,
          sessionId,
          role: "user",
          content: trimmed,
          injected: true,
          status: "done",
          timestamp: Date.now(),
        },
      ]);
    },
    stageTurn(sessionId: string, content: string) {
      const trimmed = content.trim();
      if (!trimmed) return;
      const token = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      const timestamp = Date.now();
      const messages = sessionsStore().sessions[sessionId] ?? [];
      this.setSessionMessages(sessionId, [
        ...messages,
        {
          id: `local-user-${token}`,
          sessionId,
          role: "user",
          content: trimmed,
          status: "done",
          timestamp,
        },
        {
          id: `local-assistant-${token}`,
          sessionId,
          role: "assistant",
          content: "",
          status: "pending",
          timestamp: timestamp + 1,
        },
      ]);
    },
    mergeSession(fromSessionId: string, toSessionId: string) {
      sessionsStore().mergeSession(fromSessionId, toSessionId);
    },
    applyChatStarted(payload: RawChatStarted) {
      const normalized = normalizeChatStarted(payload);
      if (!normalized) {
        return;
      }

      const eventSessionId = normalized.sessionId;
      // Overlay draft remap is ONLY for the Alt+Alt draft conversation itself.
      // Do NOT treat every unknown sessionId as overlay — Companion new chats mint
      // fresh ids that are not yet in `sessions`, and must stay independent even
      // while another desktop turn is still marked sending.
      const isOverlayEvent =
        Boolean(this.overlayDraftSessionId) &&
        (!eventSessionId || eventSessionId === this.overlayDraftSessionId);
      const targetSessionId = isOverlayEvent
        ? this.resolveOverlaySessionId(eventSessionId)
        : eventSessionId || this.resolveOverlaySessionId(eventSessionId);
      if (!targetSessionId) {
        return;
      }
      const userMessage = {
        ...normalized.userMessage,
        sessionId: targetSessionId,
      };
      const assistantMessage = {
        ...normalized.assistantMessage,
        sessionId: targetSessionId,
      };

      if (eventSessionId && eventSessionId !== targetSessionId) {
        this.mergeSession(eventSessionId, targetSessionId);
      }

      const messages = [...(sessionsStore().sessions[targetSessionId] ?? [])];

      // Always surface the user turn (including plan approve) in the thread.
      const localUserIndex = findLastMessageIndex(
        messages,
        (item) => item.id.startsWith("local-user-") && item.content === userMessage.content,
      );
      if (localUserIndex !== -1) {
        messages[localUserIndex] = userMessage;
      } else {
        const existingUserIndex = messages.findIndex((item) => item.id === userMessage.id);
        if (existingUserIndex === -1) {
          messages.push(userMessage);
        } else {
          messages[existingUserIndex] = userMessage;
        }
      }

      let assistantIndex = messages.findIndex((item) => item.id === assistantMessage.id);
      if (assistantIndex === -1) {
        assistantIndex = findLastMessageIndex(
          messages,
          (item) => item.id.startsWith("local-assistant-") && item.status === "pending",
        );
      }
      if (assistantIndex === -1) {
        messages.push(assistantMessage);
      } else {
        messages[assistantIndex] = assistantMessage;
      }

      this.setSessionMessages(targetSessionId, messages);
      if (isOverlayEvent) {
        this.setOverlayDraftSession(targetSessionId);
      }
      this.sending = { ...this.sending, [targetSessionId]: true };
    },
    reconcileOptimisticIds(sessionId: string, userMessageId: string, assistantMessageId: string) {
      const messages = [...(sessionsStore().sessions[sessionId] ?? [])];
      const localUserIndex = findLastMessageIndex(messages, (item) =>
        item.id.startsWith("local-user-"),
      );
      const localAssistantIndex = findLastMessageIndex(messages, (item) =>
        item.id.startsWith("local-assistant-"),
      );
      let changed = false;
      if (localUserIndex !== -1) {
        messages[localUserIndex] = { ...messages[localUserIndex], id: userMessageId };
        changed = true;
      }
      if (localAssistantIndex !== -1) {
        messages[localAssistantIndex] = {
          ...messages[localAssistantIndex],
          id: assistantMessageId,
        };
        changed = true;
      }
      if (changed) this.setSessionMessages(sessionId, messages);
    },
    markMessageInjected(sessionId: string, messageId: string) {
      const messages = [...(sessionsStore().sessions[sessionId] ?? [])];
      const index = messages.findIndex((item) => item.id === messageId);
      if (index === -1) return;
      const current = messages[index]!;
      messages[index] = {
        ...current,
        injected: true,
        content: current.content.startsWith("<!--peek:soft-inject-->")
          ? current.content
          : `<!--peek:soft-inject-->\n${current.content}`,
      };
      this.setSessionMessages(sessionId, messages);
    },
    failOptimisticSend(sessionId: string, error: unknown, softInject = false) {
      const messages = [...(sessionsStore().sessions[sessionId] ?? [])];
      if (softInject) {
        const index = findLastMessageIndex(messages, (item) => item.id.startsWith("local-user-"));
        if (index !== -1) {
          messages.splice(index, 1);
          this.setSessionMessages(sessionId, messages);
        }
        return;
      }
      const index = findLastMessageIndex(
        messages,
        (item) => normalizeRole(item.role) === "assistant" && item.status === "pending",
      );
      if (index === -1) return;
      const settingStore = useSettingStore();
      const raw = String(error);
      const configureProvider = raw === "CONFIGURE_PROVIDER" || isConfigureProviderError(raw);
      const content = configureProvider
        ? `${CONFIGURE_PROVIDER_MARKER}\n${tr(settingStore.language, "configureProviderHint")}`
        : `发送失败：${raw}`;
      messages[index] = {
        ...messages[index],
        content,
        status: "error",
        completedAt: Date.now(),
      };
      this.setSessionMessages(sessionId, messages);
    },
    appendDelta(sessionId: string, messageId: string, delta: string, fallbackSessionId?: string) {
      const resolvedSessionId = this.resolveOverlaySessionId(
        resolveSessionId(sessionId, fallbackSessionId),
      );
      const messages = sessionsStore().sessions[resolvedSessionId];
      if (!messages) {
        return;
      }

      const index = messages.findIndex((item) => item.id === messageId);
      if (index === -1) {
        return;
      }

      const next = [...messages];
      next[index] = {
        ...next[index],
        content: next[index].content + delta,
        status: "streaming",
        // Once tokens arrive, drop ephemeral analyzing/status labels.
        activityStatus: undefined,
      };
      this.setSessionMessages(resolvedSessionId, next);
    },
    appendReasoning(
      sessionId: string,
      messageId: string,
      chunk: string,
      fallbackSessionId?: string,
    ) {
      const resolvedSessionId = this.resolveOverlaySessionId(
        resolveSessionId(sessionId, fallbackSessionId),
      );
      const messages = sessionsStore().sessions[resolvedSessionId];
      if (!messages) {
        return;
      }

      const index = messages.findIndex((item) => item.id === messageId);
      if (index === -1) {
        return;
      }

      const next = [...messages];
      const current = next[index];
      next[index] = {
        ...current,
        reasoning: (current.reasoning ?? "") + chunk,
        workTimeline: appendTimelineText(current.workTimeline, chunk, "reasoning"),
        status: current.status === "pending" ? "streaming" : current.status,
        activityStatus: undefined,
      };
      this.setSessionMessages(resolvedSessionId, next);
    },
    setActivityStatus(
      sessionId: string,
      messageId: string,
      kind: string,
      fallbackSessionId?: string,
    ) {
      const resolvedSessionId = this.resolveOverlaySessionId(
        resolveSessionId(sessionId, fallbackSessionId),
      );
      const messages = sessionsStore().sessions[resolvedSessionId];
      if (!messages) {
        return;
      }

      const index = messages.findIndex((item) => item.id === messageId);
      if (index === -1) {
        return;
      }

      const next = [...messages];
      const current = next[index];
      if (kind.startsWith("stream_retry")) {
        const keptActivities = current.toolActivities?.filter(
          (activity) => activity.status !== "running",
        );
        next[index] = {
          ...current,
          content: "",
          reasoning: undefined,
          workTimeline: undefined,
          toolActivities: keptActivities?.length ? keptActivities : undefined,
          activityStatus: kind,
          status: "streaming",
        };
        this.setSessionMessages(resolvedSessionId, next);
        return;
      }
      // Compact finished: drop the in-progress label instead of leaving a
      // leftover "compressing" status on the assistant bubble.
      const activityStatus =
        kind.startsWith("context_compacted") || !kind.trim() ? undefined : kind;
      if (current.activityStatus === activityStatus) {
        return;
      }
      next[index] = {
        ...current,
        activityStatus,
      };
      this.setSessionMessages(resolvedSessionId, next);
    },
    patchMessageContent(
      sessionId: string,
      messageId: string,
      content: string,
      fallbackSessionId?: string,
    ) {
      const resolvedSessionId = this.resolveOverlaySessionId(
        resolveSessionId(sessionId, fallbackSessionId),
      );
      const messages = sessionsStore().sessions[resolvedSessionId];
      if (!messages) {
        return;
      }

      const index = messages.findIndex((item) => item.id === messageId);
      if (index === -1) {
        return;
      }

      const next = [...messages];
      next[index] = {
        ...next[index],
        content,
        estimatedTokens: undefined,
      };
      this.setSessionMessages(resolvedSessionId, next);
    },
    applyStreamDeltas(
      updates: Array<{
        sessionId: string;
        messageId: string;
        contentDelta?: string;
        reasoningDelta?: string;
        fallbackSessionId?: string;
      }>,
    ) {
      if (updates.length === 0) {
        return;
      }

      const grouped = new Map<
        string,
        Map<string, { contentDelta: string; reasoningDelta: string; fallbackSessionId?: string }>
      >();

      for (const update of updates) {
        const resolvedSessionId = this.resolveOverlaySessionId(
          resolveSessionId(update.sessionId, update.fallbackSessionId),
        );
        const byMessage =
          grouped.get(resolvedSessionId) ??
          new Map<
            string,
            { contentDelta: string; reasoningDelta: string; fallbackSessionId?: string }
          >();

        const current = byMessage.get(update.messageId) ?? {
          contentDelta: "",
          reasoningDelta: "",
          fallbackSessionId: update.fallbackSessionId,
        };

        if (update.contentDelta) {
          current.contentDelta += update.contentDelta;
        }
        if (update.reasoningDelta) {
          current.reasoningDelta += update.reasoningDelta;
        }

        byMessage.set(update.messageId, current);
        grouped.set(resolvedSessionId, byMessage);
      }

      for (const [resolvedSessionId, byMessage] of grouped) {
        const messages = sessionsStore().sessions[resolvedSessionId];
        if (!messages) {
          continue;
        }

        const next = [...messages];
        let changed = false;

        for (const [messageId, delta] of byMessage) {
          const index = next.findIndex((item) => item.id === messageId);
          if (index === -1) {
            continue;
          }

          const current = next[index];
          // Reasoning normally precedes the content it informs within one
          // batched frame, so fold it into the timeline first.
          let workTimeline = current.workTimeline;
          if (delta.reasoningDelta.length > 0) {
            workTimeline = appendTimelineText(workTimeline, delta.reasoningDelta, "reasoning");
          }
          if (delta.contentDelta.length > 0) {
            workTimeline = appendTimelineText(workTimeline, delta.contentDelta, "content");
          }
          next[index] = {
            ...current,
            content: current.content + delta.contentDelta,
            reasoning:
              delta.reasoningDelta.length > 0
                ? (current.reasoning ?? "") + delta.reasoningDelta
                : current.reasoning,
            workTimeline,
            status:
              current.status === "pending" || delta.contentDelta || delta.reasoningDelta
                ? "streaming"
                : current.status,
          };
          changed = true;
        }

        if (changed) {
          this.setSessionMessages(resolvedSessionId, next);
        }
      }
    },
    finishMessage(
      sessionId: string,
      messageId: string,
      content: string,
      fallbackSessionId?: string,
      reasoning?: string,
    ) {
      const resolvedSessionId = this.resolveOverlaySessionId(
        resolveSessionId(sessionId, fallbackSessionId),
      );
      const messages = sessionsStore().sessions[resolvedSessionId];
      if (!messages) {
        this.clearSendingMany([
          sessionId,
          resolvedSessionId,
          fallbackSessionId,
          this.overlayDraftSessionId,
        ]);
        return;
      }

      const index = messages.findIndex((item) => item.id === messageId);
      if (index === -1) {
        this.clearSendingMany([
          sessionId,
          resolvedSessionId,
          fallbackSessionId,
          this.overlayDraftSessionId,
        ]);
        return;
      }

      const previous = messages[index];
      let workTimeline = previous.workTimeline ? [...previous.workTimeline] : undefined;
      // Breaker/max-steps notices replace or append onto content; ensure they
      // appear in the work timeline so AgentWorkDetails can render them.
      const stopNotice = (content ?? "")
        .split(/\n+/)
        .map((line) => line.trim())
        .find((line) => line.startsWith("已停止：") || line.startsWith("Stopped:"));
      if (stopNotice) {
        const timelineText = (workTimeline ?? [])
          .filter((item) => item.type === "content")
          .map((item) => item.content)
          .join("");
        if (!timelineText.includes(stopNotice)) {
          workTimeline = [
            ...(workTimeline ?? []),
            {
              type: "content" as const,
              id: `${messageId}-stop-notice`,
              content: `\n\n${stopNotice}`,
            },
          ];
        }
      }

      // Prefer live streamed text when ChatFinished carries a shorter last-round
      // snapshot (multi-turn agent loops). Never shrink content/reasoning.
      const nextContent = content.length >= previous.content.length ? content : previous.content;
      const nextReasoning = (() => {
        if (reasoning === undefined) return previous.reasoning;
        const prev = previous.reasoning ?? "";
        if (!prev) return reasoning;
        if (!reasoning) return previous.reasoning;
        return reasoning.length >= prev.length ? reasoning : previous.reasoning;
      })();

      const next = [...messages];
      const completed: ChatMessage = {
        ...previous,
        content: nextContent,
        status: "done",
        completedAt: Date.now(),
        activityStatus: undefined,
        ...(workTimeline ? { workTimeline } : {}),
        ...(nextReasoning !== undefined ? { reasoning: nextReasoning } : {}),
      };
      completed.estimatedTokens = estimateMessageTokens({
        ...completed,
        estimatedTokens: undefined,
      });
      next[index] = completed;
      this.setSessionMessages(resolvedSessionId, next);
      this.clearSendingMany([
        sessionId,
        resolvedSessionId,
        fallbackSessionId,
        this.overlayDraftSessionId,
      ]);
    },
    upsertToolActivity(
      sessionId: string,
      messageId: string,
      activity: ToolActivity,
      fallbackSessionId?: string,
    ) {
      const resolvedSessionId = this.resolveOverlaySessionId(
        resolveSessionId(sessionId, fallbackSessionId),
      );
      const messages = sessionsStore().sessions[resolvedSessionId];
      if (!messages) {
        return;
      }

      const index = messages.findIndex((item) => item.id === messageId);
      if (index === -1) {
        return;
      }

      const current = messages[index];
      const activities = [...(current.toolActivities ?? [])];
      const existingIndex = activities.findIndex((item) => item.id === activity.id);
      const isNewActivity = existingIndex === -1;
      if (existingIndex === -1) {
        activities.push(activity);
      } else {
        activities[existingIndex] = { ...activities[existingIndex], ...activity };
      }

      const alreadyOnTimeline = current.workTimeline?.some(
        (item) => item.type === "tool" && item.toolActivityId === activity.id,
      );
      const next = [...messages];
      next[index] = {
        ...current,
        toolActivities: activities,
        estimatedTokens: undefined,
        workTimeline:
          isNewActivity && !alreadyOnTimeline
            ? [
                ...(current.workTimeline ?? []),
                {
                  type: "tool" as const,
                  id: `tool-${activity.id}`,
                  toolActivityId: activity.id,
                },
              ]
            : current.workTimeline,
        status:
          current.status === "pending" || current.status === "streaming"
            ? "streaming"
            : current.status,
      };
      this.setSessionMessages(resolvedSessionId, next);
    },
    applyFileOffer(payload: FileOfferEvent, fallbackSessionId?: string) {
      const offer: SharedFileOffer = {
        offerId: payload.offerId || payload.offer_id || "",
        path: payload.path,
        absolutePath: payload.absolutePath || payload.absolute_path,
        name: payload.name,
        mime: payload.mime || "",
        size: payload.size ?? 0,
        workspaceId: payload.workspaceId || payload.workspace_id,
      };
      this.attachOfferToAssistant(
        payload.sessionId || payload.session_id,
        fallbackSessionId,
        (message) => {
          const files = message.sharedFiles ?? [];
          if (offer.offerId && files.some((item) => item.offerId === offer.offerId)) {
            return message;
          }
          return { ...message, sharedFiles: [...files, offer] };
        },
      );
    },
    applyUrlOffer(payload: UrlOfferEvent, fallbackSessionId?: string) {
      const offer: SharedUrlOffer = {
        offerId: payload.offerId || payload.offer_id || "",
        label: payload.label || "Preview",
        originUrl: payload.originUrl || payload.origin_url || "",
        publicUrl: payload.publicUrl || payload.public_url || "",
      };
      if (!offer.publicUrl) {
        return;
      }
      this.attachOfferToAssistant(
        payload.sessionId || payload.session_id,
        fallbackSessionId,
        (message) => {
          const urls = message.sharedUrls ?? [];
          if (offer.offerId && urls.some((item) => item.offerId === offer.offerId)) {
            return message;
          }
          return { ...message, sharedUrls: [...urls, offer] };
        },
      );
    },
    attachOfferToAssistant(
      sessionId: string | undefined,
      fallbackSessionId: string | undefined,
      update: (message: ChatMessage) => ChatMessage,
    ) {
      const resolvedSessionId = this.resolveOverlaySessionId(
        resolveSessionId(sessionId, fallbackSessionId),
      );
      const messages = sessionsStore().sessions[resolvedSessionId];
      if (!messages?.length) {
        return;
      }
      let index = -1;
      for (let i = messages.length - 1; i >= 0; i -= 1) {
        if (messages[i].role === "assistant") {
          index = i;
          break;
        }
      }
      if (index < 0) {
        index = messages.length - 1;
      }
      const next = [...messages];
      next[index] = update(messages[index]);
      this.setSessionMessages(resolvedSessionId, next);
    },
    attachToolApprovalPreview(
      sessionId: string,
      toolName: string,
      preview: ToolPreviewPayload | null,
      fallbackSessionId?: string,
    ) {
      const resolvedSessionId = this.resolveOverlaySessionId(
        resolveSessionId(sessionId, fallbackSessionId),
      );
      const messages = sessionsStore().sessions[resolvedSessionId];
      if (!messages?.length) {
        return;
      }

      for (let messageIndex = messages.length - 1; messageIndex >= 0; messageIndex -= 1) {
        const message = messages[messageIndex];
        const activities = message.toolActivities;
        if (!activities?.length) {
          continue;
        }
        for (let activityIndex = activities.length - 1; activityIndex >= 0; activityIndex -= 1) {
          const activity = activities[activityIndex];
          if (activity.status !== "running" || activity.toolName !== toolName) {
            continue;
          }
          const nextActivities = [...activities];
          nextActivities[activityIndex] = { ...activity, preview };
          const next = [...messages];
          next[messageIndex] = { ...message, toolActivities: nextActivities };
          this.setSessionMessages(resolvedSessionId, next);
          return;
        }
      }
    },
    failMessage(sessionId: string, messageId: string, error: string, fallbackSessionId?: string) {
      const resolvedSessionId = this.resolveOverlaySessionId(
        resolveSessionId(sessionId, fallbackSessionId),
      );
      const messages = sessionsStore().sessions[resolvedSessionId];
      if (!messages) {
        this.clearSendingMany([
          sessionId,
          resolvedSessionId,
          fallbackSessionId,
          this.overlayDraftSessionId,
        ]);
        return;
      }

      const index = messages.findIndex((item) => item.id === messageId);
      if (index === -1) {
        this.clearSendingMany([
          sessionId,
          resolvedSessionId,
          fallbackSessionId,
          this.overlayDraftSessionId,
        ]);
        return;
      }

      const next = [...messages];
      next[index] = {
        ...next[index],
        content: error,
        status: "error",
        completedAt: Date.now(),
        activityStatus: undefined,
      };
      this.setSessionMessages(resolvedSessionId, next);
      this.clearSendingMany([
        sessionId,
        resolvedSessionId,
        fallbackSessionId,
        this.overlayDraftSessionId,
      ]);
    },
    clearSending(sessionId: string) {
      if (!sessionId || !this.sending[sessionId]) {
        return;
      }
      const next = { ...this.sending };
      delete next[sessionId];
      this.sending = next;
    },
    clearSendingMany(sessionIds: Array<string | undefined | null>) {
      const ids = sessionIds.filter((id): id is string => Boolean(id));
      if (ids.length === 0) {
        return;
      }
      const next = { ...this.sending };
      let changed = false;
      for (const id of ids) {
        if (next[id]) {
          delete next[id];
          changed = true;
        }
      }
      if (changed) {
        this.sending = next;
      }
    },
    hasActiveAssistantResponse(sessionId: string) {
      return (sessionsStore().sessions[sessionId] ?? []).some(
        (message) =>
          normalizeRole(message.role) === "assistant" &&
          (message.status === "pending" || message.status === "streaming"),
      );
    },
    completeAskUserToolActivities(sessionId: string, answer?: string) {
      const resolvedSessionId = this.resolveOverlaySessionId(sessionId);
      const messages = sessionsStore().sessions[resolvedSessionId];
      if (!messages) {
        return;
      }

      let changed = false;
      const next = messages.map((message) => {
        const activities = message.toolActivities;
        if (
          !activities?.some(
            (activity) => activity.toolName === "ask_user" && activity.status === "running",
          )
        ) {
          return message;
        }

        changed = true;
        return {
          ...message,
          toolActivities: activities.map((activity) =>
            activity.toolName === "ask_user" && activity.status === "running"
              ? {
                  ...activity,
                  status: "done" as const,
                  success: true,
                  ...(answer ? { result: answer } : {}),
                }
              : activity,
          ),
        };
      });

      if (changed) {
        this.setSessionMessages(resolvedSessionId, next);
      }
    },
    async loadHistory(sessionId: string) {
      try {
        const response = await chatHistory({ sessionId });
        const messages = response.messages
          .map((message) => normalizeMessage(message, sessionId))
          .filter((message): message is ChatMessage => message !== null);
        if (messages.length > 0) {
          this.markSessionStarted(sessionId);
        }
        // If this session is not actively streaming in the current process,
        // treat leftover pending/running rows from a crash as interrupted.
        const completedAt = response.messageCompletedAt ?? {};
        const loaded = messages.map((message) =>
          completedAt[message.id] && !message.completedAt
            ? { ...message, completedAt: completedAt[message.id] }
            : message,
        );
        const nextMessages = this.sending[sessionId]
          ? mergeActiveHistory(loaded, sessionsStore().sessions[sessionId] ?? [])
          : settleInterruptedMessages(loaded);
        const existing = sessionsStore().sessions[sessionId];
        if (
          !existing ||
          messagesHistoryFingerprint(existing) !== messagesHistoryFingerprint(nextMessages)
        ) {
          this.setSessionMessages(sessionId, nextMessages);
        }
        const historyCache = cacheUsagesFromHistory(response.messageCacheUsages);
        if (!this.sending[sessionId] || !this.sessionCacheUsage[sessionId]) {
          this.setSessionCacheUsage(sessionId, response.lastCacheUsage);
          this.setMessageCacheUsage(sessionId, historyCache);
        } else {
          this.setMessageCacheUsage(sessionId, {
            ...historyCache,
            ...(this.messageCacheUsage[sessionId] ?? {}),
          });
        }
        if (!this.sending[sessionId]) {
          this.clearSending(sessionId);
        }
      } catch (error) {
        log.error("chat_history failed", error);
        // A transient history failure must not blank an already visible chat.
        if (!sessionsStore().sessions[sessionId]) {
          this.setSessionMessages(sessionId, []);
        }
      }
    },
    settleInterruptedSession(sessionId: string) {
      const messages = sessionsStore().sessions[sessionId];
      if (!messages) {
        this.clearSending(sessionId);
        return;
      }
      this.setSessionMessages(sessionId, settleInterruptedMessages(messages));
      this.clearSending(sessionId);
    },
    async send(
      message: string,
      sessionId: string,
      options?: {
        staged?: boolean;
        workspaceId?: string;
        quickAsk?: boolean;
        /** Internal: this send comes from the staged queue (guide / auto-send
         * after the turn finishes) and must never be re-staged. */
        fromQueue?: boolean;
        /** Skip complexity auto-plan (approve & execute follow-up). */
        skipAutoPlan?: boolean;
        /** Approve & execute continuation: unlocks writers; message is persisted. */
        resumePlan?: boolean;
      },
    ) {
      const trimmed = message.trim();
      if (!trimmed) {
        return false;
      }

      const busy = Boolean(this.sending[sessionId] || this.hasActiveAssistantResponse(sessionId));

      if (options?.resumePlan && busy) {
        // Never stage an approval continuation for later; the plan card only
        // shows after the turn finished, so this is a race guard only.
        return false;
      }

      // While a turn is executing, new user messages are staged instead of
      // being injected immediately. They reach the AI either via the guide
      // button (flushStaged → soft-inject) or automatically when the turn
      // finishes (flushStaged → next turn).
      if (!options?.staged && !options?.fromQueue && busy) {
        this.pushStagedMessage(sessionId, trimmed);
        return true;
      }

      // Queue flushes may inject into the running turn; regular sends only
      // happen when no turn is in flight.
      const softInject = !options?.staged && busy;

      this.setOverlayDraftSession(sessionId);

      if (!options?.staged) {
        if (softInject) {
          this.stageSoftInject(sessionId, trimmed);
        } else {
          this.stageTurn(sessionId, trimmed);
        }
      }

      this.sending[sessionId] = true;
      try {
        const chatModelStore = useChatModelStore();
        if (chatModelStore.models.length === 0 && !chatModelStore.loading) {
          await chatModelStore.fetch();
        }

        let compose = this.ensureCompose(sessionId);
        if (
          chatModelStore.models.length > 0 &&
          (!compose.chatModel.trim() ||
            !isKnownModelSelection(
              chatModelStore.models,
              compose.chatModel,
              compose.chatModelProvider,
            ))
        ) {
          const first = chatModelStore.models[0]!;
          this.setCompose(sessionId, {
            chatModel: first.id,
            chatModelProvider: first.provider,
          });
          const settingStore = useSettingStore();
          if (
            !settingStore.chatModel.trim() ||
            !isKnownModelSelection(
              chatModelStore.models,
              settingStore.chatModel,
              settingStore.chatModelProvider,
            )
          ) {
            void settingStore.update({
              chatModel: first.id,
              chatModelProvider: first.provider,
            });
          }
          compose = this.ensureCompose(sessionId);
        }

        if (chatModelStore.models.length === 0 || !compose.chatModel.trim()) {
          this.failOptimisticSend(sessionId, "CONFIGURE_PROVIDER", softInject);
          if (!softInject) {
            this.clearSending(sessionId);
          }
          return false;
        }

        this.markSessionStarted(sessionId);
        const composeForSend = this.ensureCompose(sessionId);
        const workspaceId = options?.workspaceId ?? composeForSend.draftWorkspaceId ?? undefined;
        const quickAsk = options?.quickAsk ?? !workspaceId;
        this.setComposeDraft(sessionId, composeForSend.draft ?? "", {
          workspaceId: quickAsk ? null : (workspaceId ?? null),
        });
        const response = await chat({
          message: trimmed,
          sessionId,
          workspaceId,
          quickAsk,
          modelId: compose.chatModel.trim() || undefined,
          modelProvider: compose.chatModelProvider.trim() || undefined,
          chatMode: compose.chatMode,
          toolApprovalMode: compose.toolApprovalMode,
          imageGen:
            compose.chatMode === "image"
              ? imageGenPayload(compose.imageGen, useSettingStore().imageStyleTemplates)
              : undefined,
          skipAutoPlan: options?.skipAutoPlan,
          resumePlan: options?.resumePlan,
        });
        this.reconcileOptimisticIds(sessionId, response.userMessageId, response.assistantMessageId);
        if (softInject) {
          this.markMessageInjected(sessionId, response.userMessageId);
        }
        if (response.sessionId && response.sessionId !== sessionId) {
          this.mergeSession(response.sessionId, sessionId);
          this.sending[sessionId] = true;
        }
        return true;
      } catch (error) {
        log.error("chat failed", error);
        this.failOptimisticSend(sessionId, error, softInject);
        if (!softInject) {
          this.clearSending(sessionId);
        }
        return false;
      }
    },
  },
});
