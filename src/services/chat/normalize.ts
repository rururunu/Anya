import type {
  ChatMessage,
  MessageStatus,
  Role,
  ToolActivity,
  ToolPreviewPayload,
} from "@/types/chat";
import { isSoftInjectContent } from "@/services/chat/softInject";

type RawMessage = Partial<ChatMessage> & {
  session_id?: string;
};

export type RawChatStarted = {
  sessionId?: string;
  session_id?: string;
  resumePlan?: boolean;
  userMessage?: RawMessage;
  user_message?: RawMessage;
  assistantMessage?: RawMessage;
  assistant_message?: RawMessage;
};

/** Pick the first non-empty session id from camelCase / snake_case IPC payloads. */
export function resolveSessionId(...candidates: Array<string | undefined>): string {
  for (const candidate of candidates) {
    if (candidate?.trim()) {
      return candidate;
    }
  }
  return "";
}

/** Normalize mixed-case / unknown role strings into a known chat role. */
export function normalizeRole(role: Role | string | undefined): Role {
  const value = String(role ?? "assistant").toLowerCase();
  if (value === "user" || value === "assistant" || value === "system" || value === "tool") {
    return value;
  }
  return "assistant";
}

export function normalizeStatus(status: MessageStatus | string | undefined): MessageStatus {
  const value = String(status ?? "done").toLowerCase();
  if (
    value === "pending" ||
    value === "streaming" ||
    value === "done" ||
    value === "error" ||
    value === "cancelled"
  ) {
    return value;
  }
  return "done";
}

export function normalizeMessage(
  raw: RawMessage | undefined,
  fallbackSessionId?: string,
): ChatMessage | null {
  if (!raw?.id) {
    return null;
  }

  const sessionId = resolveSessionId(raw.sessionId, raw.session_id, fallbackSessionId);
  if (!sessionId) {
    return null;
  }

  return {
    id: raw.id,
    sessionId,
    role: normalizeRole(raw.role),
    content: raw.content ?? "",
    reasoning: raw.reasoning,
    workTimeline: raw.workTimeline,
    toolActivities: raw.toolActivities,
    askUserAnswer: raw.askUserAnswer,
    injected: raw.injected === true || isSoftInjectContent(raw.content ?? ""),
    status: normalizeStatus(raw.status),
    timestamp: raw.timestamp ?? Date.now(),
    estimatedTokens: raw.estimatedTokens,
    completedAt: raw.completedAt,
  };
}

export function normalizeChatStarted(raw: RawChatStarted) {
  const sessionId = resolveSessionId(
    raw.sessionId,
    raw.session_id,
    raw.userMessage?.sessionId,
    raw.user_message?.session_id,
    raw.assistantMessage?.sessionId,
    raw.assistant_message?.session_id,
  );
  const userMessage = normalizeMessage(raw.userMessage ?? raw.user_message, sessionId);
  const assistantMessage = normalizeMessage(
    raw.assistantMessage ?? raw.assistant_message,
    sessionId,
  );

  if (!sessionId || !userMessage || !assistantMessage) {
    return null;
  }

  return { sessionId, userMessage, assistantMessage, resumePlan: raw.resumePlan === true };
}

type RawToolActivityEvent = {
  sessionId?: string;
  session_id?: string;
  messageId?: string;
  message_id?: string;
  activityId?: string;
  activity_id?: string;
  subagentId?: string;
  subagent_id?: string;
  parentActivityId?: string;
  parent_activity_id?: string;
  toolName?: string;
  tool_name?: string;
  title?: string;
  kind?: string;
  detail?: string;
  arguments?: Record<string, unknown>;
  result?: string;
  preview?: ToolPreviewPayload | null;
  success?: boolean;
  status?: string;
};

export function normalizeToolActivityEvent(raw: RawToolActivityEvent): {
  sessionId: string;
  messageId: string;
  activity: ToolActivity;
} | null {
  const sessionId = resolveSessionId(raw.sessionId, raw.session_id);
  const messageId = raw.messageId ?? raw.message_id ?? "";
  const activityId = raw.activityId ?? raw.activity_id ?? "";
  if (!sessionId || !messageId || !activityId) {
    return null;
  }

  const statusRaw = String(raw.status ?? "done").toLowerCase();
  const status =
    statusRaw === "running" || statusRaw === "done" || statusRaw === "error" ? statusRaw : "done";

  return {
    sessionId,
    messageId,
    activity: {
      id: activityId,
      subagentId: raw.subagentId ?? raw.subagent_id,
      parentActivityId: raw.parentActivityId ?? raw.parent_activity_id,
      toolName: raw.toolName ?? raw.tool_name ?? "tool",
      title: raw.title ?? raw.toolName ?? raw.tool_name ?? "工具调用",
      kind: raw.kind ?? "other",
      detail: raw.detail,
      arguments: raw.arguments,
      result: raw.result,
      preview: raw.preview,
      success: raw.success ?? status !== "error",
      status,
    },
  };
}
