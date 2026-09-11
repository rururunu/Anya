import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { MascotExpression } from "@/components/icons/MascotPetView.vue";
import { IPC_EVENTS } from "@/types/ipc";
import { resolveSessionId } from "@/services/chat/normalize";

export type SessionAgentStatus = "thinking" | "working" | "talking";

export interface SessionState {
  sessionId: string;
  status: SessionAgentStatus;
  activeTools: Set<string>;
  lastActiveAt: number;
}

export interface UseDesktopPetAgentSessionsOptions {
  onActivity?: () => void;
  watchdogIntervalMs?: number;
  sessionTimeoutMs?: number;
  doneDurationMs?: number;
  errorDurationMs?: number;
}

const DEFAULT_SESSION_TIMEOUT_MS = 120_000;
const DEFAULT_WATCHDOG_INTERVAL_MS = 15_000;
const DEFAULT_DONE_DURATION_MS = 3_500;
const DEFAULT_ERROR_DURATION_MS = 4_000;

/**
 * 根据多会话状态集合与瞬态表情计算宠物的聚合表情。
 */
export function resolveAgentExpression(
  sessions: Iterable<SessionState>,
  transient: "done" | "error" | null,
): MascotExpression {
  let hasWorking = false;
  let hasTalking = false;
  let hasThinking = false;

  for (const session of sessions) {
    if (session.status === "working") {
      hasWorking = true;
      break;
    } else if (session.status === "talking") {
      hasTalking = true;
    } else if (session.status === "thinking") {
      hasThinking = true;
    }
  }

  if (hasWorking) return "working";
  if (hasTalking) return "talking";
  if (hasThinking) return "thinking";
  if (transient) return transient;
  return "idle";
}

/**
 * 管理桌面宠物的多会话 Agent 状态跟踪、优先级仲裁与防卡死看门狗。
 */
export function useDesktopPetAgentSessions(options: UseDesktopPetAgentSessionsOptions = {}) {
  const {
    onActivity,
    watchdogIntervalMs = DEFAULT_WATCHDOG_INTERVAL_MS,
    sessionTimeoutMs = DEFAULT_SESSION_TIMEOUT_MS,
    doneDurationMs = DEFAULT_DONE_DURATION_MS,
    errorDurationMs = DEFAULT_ERROR_DURATION_MS,
  } = options;

  const activeSessions = ref<Map<string, SessionState>>(new Map());
  const transientState = ref<"done" | "error" | null>(null);

  let transientTimer: ReturnType<typeof setTimeout> | null = null;
  let watchdogTimer: ReturnType<typeof setInterval> | null = null;
  const unlistenFns: UnlistenFn[] = [];

  function clearTransientTimer() {
    if (transientTimer) {
      clearTimeout(transientTimer);
      transientTimer = null;
    }
  }

  const agentExpression = computed<MascotExpression>(() => {
    return resolveAgentExpression(activeSessions.value.values(), transientState.value);
  });

  function getOrCreateSession(sessionId: string, initialStatus: SessionAgentStatus): SessionState {
    let session = activeSessions.value.get(sessionId);
    if (!session) {
      session = {
        sessionId,
        status: initialStatus,
        activeTools: new Set(),
        lastActiveAt: Date.now(),
      };
      activeSessions.value.set(sessionId, session);
    }
    return session;
  }

  /** 处理会话开始事件。 */
  function handleChatStarted(payload: unknown) {
    const raw = payload as { sessionId?: string; session_id?: string } | undefined;
    const sId = resolveSessionId(raw?.sessionId, raw?.session_id) || "default-session";
    clearTransientTimer();
    transientState.value = null;
    activeSessions.value.set(sId, {
      sessionId: sId,
      status: "thinking",
      activeTools: new Set(),
      lastActiveAt: Date.now(),
    });
    onActivity?.();
  }

  /** 处理模型推理/思考事件。 */
  function handleChatReasoning(payload: unknown) {
    const raw = payload as { sessionId?: string; session_id?: string } | undefined;
    const sId = resolveSessionId(raw?.sessionId, raw?.session_id) || "default-session";
    clearTransientTimer();
    transientState.value = null;
    const session = getOrCreateSession(sId, "thinking");
    if (session.activeTools.size === 0 && session.status !== "talking") {
      session.status = "thinking";
    }
    session.lastActiveAt = Date.now();
    activeSessions.value.set(sId, { ...session });
    onActivity?.();
  }

  /** 处理会话状态标签事件。 */
  function handleChatStatus(payload: unknown) {
    const raw = payload as { sessionId?: string; session_id?: string } | undefined;
    const sId = resolveSessionId(raw?.sessionId, raw?.session_id) || "default-session";
    clearTransientTimer();
    transientState.value = null;
    const session = getOrCreateSession(sId, "thinking");
    if (session.activeTools.size === 0 && session.status !== "talking") {
      session.status = "thinking";
    }
    session.lastActiveAt = Date.now();
    activeSessions.value.set(sId, { ...session });
    onActivity?.();
  }

  /** 处理模型流式输出文本增量事件。 */
  function handleChatDelta(payload: unknown) {
    const raw = payload as { sessionId?: string; session_id?: string } | undefined;
    const sId = resolveSessionId(raw?.sessionId, raw?.session_id) || "default-session";
    clearTransientTimer();
    transientState.value = null;
    const session = getOrCreateSession(sId, "talking");
    if (session.activeTools.size === 0) {
      session.status = "talking";
    }
    session.lastActiveAt = Date.now();
    activeSessions.value.set(sId, { ...session });
    onActivity?.();
  }

  /** 处理工具开始执行事件。 */
  function handleToolStarted(payload: unknown) {
    const raw = payload as
      | {
          sessionId?: string;
          session_id?: string;
          activityId?: string;
          activity_id?: string;
          toolName?: string;
          tool_name?: string;
        }
      | undefined;
    const sId = resolveSessionId(raw?.sessionId, raw?.session_id) || "default-session";
    clearTransientTimer();
    transientState.value = null;
    const session = getOrCreateSession(sId, "working");
    const actId = raw?.activityId ?? raw?.activity_id ?? raw?.toolName ?? raw?.tool_name ?? "tool";
    session.activeTools.add(actId);
    session.status = "working";
    session.lastActiveAt = Date.now();
    activeSessions.value.set(sId, { ...session });
    onActivity?.();
  }

  /** 处理工具完成执行事件。 */
  function handleToolFinished(payload: unknown) {
    const raw = payload as
      | {
          sessionId?: string;
          session_id?: string;
          activityId?: string;
          activity_id?: string;
          toolName?: string;
          tool_name?: string;
        }
      | undefined;
    const sId = resolveSessionId(raw?.sessionId, raw?.session_id) || "default-session";
    const session = activeSessions.value.get(sId);
    if (session) {
      const actId =
        raw?.activityId ?? raw?.activity_id ?? raw?.toolName ?? raw?.tool_name ?? "tool";
      session.activeTools.delete(actId);
      if (session.activeTools.size === 0) {
        session.status = "thinking";
      }
      session.lastActiveAt = Date.now();
      activeSessions.value.set(sId, { ...session });
    }
    onActivity?.();
  }

  /** 处理会话结束或取消事件。 */
  function handleChatFinished(payload: unknown) {
    const raw = payload as
      | {
          sessionId?: string;
          session_id?: string;
          finishReason?: string;
          finish_reason?: string;
        }
      | undefined;
    const sId = resolveSessionId(raw?.sessionId, raw?.session_id) || "default-session";
    activeSessions.value.delete(sId);

    const finishReason = raw?.finishReason ?? raw?.finish_reason;
    const isCancelled = finishReason === "cancelled" || finishReason === "cancel";

    if (isCancelled) {
      if (activeSessions.value.size === 0) {
        clearTransientTimer();
        transientState.value = null;
      }
    } else {
      if (activeSessions.value.size === 0) {
        clearTransientTimer();
        transientState.value = "done";
        transientTimer = setTimeout(() => {
          transientState.value = null;
        }, doneDurationMs);
      }
    }
    onActivity?.();
  }

  /** 处理会话异常事件。 */
  function handleChatError(payload: unknown) {
    const raw = payload as { sessionId?: string; session_id?: string } | undefined;
    const sId = resolveSessionId(raw?.sessionId, raw?.session_id) || "default-session";
    activeSessions.value.delete(sId);
    if (activeSessions.value.size === 0) {
      clearTransientTimer();
      transientState.value = "error";
      transientTimer = setTimeout(() => {
        transientState.value = null;
      }, errorDurationMs);
    }
    onActivity?.();
  }

  /** 清理所有会话状态。 */
  function clearAllSessions() {
    activeSessions.value.clear();
    clearTransientTimer();
    transientState.value = null;
  }

  /** 看门狗检查：清理无响应超时的孤儿会话，防止永久卡死。 */
  function runWatchdog() {
    const now = Date.now();
    let changed = false;
    for (const [sId, session] of activeSessions.value.entries()) {
      if (now - session.lastActiveAt > sessionTimeoutMs) {
        activeSessions.value.delete(sId);
        changed = true;
      }
    }
    if (changed && activeSessions.value.size === 0 && transientState.value === null) {
      onActivity?.();
    }
  }

  onMounted(async () => {
    try {
      const u1 = await listen(IPC_EVENTS.chatStarted, (e) => handleChatStarted(e.payload));
      unlistenFns.push(u1);
      const u2 = await listen(IPC_EVENTS.chatReasoning, (e) => handleChatReasoning(e.payload));
      unlistenFns.push(u2);
      const u3 = await listen(IPC_EVENTS.chatStatus, (e) => handleChatStatus(e.payload));
      unlistenFns.push(u3);
      const u4 = await listen(IPC_EVENTS.chatDelta, (e) => handleChatDelta(e.payload));
      unlistenFns.push(u4);
      const u5 = await listen(IPC_EVENTS.toolStarted, (e) => handleToolStarted(e.payload));
      unlistenFns.push(u5);
      const u6 = await listen(IPC_EVENTS.toolFinished, (e) => handleToolFinished(e.payload));
      unlistenFns.push(u6);
      const u7 = await listen(IPC_EVENTS.chatFinished, (e) => handleChatFinished(e.payload));
      unlistenFns.push(u7);
      const u8 = await listen(IPC_EVENTS.chatError, (e) => handleChatError(e.payload));
      unlistenFns.push(u8);
    } catch (err) {
      console.warn("Failed to listen agent session events:", err);
    }

    watchdogTimer = setInterval(runWatchdog, watchdogIntervalMs);
  });

  onBeforeUnmount(() => {
    clearTransientTimer();
    if (watchdogTimer) {
      clearInterval(watchdogTimer);
      watchdogTimer = null;
    }
    for (const fn of unlistenFns) fn();
    unlistenFns.length = 0;
  });

  return {
    activeSessions,
    transientState,
    agentExpression,
    handleChatStarted,
    handleChatReasoning,
    handleChatStatus,
    handleChatDelta,
    handleToolStarted,
    handleToolFinished,
    handleChatFinished,
    handleChatError,
    clearAllSessions,
    runWatchdog,
  };
}
