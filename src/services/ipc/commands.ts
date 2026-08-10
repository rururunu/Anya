import type { AppInfo } from "@/types/app";
import type {
  AgentDebugEvent,
  ChatCancelRequest,
  CapturedContext,
  ChatDeltaEvent,
  ChatErrorEvent,
  ChatFinishedEvent,
  ChatHistoryRequest,
  ChatHistoryResponse,
  ChatModelInfo,
  ChatReasoningEvent,
  ChatSendRequest,
  ChatSendResponse,
  ChatStartedEvent,
  ChatStatusEvent,
  ChatUserContentEvent,
  ContextUsageRequest,
  ContextUsageResponse,
  ListChatSessionsResponse,
  RespondAskUserRequest,
  RespondPathPermissionRequest,
  RespondToolApprovalRequest,
  CheckpointInfo,
  RewindSessionRequest,
  RewindSessionResponse,
} from "@/types/chat";
import { IPC_COMMANDS } from "@/types/ipc";
import type { AppSettings, AppSettingsPatch, GeminiAuthStatus } from "@/types/setting";
import type { TokenUsageReport } from "@/types/tokenUsage";
import { invoke } from "@tauri-apps/api/core";

export function ipcInvoke<TResponse>(
  command: string,
  payload?: Record<string, unknown>,
): Promise<TResponse> {
  return payload ? invoke<TResponse>(command, payload) : invoke<TResponse>(command);
}

export function openSettings() {
  return ipcInvoke<void>(IPC_COMMANDS.openSettings);
}

export function openSessionInOverlay(sessionId: string) {
  return ipcInvoke<void>(IPC_COMMANDS.openSessionInOverlay, {
    sessionId,
    session_id: sessionId,
  });
}

export function openSessionInWorkbench(sessionId: string, overlayLabel: string) {
  return ipcInvoke<void>(IPC_COMMANDS.openSessionInWorkbench, {
    sessionId,
    overlayLabel,
  });
}

export function showInteractionNotification(request: {
  sessionId: string;
  title: string;
  body: string;
  ignoreLabel: string;
  openLabel: string;
  persistent?: boolean;
}) {
  return ipcInvoke<void>(IPC_COMMANDS.showInteractionNotification, { request });
}

export function setWindowSessionView(sessionId?: string) {
  return ipcInvoke<void>(IPC_COMMANDS.setWindowSessionView, {
    sessionId: sessionId || null,
  });
}

export function hideOverlay(label?: string) {
  return ipcInvoke<void>(IPC_COMMANDS.hideOverlay, label ? { label } : undefined);
}

export function minimizeOverlay(label?: string) {
  return ipcInvoke<void>(IPC_COMMANDS.minimizeOverlay, label ? { label } : undefined);
}

export function closeOverlay(label: string) {
  return ipcInvoke<void>(IPC_COMMANDS.closeOverlay, { label });
}

export function exitApp() {
  return ipcInvoke<void>(IPC_COMMANDS.exitApp);
}

export function getAppSettings() {
  return ipcInvoke<AppSettings>(IPC_COMMANDS.getAppSettings);
}

export function setAppSettings(patch: AppSettingsPatch) {
  return ipcInvoke<AppSettings>(IPC_COMMANDS.setAppSettings, { patch });
}

export function geminiAuthStatus() {
  return ipcInvoke<GeminiAuthStatus>(IPC_COMMANDS.geminiAuthStatus);
}

export function geminiOauthLogin() {
  return ipcInvoke<GeminiAuthStatus>(IPC_COMMANDS.geminiOauthLogin);
}

export function geminiOauthCancelLogin() {
  return ipcInvoke<void>(IPC_COMMANDS.geminiOauthCancelLogin);
}

export function geminiOauthLogout() {
  return ipcInvoke<GeminiAuthStatus>(IPC_COMMANDS.geminiOauthLogout);
}

export function geminiImportClientSecrets(path: string) {
  return ipcInvoke<GeminiAuthStatus>(IPC_COMMANDS.geminiImportClientSecrets, {
    path,
  });
}

export function getAppInfo() {
  return ipcInvoke<AppInfo>(IPC_COMMANDS.getAppInfo);
}

export function chat(request: ChatSendRequest) {
  return ipcInvoke<ChatSendResponse>(IPC_COMMANDS.chat, { request });
}

export function chatCancel(request: ChatCancelRequest) {
  return ipcInvoke<void>(IPC_COMMANDS.chatCancel, { request });
}

export function getAgentDebugSnapshot() {
  return ipcInvoke<AgentDebugEvent[]>(IPC_COMMANDS.agentDebugSnapshot);
}

export function chatHistory(request: ChatHistoryRequest = {}) {
  return ipcInvoke<ChatHistoryResponse>(IPC_COMMANDS.chatHistory, { request });
}

export function listChatSessions() {
  return ipcInvoke<ListChatSessionsResponse>(IPC_COMMANDS.listChatSessions);
}

export function listChatModels() {
  return ipcInvoke<ChatModelInfo[]>(IPC_COMMANDS.listChatModels);
}

export function listCustomProviderModels(baseUrl: string, apiKey: string) {
  return ipcInvoke<string[]>(IPC_COMMANDS.listCustomProviderModels, {
    baseUrl,
    apiKey,
  });
}

export function getContextUsage(request: ContextUsageRequest = {}) {
  return ipcInvoke<ContextUsageResponse>(IPC_COMMANDS.getContextUsage, {
    request,
  });
}

export function getTokenUsageReport(request: {
  from?: number;
  to?: number;
  granularity: "day" | "week" | "month";
}) {
  return ipcInvoke<TokenUsageReport>(IPC_COMMANDS.getTokenUsageReport, {
    request,
  });
}

export function getEnvironmentContext() {
  return ipcInvoke<CapturedContext>(IPC_COMMANDS.getEnvironmentContext);
}

export function deleteChatSession(sessionId: string) {
  return ipcInvoke<void>("delete_chat_session", { sessionId });
}

export function clearAllChatSessions() {
  return ipcInvoke<void>("clear_all_chat_sessions");
}

export function setOverlayChatMode(label: string, enabled: boolean) {
  return ipcInvoke<void>(IPC_COMMANDS.setOverlayChatMode, { label, enabled });
}

export function setOverlayPopupOpen(label: string, open: boolean) {
  return ipcInvoke<void>(IPC_COMMANDS.setOverlayPopupOpen, { label, open });
}

export function takeOverlayContext(label: string) {
  return ipcInvoke<CapturedContext | null>(IPC_COMMANDS.takeOverlayContext, {
    label,
  });
}

export function openImagePreview(pathOrBase64: string) {
  return ipcInvoke<void>("open_image_preview", {
    pathOrBase64,
    path_or_base64: pathOrBase64,
  });
}

export function getPreviewImage() {
  return ipcInvoke<string>("get_preview_image");
}

export function respondAskUser(request: RespondAskUserRequest) {
  return ipcInvoke<void>(IPC_COMMANDS.respondAskUser, { request });
}

export function respondPathPermission(request: RespondPathPermissionRequest) {
  return ipcInvoke<void>(IPC_COMMANDS.respondPathPermission, { request });
}

export function respondToolApproval(request: RespondToolApprovalRequest) {
  return ipcInvoke<void>(IPC_COMMANDS.respondToolApproval, { request });
}

export function setPlanMode(
  sessionId: string,
  active: boolean,
  source: "auto" | "manual" = "manual",
) {
  return ipcInvoke<void>(IPC_COMMANDS.setPlanMode, {
    request: { sessionId, active, source },
  });
}

export function getPlanMode(sessionId: string) {
  return ipcInvoke<boolean>(IPC_COMMANDS.getPlanMode, {
    request: { sessionId },
  });
}

export function listCheckpoints(sessionId: string) {
  return ipcInvoke<CheckpointInfo[]>(IPC_COMMANDS.listCheckpoints, {
    request: { sessionId },
  });
}

export function rewindSession(request: RewindSessionRequest) {
  return ipcInvoke<RewindSessionResponse>(IPC_COMMANDS.rewindSession, {
    request,
  });
}

export type {
  AppInfo,
  AppSettings,
  AppSettingsPatch,
  ChatCancelRequest,
  ChatDeltaEvent,
  ChatErrorEvent,
  ChatFinishedEvent,
  ChatHistoryRequest,
  ChatHistoryResponse,
  ChatModelInfo,
  ChatReasoningEvent,
  ChatSendRequest,
  ChatSendResponse,
  ChatStartedEvent,
  ChatStatusEvent,
  ChatUserContentEvent,
};
