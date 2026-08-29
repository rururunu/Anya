import { defineStore } from "pinia";

import { branchChatSession, listChatSessions, regenerateChatSessionTitle } from "@/services/ipc";
import { resolveSessionId } from "@/services/chat/normalize";
import { createLogger } from "@/services/logger";
import { composeCache, loadComposeCache } from "./chatCompose";
import type { ChatMessage, ChatSessionSummary } from "@/types/chat";
import type { SessionCompose } from "./chatCompose";

const log = createLogger("chat-sessions-store");

export type ArchiveVisualState = "shown" | "leaving";

const ARCHIVE_SHOWN_MS = 420;
const ARCHIVE_LEAVE_MS = 280;

function delay(ms: number) {
  return new Promise<void>((resolve) => {
    window.setTimeout(resolve, ms);
  });
}

/**
 * Per-session message mirror, overlay draft routing, sidebar session list,
 * archive / title / branch IPC helpers.
 */
export const useChatSessionsStore = defineStore("chatSessions", {
  state: () => ({
    sessions: {} as Record<string, ChatMessage[]>,
    startedSessionIds: {} as Record<string, boolean>,
    overlayDraftSessionId: "" as string,
    /**
     * Bumps when draft presence / draft-only sidebar rows should refresh.
     * Intentionally separate from sessionCompose so typing does not rebuild
     * the whole session list on every keystroke.
     */
    draftListVersion: 0,
    summaries: [] as ChatSessionSummary[],
    sessionsLoading: false,
    titleGeneratingSessionIds: [] as string[],
    archiveVisualBySessionId: {} as Record<string, ArchiveVisualState>,
  }),
  getters: {
    overlayMessages(state): ChatMessage[] {
      const sessionId = state.overlayDraftSessionId;
      if (!sessionId) {
        return [];
      }
      return state.sessions[sessionId] ?? [];
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
    setSummaries(summaries: ChatSessionSummary[]) {
      this.summaries = summaries;
    },
    patchSummary(sessionId: string, patch: Partial<ChatSessionSummary>) {
      this.summaries = this.summaries.map((session) =>
        session.sessionId === sessionId ? { ...session, ...patch } : session,
      );
    },
    removeSummary(sessionId: string) {
      this.summaries = this.summaries.filter((session) => session.sessionId !== sessionId);
    },
    bumpDraftListVersion() {
      this.draftListVersion += 1;
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
    removeSessionMessages(sessionId: string) {
      if (!sessionId || !(sessionId in this.sessions)) {
        return;
      }
      const next = { ...this.sessions };
      delete next[sessionId];
      this.sessions = next;
    },
    resolveOverlaySessionId(preferred?: string) {
      // Prefer the event/request session id. Only fall back to the overlay draft
      // when the payload omitted a session (legacy / incomplete IPC).
      // Preferring overlayDraft first remapped Companion "new chat" turns into
      // whatever conversation was last sent from the desktop.
      return resolveSessionId(preferred, this.overlayDraftSessionId);
    },
    /** True when the conversation has unsent composer text cached. */
    sessionHasDraft(sessionId: string, sessionCompose?: Record<string, SessionCompose>): boolean {
      if (!sessionId) return false;
      loadComposeCache();
      const compose = sessionCompose?.[sessionId] ?? composeCache.entries[sessionId];
      return Boolean(compose?.draft?.trim());
    },
    /** Local draft-only sessions that are not yet in the backend session list. */
    listDraftOnlySessions(
      knownSessionIds: Iterable<string>,
      sessionCompose?: Record<string, SessionCompose>,
    ): Array<{
      sessionId: string;
      workspaceId?: string;
      preview: string;
      updatedAt: number;
    }> {
      loadComposeCache();
      const known = new Set(knownSessionIds);
      const entries = sessionCompose
        ? { ...composeCache.entries, ...sessionCompose }
        : composeCache.entries;
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
    async refreshSummaries() {
      this.sessionsLoading = true;
      try {
        const chatResponse = await listChatSessions();
        this.summaries = chatResponse.sessions;
        this.setStartedSessionIds(chatResponse.sessions.map((session) => session.sessionId));
      } catch (error) {
        log.error("list_chat_sessions failed", error);
      } finally {
        this.sessionsLoading = false;
      }
    },
    async regenerateTitle(sessionId: string) {
      if (!sessionId || this.titleGeneratingSessionIds.includes(sessionId)) return;
      this.titleGeneratingSessionIds = [...this.titleGeneratingSessionIds, sessionId];
      try {
        await regenerateChatSessionTitle(sessionId);
        await this.refreshSummaries();
      } catch (error) {
        log.error("regenerate_chat_session_title failed", error);
      } finally {
        this.titleGeneratingSessionIds = this.titleGeneratingSessionIds.filter(
          (id) => id !== sessionId,
        );
      }
    },
    async branchSession(sessionId: string, messageId?: string) {
      if (!sessionId) return null;
      try {
        return await branchChatSession(sessionId, messageId);
      } catch (error) {
        log.error("branch_chat_session failed", error);
        return null;
      }
    },
    async playArchiveVisual(sessionId: string) {
      if (!sessionId || this.archiveVisualBySessionId[sessionId]) return;

      this.archiveVisualBySessionId = {
        ...this.archiveVisualBySessionId,
        [sessionId]: "shown",
      };
      await delay(ARCHIVE_SHOWN_MS);

      this.archiveVisualBySessionId = {
        ...this.archiveVisualBySessionId,
        [sessionId]: "leaving",
      };
      await delay(ARCHIVE_LEAVE_MS);

      const { [sessionId]: _removed, ...remainingVisual } = this.archiveVisualBySessionId;
      this.archiveVisualBySessionId = remainingVisual;
    },
  },
});
