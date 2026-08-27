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
  ChatSessionSummary,
} from "@/types/chat";
import { IPC_COMMANDS } from "@/types/ipc";
import type {
  AppSettings,
  AppSettingsPatch,
  GeminiAuthStatus,
  SemanticSearchConfig,
  SemanticSearchState,
} from "@/types/setting";
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
  requestId?: string;
  title: string;
  body: string;
  ignoreLabel: string;
  openLabel: string;
  persistent?: boolean;
}) {
  return ipcInvoke<void>(IPC_COMMANDS.showInteractionNotification, { request });
}

export function dismissInteractionNotification(request: {
  sessionId?: string;
  requestId?: string;
}) {
  return ipcInvoke<void>(IPC_COMMANDS.dismissInteractionNotification, { request });
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

export function getSemanticSearchStatus() {
  return ipcInvoke<SemanticSearchState>("get_semantic_search_status");
}

export function setSemanticSearch(config: SemanticSearchConfig) {
  return ipcInvoke<SemanticSearchState>("set_semantic_search", { config });
}

export function testSemanticSearchApi(baseUrl: string, apiKey: string, model: string) {
  return ipcInvoke<{ ok: boolean; dim: number }>("test_semantic_search_api", {
    baseUrl,
    apiKey,
    model,
  });
}

export function fetchSemanticSearchModels(baseUrl: string, apiKey: string) {
  return ipcInvoke<string[]>("fetch_semantic_search_models", { baseUrl, apiKey });
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

export function webviewGpuDisabled() {
  return ipcInvoke<boolean>(IPC_COMMANDS.webviewGpuDisabled);
}

export function relaunchApp() {
  return ipcInvoke<void>(IPC_COMMANDS.relaunchApp);
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

export function listArchivedChatSessions() {
  return ipcInvoke<ListChatSessionsResponse>(IPC_COMMANDS.listArchivedChatSessions);
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

export function branchChatSession(sessionId: string, messageId?: string) {
  return ipcInvoke<ChatSessionSummary>(IPC_COMMANDS.branchChatSession, {
    sessionId,
    session_id: sessionId,
    messageId,
    message_id: messageId,
  });
}

export function setChatSessionArchived(sessionId: string, archived: boolean) {
  return ipcInvoke<void>("set_chat_session_archived", { sessionId, archived });
}

export function setChatSessionWorkspace(sessionId: string, workspaceId: string) {
  return ipcInvoke<void>("set_chat_session_workspace", { sessionId, workspaceId });
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

export function revealInExplorer(path: string) {
  return ipcInvoke<void>(IPC_COMMANDS.revealInExplorer, { path });
}

export function openInDefaultApp(path: string) {
  return ipcInvoke<void>(IPC_COMMANDS.openInDefaultApp, { path });
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
