import { createApp } from "vue";
import { createPinia } from "pinia";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import App from "./App.vue";
import router from "./router";
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
  listenSettingsOpened,
  listenToolFinished,
  listenToolStarted,
  listenTaskListUpdated,
  listenPlanModeChanged,
  listenFileOffer,
  listenUrlOffer,
} from "@/services/ipc";
import { normalizeToolActivityEvent, resolveSessionId } from "@/services/chat/normalize";
import { createRafBatch } from "@/services/chat/rafBatch";
import { hideBootSplash, waitForNextPaint } from "@/services/bootSplash";
import { markPeekWindow } from "@/services/overlay/appearance";
import { installBrowserGuards } from "@/services/browserGuards";
import { createLogger, rootLogger } from "@/services/logger";
import { recordToolActivityUsage } from "@/services/usage/resourceUsage";
import { warmInstalledResourceIcons } from "@/services/warmIcons";
import "@/services/motion/gsapSafe";
import type {
  ChatContextNoticeEvent,
  ChatDeltaEvent,
  ChatErrorEvent,
  ChatFinishedEvent,
  ChatReasoningEvent,
  ChatStatusEvent,
  ChatUserContentEvent,
} from "@/types/chat";
import { useChatStore } from "@/stores/chat";
import { applyTheme, bootstrapThemeHint, useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import "./styles/index.css";

installBrowserGuards();

const app = createApp(App);
const pinia = createPinia();
const bootLog = createLogger("bootstrap");

function formatErrorDetail(err: unknown): Record<string, unknown> {
  if (err instanceof Error) {
    return {
      name: err.name,
      message: err.message,
      stack: err.stack,
    };
  }
  if (typeof err === "string") {
    return { message: err };
  }
  if (err && typeof err === "object") {
    const record = err as { message?: unknown; name?: unknown; stack?: unknown };
    return {
      name: typeof record.name === "string" ? record.name : undefined,
      message:
        typeof record.message === "string" ? record.message : Object.prototype.toString.call(err),
      stack: typeof record.stack === "string" ? record.stack : undefined,
    };
  }
  return { message: String(err) };
}

app.config.errorHandler = (err, _instance, info) => {
  rootLogger.error("vue errorHandler", {
    info,
    err: formatErrorDetail(err),
  });
};

window.addEventListener("unhandledrejection", (event) => {
  rootLogger.error("unhandledrejection", {
    reason:
      event.reason instanceof Error
        ? { message: event.reason.message, stack: event.reason.stack }
        : event.reason,
  });
});

window.addEventListener("error", (event) => {
  rootLogger.error("window error", {
    message: event.message,
    filename: event.filename,
    lineno: event.lineno,
    colno: event.colno,
  });
});

app.use(pinia);
app.use(router);

const settingStore = useSettingStore();
const chatStore = useChatStore();

type StreamBatchUpdate = {
  sessionId: string;
  messageId: string;
  contentDelta?: string;
  reasoningDelta?: string;
};

/** Coalesce high-frequency stream deltas onto animation frames. */
const streamBatch = createRafBatch<StreamBatchUpdate>((batch) => {
  chatStore.applyStreamDeltas(
    batch.map((item) => ({
      ...item,
      fallbackSessionId: chatStore.overlayDraftSessionId,
    })),
  );
});

/**
 * Boot the correct window surface (workbench / overlay / settings),
 * wire chat IPC listeners, then drop the HTML splash once paint is ready.
 */
async function bootstrap() {
  const webviewWindow = getCurrentWebviewWindow();
  const windowLabel = webviewWindow.label;
  const isOverlay =
    (windowLabel === "overlay" || windowLabel.startsWith("overlay-")) &&
    !windowLabel.startsWith("overlay-preview-");

  // Resolve each interactive route before loading settings. Keep the HTML
  // boot splash up until the workbench loading layer has painted, so we never
  // cut to a blank frame between splash → Suspense → Main loading.
  if (windowLabel === "workbench") {
    void router.replace("/workbench");
    applyTheme(bootstrapThemeHint(settingStore.language));
    // Load persisted settings before Main mounts. Otherwise its first render
    // sees the default onboardingCompleted=false and opens the wizard before
    // the persisted value arrives.
    await settingStore.load();
    applyTheme({
      colorScheme: settingStore.colorScheme,
      language: settingStore.language,
    });
    void warmInstalledResourceIcons(settingStore.mcpServers);
    app.mount("#app");
    await router.isReady();
    await waitForNextPaint();
    hideBootSplash({ fadeMs: 220 });
  } else if (isOverlay) {
    markPeekWindow();
    hideBootSplash({ fadeMs: 0 });
    void router.replace("/overlay");
    applyTheme(bootstrapThemeHint(settingStore.language));
    await settingStore.load();
    applyTheme({
      colorScheme: settingStore.colorScheme,
      language: settingStore.language,
    });
    void warmInstalledResourceIcons(settingStore.mcpServers);
    app.mount("#app");
    await router.isReady();
    await waitForNextPaint();
  } else {
    applyTheme(bootstrapThemeHint(settingStore.language));
    await settingStore.load();
    applyTheme({
      colorScheme: settingStore.colorScheme,
      language: settingStore.language,
    });
    void warmInstalledResourceIcons(settingStore.mcpServers);
  }

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
    };
    const sId = resolveSessionId(event.sessionId, event.session_id);
    if (sId && (chatStore.sessions[sId] || sId === chatStore.overlayDraftSessionId)) {
      chatStore.setContextNotice(sId, event.message);
      const prev = chatStore.contextUsage[sId];
      chatStore.setContextUsage(sId, {
        usageRatio: event.usageRatio,
        estimatedTokens: prev?.estimatedTokens ?? 0,
        contextWindowTokens: prev?.contextWindowTokens ?? 0,
      });
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
    bootLog.warn("remote compose listeners unavailable", error);
  }

  if (windowLabel.startsWith("overlay-preview-")) {
    document.documentElement.classList.add("peek-window");
    await router.replace("/image-preview");
  } else if (isOverlay) {
    // The overlay route was mounted eagerly above.
  } else if (windowLabel === "settings") {
    await router.replace("/settings");

    await listenSettingsOpened(() => {
      if (router.currentRoute.value.path !== "/settings") {
        void router.replace("/settings");
      }

      const root = document.getElementById("app");
      if (!root?.firstElementChild) {
        globalThis.location.reload();
      }
    });
  }

  await router.isReady();
  if (windowLabel !== "workbench" && !isOverlay) {
    app.mount("#app");
    await waitForNextPaint();
    hideBootSplash({ fadeMs: 180 });
  }

  bootLog.info("ready", { windowLabel });
}

void bootstrap().catch((err) => {
  bootLog.error("bootstrap failed", err);
});
