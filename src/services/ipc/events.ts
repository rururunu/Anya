import type { UnlistenFn } from "@tauri-apps/api/event";
import { listen } from "@tauri-apps/api/event";

import type {
  AgentEventRecord,
  AgentDebugEvent,
  AskUserEvent,
  ChatContextNoticeEvent,
  ChatDeltaEvent,
  ChatErrorEvent,
  ChatFinishedEvent,
  ChatReasoningEvent,
  ChatSessionTitleUpdatedEvent,
  ChatStartedEvent,
  ChatStatusEvent,
  ChatTokenUsageEvent,
  ChatUserContentEvent,
  PathPermissionEvent,
  InteractionResolvedEvent,
  PlanModeChangedEvent,
  TaskListUpdatedEvent,
  ToolActivityEvent,
  ToolApprovalEvent,
  FileOfferEvent,
  UrlOfferEvent,
} from "@/types/chat";
import { IPC_EVENTS, type IpcEvent } from "@/types/ipc";
import type { AppSettings } from "@/types/setting";

export function listenIpcEvent<TPayload>(
  event: IpcEvent,
  handler: (payload: TPayload) => void,
): Promise<UnlistenFn> {
  return listen<TPayload>(event, (event) => {
    handler(event.payload);
  });
}

export function listenSettingsChanged(handler: (settings: AppSettings) => void) {
  return listenIpcEvent<AppSettings>(IPC_EVENTS.settingsChanged, handler);
}

export function listenAgentEvent(handler: (payload: AgentEventRecord) => void) {
  return listenIpcEvent<AgentEventRecord>(IPC_EVENTS.agentEvent, handler);
}

export function listenAgentDebugEvent(handler: (payload: AgentDebugEvent) => void) {
  return listenIpcEvent<AgentDebugEvent>(IPC_EVENTS.agentDebugEvent, handler);
}

export function listenChatStarted(handler: (payload: ChatStartedEvent) => void) {
  return listenIpcEvent<ChatStartedEvent>(IPC_EVENTS.chatStarted, handler);
}

export function listenChatDelta(handler: (payload: ChatDeltaEvent) => void) {
  return listenIpcEvent<ChatDeltaEvent>(IPC_EVENTS.chatDelta, handler);
}

export function listenChatReasoning(handler: (payload: ChatReasoningEvent) => void) {
  return listenIpcEvent<ChatReasoningEvent>(IPC_EVENTS.chatReasoning, handler);
}

export function listenChatStatus(handler: (payload: ChatStatusEvent) => void) {
  return listenIpcEvent<ChatStatusEvent>(IPC_EVENTS.chatStatus, handler);
}

export function listenChatUserContent(handler: (payload: ChatUserContentEvent) => void) {
  return listenIpcEvent<ChatUserContentEvent>(IPC_EVENTS.chatUserContent, handler);
}

export function listenChatFinished(handler: (payload: ChatFinishedEvent) => void) {
  return listenIpcEvent<ChatFinishedEvent>(IPC_EVENTS.chatFinished, handler);
}

export function listenChatTokenUsage(handler: (payload: ChatTokenUsageEvent) => void) {
  return listenIpcEvent<ChatTokenUsageEvent>(IPC_EVENTS.chatTokenUsage, handler);
}

export function listenChatSessionTitleUpdated(
  handler: (payload: ChatSessionTitleUpdatedEvent) => void,
) {
  return listenIpcEvent<ChatSessionTitleUpdatedEvent>(IPC_EVENTS.chatSessionTitleUpdated, handler);
}

export function listenChatError(handler: (payload: ChatErrorEvent) => void) {
  return listenIpcEvent<ChatErrorEvent>(IPC_EVENTS.chatError, handler);
}

export function listenChatContextNotice(handler: (payload: ChatContextNoticeEvent) => void) {
  return listenIpcEvent<ChatContextNoticeEvent>(IPC_EVENTS.chatContextNotice, handler);
}

export function listenOverlayShown(handler: () => void) {
  return listenIpcEvent(IPC_EVENTS.overlayShown, handler);
}

export function listenOverlayHidden(handler: () => void) {
  return listenIpcEvent(IPC_EVENTS.overlayHidden, handler);
}

export function listenAskUser(handler: (payload: AskUserEvent) => void) {
  return listenIpcEvent<AskUserEvent>(IPC_EVENTS.askUser, handler);
}

export function listenPathPermission(handler: (payload: PathPermissionEvent) => void) {
  return listenIpcEvent<PathPermissionEvent>(IPC_EVENTS.pathPermission, handler);
}

export function listenToolApproval(handler: (payload: ToolApprovalEvent) => void) {
  return listenIpcEvent<ToolApprovalEvent>(IPC_EVENTS.toolApproval, handler);
}

export function listenInteractionResolved(handler: (payload: InteractionResolvedEvent) => void) {
  return listenIpcEvent<InteractionResolvedEvent>(IPC_EVENTS.interactionResolved, handler);
}

export function listenPlanModeChanged(handler: (payload: PlanModeChangedEvent) => void) {
  return listenIpcEvent<PlanModeChangedEvent>(IPC_EVENTS.planModeChanged, handler);
}

export function listenToolStarted(handler: (payload: ToolActivityEvent) => void) {
  return listenIpcEvent<ToolActivityEvent>(IPC_EVENTS.toolStarted, handler);
}

export function listenToolFinished(handler: (payload: ToolActivityEvent) => void) {
  return listenIpcEvent<ToolActivityEvent>(IPC_EVENTS.toolFinished, handler);
}

export function listenTaskListUpdated(handler: (payload: TaskListUpdatedEvent) => void) {
  return listenIpcEvent<TaskListUpdatedEvent>(IPC_EVENTS.taskListUpdated, handler);
}

export function listenFileOffer(handler: (payload: FileOfferEvent) => void) {
  return listenIpcEvent<FileOfferEvent>(IPC_EVENTS.fileOffer, handler);
}

export function listenUrlOffer(handler: (payload: UrlOfferEvent) => void) {
  return listenIpcEvent<UrlOfferEvent>(IPC_EVENTS.urlOffer, handler);
}
