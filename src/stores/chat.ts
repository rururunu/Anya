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
import { estimateMessageTokens } from "@/services/chat/tokenEstimate";
import {
  CONFIGURE_PROVIDER_MARKER,
  isConfigureProviderError,
} from "@/services/chat/ensureDefaultModel";
import { isKnownModelSelection } from "@/lib/modelThinking";
import { tr } from "@/services/i18n";
import { normalizeChatMode } from "@/types/setting";
import type {
  AskUserAnswerItem,
  ChatMessage,
  ContextUsageSnapshot,
  FileOfferEvent,
  SharedFileOffer,
  SharedUrlOffer,
  TaskItem,
  ToolActivity,
  ToolPreviewPayload,
  UrlOfferEvent,
  WorkTimelineItem,
} from "@/types/chat";

const log = createLogger("chat-store");

/** Per-conversation compose settings. Each conversation remembers its own
 * model / mode / approval choice and input draft; unopened sessions inherit
 * the previous session's values on first open, then stay independent. */
export interface SessionCompose {
  chatModel: string;
  chatModelProvider: string;
  chatMode: "ask" | "agent" | "plan";
  toolApprovalMode: "ask" | "auto" | "alwaysAllow";
  draft: string;
  /** Workspace binding for draft-only (not-yet-sent) sessions shown in the sidebar. */
  draftWorkspaceId?: string | null;
  draftUpdatedAt?: number;
}

function sanitizeCompose(raw: Partial<SessionCompose> | null | undefined): SessionCompose {
  const base = defaultCompose();
  if (!raw || typeof raw !== "object") {
    return base;
  }
  const approval = raw.toolApprovalMode;
  return {
    chatModel: typeof raw.chatModel === "string" ? raw.chatModel : base.chatModel,
    chatModelProvider:
      typeof raw.chatModelProvider === "string" ? raw.chatModelProvider : base.chatModelProvider,
    chatMode: normalizeChatMode(raw.chatMode),
    toolApprovalMode:
      approval === "ask" || approval === "auto" || approval === "alwaysAllow"
        ? approval
        : base.toolApprovalMode,
    draft: typeof raw.draft === "string" ? raw.draft : "",
    draftWorkspaceId: raw.draftWorkspaceId ?? null,
    draftUpdatedAt: typeof raw.draftUpdatedAt === "number" ? raw.draftUpdatedAt : undefined,
  };
}

export function defaultCompose(): SessionCompose {
  return {
    chatModel: "",
    chatModelProvider: "",
    chatMode: "agent",
    toolApprovalMode: "ask",
    draft: "",
  };
}

const COMPOSE_STORAGE_KEY = "aaa.sessionCompose.v1";
const REJECTED_PLAN_STORAGE_KEY = "aaa.rejectedPlanFingerprint.v1";

interface ComposeCache {
  entries: Record<string, SessionCompose>;
  last: string;
}

let composeCacheLoaded = false;
const composeCache: ComposeCache = { entries: {}, last: "" };

function loadComposeCache(): void {
  if (composeCacheLoaded) {
    return;
  }
  composeCacheLoaded = true;
  try {
    const raw = localStorage.getItem(COMPOSE_STORAGE_KEY);
    if (!raw) {
      return;
    }
    const parsed = JSON.parse(raw) as Partial<ComposeCache>;
    if (parsed && typeof parsed === "object") {
      const entries: Record<string, SessionCompose> = {};
      for (const [id, value] of Object.entries(parsed.entries ?? {})) {
        entries[id] = sanitizeCompose(value as Partial<SessionCompose>);
      }
      composeCache.entries = entries;
      composeCache.last = typeof parsed.last === "string" ? parsed.last : "";
    }
  } catch {
    // Corrupted cache — start fresh.
  }
}

function persistComposeCache(): void {
  try {
    localStorage.setItem(COMPOSE_STORAGE_KEY, JSON.stringify(composeCache));
  } catch {
    // Storage unavailable — keep in-memory state only.
  }
}

async function syncComposeToRemote(sessionId: string, compose: SessionCompose): Promise<void> {
  try {
    const { remoteSyncSessionCompose } = await import("@/commands/remote");
    const { useChatModelStore } = await import("@/stores/chatModel");
    const chatModelStore = useChatModelStore();
    const match = chatModelStore.models.find(
      (model) =>
        model.id === compose.chatModel &&
        (!compose.chatModelProvider || model.provider === compose.chatModelProvider),
    );
    await remoteSyncSessionCompose(sessionId, {
      chatMode: compose.chatMode,
      toolApprovalMode: compose.toolApprovalMode,
      chatModel: compose.chatModel,
      chatModelProvider: compose.chatModelProvider,
      chatModelLabel: match?.displayName ?? match?.id ?? null,
    });
  } catch {
    // Gateway may be stopped — compose still lives in Pinia.
  }
}

function loadRejectedPlanFingerprints(): Record<string, string> {
  try {
    const raw = localStorage.getItem(REJECTED_PLAN_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    if (!parsed || typeof parsed !== "object") return {};
    const entries: Record<string, string> = {};
    for (const [sessionId, value] of Object.entries(parsed)) {
      if (typeof value === "string" && value.trim()) {
        entries[sessionId] = value;
      }
    }
    return entries;
  } catch {
    return {};
  }
}

function persistRejectedPlanFingerprints(entries: Record<string, string>): void {
  try {
    localStorage.setItem(REJECTED_PLAN_STORAGE_KEY, JSON.stringify(entries));
  } catch {
    // Storage unavailable — keep in-memory state only.
  }
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

/**
 * Append a text chunk (reasoning or regular content) to the work timeline,
 * merging into the trailing segment when it's the same kind so consecutive
 * deltas don't fragment into one segment per network chunk. Tool activities
 * are inserted separately (see `upsertToolActivity`), so a new segment only
 * starts here once a tool call has broken the run of same-kind text.
 */
function appendTimelineText(
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

function findLastMessageIndex(
  messages: ChatMessage[],
  predicate: (message: ChatMessage) => boolean,
) {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    if (predicate(messages[index])) return index;
  }
  return -1;
}

/**
 * Thin UI store — 按 session 镜像 AI Runtime 状态。
 */
export const useChatStore = defineStore("chat", {
  state: () => ({
    sessions: {} as Record<string, ChatMessage[]>,
    sending: {} as Record<string, boolean>,
    startedSessionIds: {} as Record<string, boolean>,
    /** Per-conversation compose settings (model / mode / approval / draft). */
    sessionCompose: {} as Record<string, SessionCompose>,
    /** User messages typed while a turn is executing — held until the guide
     * button is clicked (inject into the running turn) or the turn finishes
     * (auto-send as the next turn). */
    stagedMessages: {} as Record<string, string[]>,
    /** Prevent duplicate finish events from dispatching multiple queued turns. */
    stagedDispatching: {} as Record<string, boolean>,
    overlayDraftSessionId: "" as string,
    contextNotices: {} as Record<string, string | undefined>,
    contextUsage: {} as Record<string, ContextUsageSnapshot | undefined>,
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
    overlayMessages(state): ChatMessage[] {
      const sessionId = state.overlayDraftSessionId;
      if (!sessionId) {
        return [];
      }
      return state.sessions[sessionId] ?? [];
    },
    overlayContextNotice(state): string | undefined {
      const sessionId = state.overlayDraftSessionId;
      if (!sessionId) {
        return undefined;
      }
      return state.contextNotices[sessionId];
    },
    overlayContextUsage(state): ContextUsageSnapshot | undefined {
      const sessionId = state.overlayDraftSessionId;
      if (!sessionId) {
        return undefined;
      }
      return state.contextUsage[sessionId];
    },
  },
  actions: {
    setOverlayDraftSession(sessionId: string) {
      this.overlayDraftSessionId = sessionId;
    },
    setStartedSessionIds(ids: string[]) {
      const record: Record<string, boolean> = {};
      for (const id of ids) {
        record[id] = true;
      }
      this.startedSessionIds = record;
    },
    markSessionStarted(sessionId: string) {
      if (sessionId) {
        this.startedSessionIds[sessionId] = true;
      }
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
      } else if (this.startedSessionIds[sessionId]) {
        resolved = sanitizeCompose({
          chatModel: settingStore.chatModel ?? "",
          chatModelProvider: settingStore.chatModelProvider ?? "",
          chatMode: settingStore.chatMode ?? "agent",
          toolApprovalMode: settingStore.toolApprovalMode ?? "ask",
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
          chatMode: source?.chatMode === "ask" ? "ask" : "agent",
          toolApprovalMode: source?.toolApprovalMode ?? settingStore.toolApprovalMode ?? "ask",
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
        Pick<SessionCompose, "chatModel" | "chatModelProvider" | "chatMode" | "toolApprovalMode">
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
        current.toolApprovalMode === next.toolApprovalMode
      ) {
        return;
      }
      this.sessionCompose = { ...this.sessionCompose, [sessionId]: next };
      composeCache.entries[sessionId] = next;
      composeCache.last = sessionId;
      persistComposeCache();
      void syncComposeToRemote(sessionId, next);
    },
    /** Apply compose patch originating from Companion (skip remote echo). */
    applyComposeFromRemote(
      sessionId: string,
      patch: Partial<
        Pick<SessionCompose, "chatModel" | "chatModelProvider" | "chatMode" | "toolApprovalMode">
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
        current.toolApprovalMode === next.toolApprovalMode
      ) {
        return;
      }
      this.sessionCompose = { ...this.sessionCompose, [sessionId]: next };
      composeCache.entries[sessionId] = next;
      composeCache.last = sessionId;
      persistComposeCache();
    },
    /** Persist the input draft for one conversation (debounced by callers). */
    setComposeDraft(sessionId: string, draft: string, options?: { workspaceId?: string | null }) {
      if (!sessionId) {
        return;
      }
      const current = this.ensureCompose(sessionId);
      const trimmed = draft.trim();
      const next: SessionCompose = {
        ...current,
        draft,
        draftUpdatedAt: trimmed ? Date.now() : undefined,
      };
      if (options && "workspaceId" in options) {
        next.draftWorkspaceId = options.workspaceId ?? null;
      }
      if (current.draft === next.draft && current.draftWorkspaceId === next.draftWorkspaceId) {
        return;
      }
      this.sessionCompose = { ...this.sessionCompose, [sessionId]: next };
      composeCache.entries[sessionId] = next;
      persistComposeCache();
    },
    /** True when the conversation has unsent composer text cached. */
    sessionHasDraft(sessionId: string): boolean {
      if (!sessionId) return false;
      loadComposeCache();
      const compose = this.sessionCompose[sessionId] ?? composeCache.entries[sessionId];
      return Boolean(compose?.draft?.trim());
    },
    /** Local draft-only sessions that are not yet in the backend session list. */
    listDraftOnlySessions(knownSessionIds: Iterable<string>): Array<{
      sessionId: string;
      workspaceId?: string;
      preview: string;
      updatedAt: number;
    }> {
      loadComposeCache();
      const known = new Set(knownSessionIds);
      // Prefer live store entries; fall back to disk cache for sessions not yet hydrated.
      const entries = { ...composeCache.entries, ...this.sessionCompose };
      const out: Array<{
        sessionId: string;
        workspaceId?: string;
        preview: string;
        updatedAt: number;
      }> = [];
      for (const [sessionId, compose] of Object.entries(entries)) {
        const draft = compose.draft?.trim();
        if (!draft || known.has(sessionId)) continue;
        const messages = this.sessions[sessionId] ?? [];
        if (messages.some((item) => item.role === "user" || item.role === "assistant")) {
          continue;
        }
        out.push({
          sessionId,
          workspaceId: compose.draftWorkspaceId ?? undefined,
          preview: draft,
          updatedAt: compose.draftUpdatedAt ?? Date.now(),
        });
      }
      out.sort((left, right) => right.updatedAt - left.updatedAt);
      return out;
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
      persistComposeCache();
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
    setSessionMessages(sessionId: string, messages: ChatMessage[]) {
      if (!sessionId) {
        return;
      }
      this.sessions = {
        ...this.sessions,
        [sessionId]: messages,
      };
    },
    resolveOverlaySessionId(preferred?: string) {
      // Prefer the event/request session id. Only fall back to the overlay draft
      // when the payload omitted a session (legacy / incomplete IPC).
      // Preferring overlayDraft first remapped Companion "new chat" turns into
      // whatever conversation was last sent from the desktop.
      return resolveSessionId(preferred, this.overlayDraftSessionId);
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
      const messages = this.sessions[sessionId] ?? [];
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
      const messages = this.sessions[resolvedSessionId];
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
      this.stagedMessages = {
        ...this.stagedMessages,
        [sessionId]: [...(this.stagedMessages[sessionId] ?? []), trimmed],
      };
    },
    /** Insert a message back into the queue at (clamped) index, preserving the
     * original order when re-editing a staged message from the input box. */
    insertStagedMessage(sessionId: string, index: number, content: string) {
      const trimmed = content.trim();
      if (!sessionId || !trimmed) {
        return;
      }
      const queue = [...(this.stagedMessages[sessionId] ?? [])];
      const at = Math.max(0, Math.min(index, queue.length));
      queue.splice(at, 0, trimmed);
      this.stagedMessages = {
        ...this.stagedMessages,
        [sessionId]: queue,
      };
    },
    removeStagedMessage(sessionId: string, index: number) {
      const queue = this.stagedMessages[sessionId];
      if (!queue || index < 0 || index >= queue.length) {
        return;
      }
      const next = queue.filter((_, itemIndex) => itemIndex !== index);
      if (next.length === 0) {
        this.clearStaged(sessionId);
      } else {
        this.stagedMessages = {
          ...this.stagedMessages,
          [sessionId]: next,
        };
      }
    },
    clearStaged(sessionId: string) {
      if (!this.stagedMessages[sessionId]) {
        return;
      }
      const next = { ...this.stagedMessages };
      delete next[sessionId];
      this.stagedMessages = next;
    },
    /** Dispatch exactly one queued message. The next one is dispatched by the
     * next chat-finished event, so queued turns never merge into one another. */
    async flushStaged(sessionId: string) {
      const queue = this.stagedMessages[sessionId];
      if (!queue?.length || this.stagedDispatching[sessionId] || this.sending[sessionId]) {
        return;
      }
      // 用户停止后 sending 可能已清，但助手行仍是 pending/streaming；先落定再发队列。
      if (this.hasActiveAssistantResponse(sessionId)) {
        this.settleInterruptedSession(sessionId);
      }
      if (this.hasActiveAssistantResponse(sessionId) || this.sending[sessionId]) {
        return;
      }
      const content = queue[0];
      this.stagedMessages = {
        ...this.stagedMessages,
        [sessionId]: queue.slice(1),
      };
      this.stagedDispatching = {
        ...this.stagedDispatching,
        [sessionId]: true,
      };
      try {
        const sent = await this.send(content, sessionId, { fromQueue: true });
        if (!sent) {
          this.stagedMessages = {
            ...this.stagedMessages,
            [sessionId]: [content, ...(this.stagedMessages[sessionId] ?? [])],
          };
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
      const queue = this.stagedMessages[sessionId];
      if (!queue || index < 0 || index >= queue.length) {
        return;
      }
      const content = queue[index];
      this.removeStagedMessage(sessionId, index);
      await this.send(content, sessionId, { fromQueue: true });
    },
    stageSoftInject(sessionId: string, content: string) {
      const trimmed = content.trim();
      if (!trimmed) return;
      const token = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      const messages = this.sessions[sessionId] ?? [];
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
      const messages = this.sessions[sessionId] ?? [];
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
      if (!fromSessionId || !toSessionId || fromSessionId === toSessionId) {
        return;
      }

      const source = this.sessions[fromSessionId] ?? [];
      const target = this.sessions[toSessionId] ?? [];
      if (source.length === 0) {
        return;
      }

      const merged = [...target];
      for (const message of source) {
        const index = merged.findIndex((item) => item.id === message.id);
        if (index === -1) {
          merged.push({ ...message, sessionId: toSessionId });
        } else {
          merged[index] = { ...message, sessionId: toSessionId };
        }
      }

      const nextSessions = { ...this.sessions, [toSessionId]: merged };
      delete nextSessions[fromSessionId];
      this.sessions = nextSessions;

      if (this.overlayDraftSessionId === fromSessionId) {
        this.overlayDraftSessionId = toSessionId;
      }
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

      const messages = [...(this.sessions[targetSessionId] ?? [])];

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
        this.overlayDraftSessionId = targetSessionId;
      }
      this.sending = { ...this.sending, [targetSessionId]: true };
    },
    reconcileOptimisticIds(sessionId: string, userMessageId: string, assistantMessageId: string) {
      const messages = [...(this.sessions[sessionId] ?? [])];
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
      const messages = [...(this.sessions[sessionId] ?? [])];
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
      const messages = [...(this.sessions[sessionId] ?? [])];
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
      const messages = this.sessions[resolvedSessionId];
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
      const messages = this.sessions[resolvedSessionId];
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
      const messages = this.sessions[resolvedSessionId];
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
        next[index] = {
          ...current,
          content: "",
          reasoning: undefined,
          workTimeline: undefined,
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
      const messages = this.sessions[resolvedSessionId];
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
        const messages = this.sessions[resolvedSessionId];
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
      const messages = this.sessions[resolvedSessionId];
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
      const messages = this.sessions[resolvedSessionId];
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

      const next = [...messages];
      next[index] = {
        ...current,
        toolActivities: activities,
        estimatedTokens: undefined,
        workTimeline: isNewActivity
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
      const messages = this.sessions[resolvedSessionId];
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
      const messages = this.sessions[resolvedSessionId];
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
      const messages = this.sessions[resolvedSessionId];
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
      return (this.sessions[sessionId] ?? []).some(
        (message) =>
          normalizeRole(message.role) === "assistant" &&
          (message.status === "pending" || message.status === "streaming"),
      );
    },
    completeAskUserToolActivities(sessionId: string, answer?: string) {
      const resolvedSessionId = this.resolveOverlaySessionId(sessionId);
      const messages = this.sessions[resolvedSessionId];
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
        this.setSessionMessages(
          sessionId,
          this.sending[sessionId]
            ? mergeActiveHistory(messages, this.sessions[sessionId] ?? [])
            : settleInterruptedMessages(messages),
        );
        if (!this.sending[sessionId]) {
          this.clearSending(sessionId);
        }
      } catch (error) {
        log.error("chat_history failed", error);
        // A transient history failure must not blank an already visible chat.
        if (!this.sessions[sessionId]) {
          this.setSessionMessages(sessionId, []);
        }
      }
    },
    settleInterruptedSession(sessionId: string) {
      const messages = this.sessions[sessionId];
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

      this.overlayDraftSessionId = sessionId;

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
        const response = await chat({
          message: trimmed,
          sessionId,
          workspaceId,
          quickAsk,
          modelId: compose.chatModel.trim() || undefined,
          modelProvider: compose.chatModelProvider.trim() || undefined,
          chatMode: compose.chatMode,
          toolApprovalMode: compose.toolApprovalMode,
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
