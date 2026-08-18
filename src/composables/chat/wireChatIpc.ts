import {
  listenChatContextNotice,
  listenChatDelta,
  listenChatError,
  listenChatFinished,
  listenChatReasoning,
  listenChatStarted,
  listenChatStatus,
  listenChatUserContent,
  listenSettingsChanged,
  listenToolFinished,
  listenToolStarted,
  listenTaskListUpdated,
  listenPlanModeChanged,
  listenFileOffer,
  listenUrlOffer,
} from "@/services/ipc";
import { normalizeToolActivityEvent, resolveSessionId } from "@/services/chat/normalize";
import { parseContextCompactedStatus } from "@/services/chat/compactMarker";
import { createRafBatch } from "@/services/chat/rafBatch";
import { recordToolActivityUsage } from "@/services/usage/resourceUsage";
import { createLogger } from "@/services/logger";
import { tr } from "@/services/i18n";
import { useChatStore } from "@/stores/chat";
import { useSettingStore } from "@/stores/setting";
import type {
  ChatContextNoticeEvent,
  ChatDeltaEvent,
  ChatErrorEvent,
  ChatFinishedEvent,
  ChatReasoningEvent,
  ChatStatusEvent,
  ChatUserContentEvent,
} from "@/types/chat";

type StreamBatchUpdate = {
  sessionId: string;
  messageId: string;
  contentDelta?: string;
  reasoningDelta?: string;
};

export interface ChatIpcDeps {
  chatStore: ReturnType<typeof useChatStore>;
  settingStore: ReturnType<typeof useSettingStore>;
}

/**
 * Wire every chat-related IPC event (stream deltas, tool activity, plan gate,
 * file/url offers, remote compose sync) into the chat store. Extracted from
 * `main.ts` bootstrap so the entry point stays about window bootstrapping.
 */
export async function wireChatIpc({ chatStore, settingStore }: ChatIpcDeps): Promise<void> {
  const log = createLogger("chatIpc");

  // Coalesce high-frequency stream deltas onto animation frames.
  const streamBatch = createRafBatch<StreamBatchUpdate>((batch) => {
    chatStore.applyStreamDeltas(
      batch.map((item) => ({
        ...item,
        fallbackSessionId: chatStore.overlayDraftSessionId,
      })),
    );
  });

  await listenSettingsChanged((settings) => {
    settingStore.applyPublicSettings(settings);
  });

  await listenChatStarted((payload) => {
    const sId = payload.sessionId;
    if (!sId) {
      return;
    }
    // Apply for overlay drafts and workbench sessions alike — remote Companion
    // sends never touch overlayDraftSessionId, but still need sending/sidebar state.
    chatStore.applyChatStarted(payload);
  });

  await listenChatContextNotice((payload) => {
    const event = payload as ChatContextNoticeEvent & {
      session_id?: string;
      folded_messages?: number;
      estimated_tokens?: number;
      context_window_tokens?: number;
    };
    const sId = resolveSessionId(event.sessionId, event.session_id);
    if (sId && (chatStore.sessions[sId] || sId === chatStore.overlayDraftSessionId)) {
      const estimatedTokens = event.estimatedTokens ?? event.estimated_tokens;
      const contextWindowTokens = event.contextWindowTokens ?? event.context_window_tokens;
      const prev = chatStore.contextUsage[sId];
      if (
        typeof estimatedTokens === "number" &&
        estimatedTokens > 0 &&
        typeof contextWindowTokens === "number" &&
        contextWindowTokens > 0
      ) {
        chatStore.setContextUsage(sId, {
          usageRatio: event.usageRatio,
          estimatedTokens,
          contextWindowTokens,
        });
      } else if (typeof event.usageRatio === "number") {
        chatStore.setContextUsage(sId, {
          usageRatio: event.usageRatio,
          estimatedTokens: prev?.estimatedTokens ?? 0,
          contextWindowTokens: prev?.contextWindowTokens ?? 0,
        });
      }
      // Compacted notices only refresh the usage ring. The in-thread
      // indicator is a transient activity label, not a history divider.
    }
  });

  await listenChatDelta((payload) => {
    const event = payload as ChatDeltaEvent & {
      session_id?: string;
      message_id?: string;
    };
    const sId = resolveSessionId(event.sessionId, event.session_id);
    if (sId && (chatStore.sessions[sId] || sId === chatStore.overlayDraftSessionId)) {
      streamBatch.push({
        sessionId: sId,
        messageId: event.messageId ?? event.message_id ?? "",
        contentDelta: event.delta,
      });
    }
  });

  await listenChatReasoning((payload) => {
    const event = payload as ChatReasoningEvent & {
      session_id?: string;
      message_id?: string;
    };
    const sId = resolveSessionId(event.sessionId, event.session_id);
    if (sId && (chatStore.sessions[sId] || sId === chatStore.overlayDraftSessionId)) {
      streamBatch.push({
        sessionId: sId,
        messageId: event.messageId ?? event.message_id ?? "",
        reasoningDelta: event.content,
      });
    }
  });

  await listenChatStatus((payload) => {
    const event = payload as ChatStatusEvent & {
      session_id?: string;
      message_id?: string;
      kind?: string;
    };
    const sId = resolveSessionId(event.sessionId, event.session_id);
    if (sId && (chatStore.sessions[sId] || sId === chatStore.overlayDraftSessionId)) {
      if (event.kind?.startsWith("stream_retry")) {
        streamBatch.drain();
      }
      const compacted = parseContextCompactedStatus(event.kind ?? "");
      if (
        compacted &&
        compacted.usageRatio != null &&
        compacted.estimatedTokens &&
        compacted.contextWindowTokens
      ) {
        chatStore.setContextUsage(sId, {
          usageRatio: compacted.usageRatio,
          estimatedTokens: compacted.estimatedTokens,
          contextWindowTokens: compacted.contextWindowTokens,
        });
      }
      chatStore.setActivityStatus(
        sId,
        event.messageId ?? event.message_id ?? "",
        event.kind ?? "",
        chatStore.overlayDraftSessionId,
      );
    }
  });

  await listenChatUserContent((payload) => {
    const event = payload as ChatUserContentEvent & {
      session_id?: string;
      message_id?: string;
    };
    const sId = resolveSessionId(event.sessionId, event.session_id);
    if (sId && (chatStore.sessions[sId] || sId === chatStore.overlayDraftSessionId)) {
      chatStore.patchMessageContent(
        sId,
        event.messageId ?? event.message_id ?? "",
        event.content,
        chatStore.overlayDraftSessionId,
      );
    }
  });

  await listenChatFinished((payload) => {
    streamBatch.drain();
    const event = payload as ChatFinishedEvent & {
      session_id?: string;
      message_id?: string;
    };
    const sId = resolveSessionId(event.sessionId, event.session_id);
    if (sId && (chatStore.sessions[sId] || sId === chatStore.overlayDraftSessionId)) {
      chatStore.finishMessage(
        sId,
        event.messageId ?? event.message_id ?? "",
        event.content,
        chatStore.overlayDraftSessionId,
        event.reasoning,
      );
    }
    // 本轮结束后（含用户停止）自动发出暂存消息。
    if (sId) {
      void chatStore.flushStaged(sId);
    }
  });

  await listenChatError((payload) => {
    streamBatch.drain();
    const event = payload as ChatErrorEvent & {
      session_id?: string;
      message_id?: string;
    };
    const sId = resolveSessionId(event.sessionId, event.session_id);
    if (sId && (chatStore.sessions[sId] || sId === chatStore.overlayDraftSessionId)) {
      chatStore.failMessage(
        sId,
        event.messageId ?? event.message_id ?? "",
        event.message,
        chatStore.overlayDraftSessionId,
      );
      void chatStore.flushStaged(sId);
    }
  });

  const handleToolActivity = (payload: unknown, options?: { recordUsage?: boolean }) => {
    streamBatch.drain();
    const normalized = normalizeToolActivityEvent(
      payload as Parameters<typeof normalizeToolActivityEvent>[0],
    );
    if (!normalized) {
      return;
    }
    if (options?.recordUsage && normalized.activity.status === "running") {
      recordToolActivityUsage(normalized.activity.toolName, normalized.activity.arguments);
    }
    const { sessionId, messageId, activity } = normalized;
    if (
      sessionId &&
      (chatStore.sessions[sessionId] || sessionId === chatStore.overlayDraftSessionId)
    ) {
      chatStore.upsertToolActivity(sessionId, messageId, activity, chatStore.overlayDraftSessionId);
    }
  };

  await listenToolStarted((payload) => handleToolActivity(payload, { recordUsage: true }));
  await listenToolFinished(handleToolActivity);

  await listenFileOffer((payload) => {
    chatStore.applyFileOffer(payload, chatStore.overlayDraftSessionId);
  });
  await listenUrlOffer((payload) => {
    chatStore.applyUrlOffer(payload, chatStore.overlayDraftSessionId);
  });

  await listenTaskListUpdated((payload) => {
    const sessionId = resolveSessionId(payload.sessionId, chatStore.overlayDraftSessionId);
    if (!sessionId) return;
    chatStore.setSessionTasks(sessionId, payload.tasks ?? []);
  });

  await listenPlanModeChanged((payload) => {
    const sessionId = resolveSessionId(payload.sessionId, chatStore.overlayDraftSessionId);
    if (!sessionId) return;
    const active = Boolean(payload.active);
    const source = payload.source === "auto" ? "auto" : "manual";
    chatStore.setSessionPlanMode(sessionId, active);
    // A rejected plan fingerprint still blocks countdown in MessageList until
    // the checklist is new/updated — keep the trigger from the backend so a
    // revised auto-plan can countdown again.
    chatStore.setSessionPlanTrigger(sessionId, active ? source : "manual");
    // Never rewrite the user's mode chip when the writer gate flips. Auto-plan
    // and sticky gates are independent from Agent/Ask/Plan picker choice.
  });

  // Companion ↔ desktop compose / plan sync (Remote Gateway).
  try {
    const { listen } = await import("@tauri-apps/api/event");
    await listen<{
      sessionId?: string;
      compose?: {
        chatMode?: "ask" | "agent" | "plan";
        toolApprovalMode?: "ask" | "auto" | "alwaysAllow";
        chatModel?: string;
        chatModelProvider?: string;
      };
      source?: string;
    }>("remote-compose-changed", (event) => {
      const payload = event.payload;
      const sessionId = payload.sessionId;
      if (!sessionId || !payload.compose) {
        return;
      }
      chatStore.applyComposeFromRemote(sessionId, {
        chatMode: payload.compose.chatMode,
        toolApprovalMode: payload.compose.toolApprovalMode,
        chatModel: payload.compose.chatModel,
        chatModelProvider: payload.compose.chatModelProvider,
      });
    });
    await listen<{ sessionId?: string }>("remote-compose-needed", (event) => {
      const sessionId = event.payload.sessionId;
      if (!sessionId) {
        return;
      }
      chatStore.ensureCompose(sessionId);
    });
    await listen<{ sessionId?: string }>("remote-plan-approve", (event) => {
      const sessionId = event.payload.sessionId;
      if (!sessionId) {
        return;
      }
      // Mirror desktop "批准并执行": leave plan gate and resume execution.
      // Approval text is persisted into history / Companion inbox.
      void chatStore.send(tr(settingStore.language, "planModeExecuteMessage"), sessionId, {
        resumePlan: true,
        skipAutoPlan: true,
      });
    });
  } catch (error) {
    log.warn("remote compose listeners unavailable", error);
  }
}
