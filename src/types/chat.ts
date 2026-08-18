export type Role = "system" | "user" | "assistant" | "tool";

export type MessageStatus = "pending" | "streaming" | "done" | "error" | "cancelled";

export type AgentState =
  | "created"
  | "contextLoading"
  | "planning"
  | "executing"
  | "waitingTool"
  | "observing"
  | "reflecting"
  | "completed"
  | "failed"
  | "cancelled";

export type AgentPlanStepStatus = "planned" | "running" | "completed" | "failed";

export interface AgentPlanStep {
  id: string;
  action: string;
  tool?: string;
  description: string;
  status: AgentPlanStepStatus;
}

export interface AgentPlan {
  steps: AgentPlanStep[];
}

export type AgentEvent =
  | { type: "userMessage"; data: { input: string } }
  | { type: "stateChanged"; data: { from: AgentState; to: AgentState } }
  | {
      type: "contextCollected";
      data: { hasWorkspace: boolean; hasActiveFile: boolean; ide?: string };
    }
  | { type: "planCreated"; data: { plan: AgentPlan } }
  | {
      type: "toolCalled";
      data: { callId: string; tool: string; description: string };
    }
  | {
      type: "toolResult";
      data: { callId: string; tool: string; success: boolean; result: string };
    }
  | { type: "fileChanged"; data: { path: string } }
  | { type: "error"; data: { message: string } }
  | { type: "completed" };

export interface AgentEventRecord {
  runId: string;
  sequence: number;
  timestampMs: number;
  event: AgentEvent;
}

export type TokenAccuracy = "exact" | "mixed" | "estimated";

export interface TokenUsage {
  inputTokens: number;
  outputTokens: number;
  systemTokens: number;
  contextTokens: number;
  toolCallTokens: number;
  toolResultTokens: number;
  memoryTokens: number;
  totalTokens: number;
  accuracy: TokenAccuracy;
  source?: string;
}

export type AgentDebugEvent =
  | { type: "runCreated"; data: { runId: string; state: AgentState } }
  | {
      type: "contextSnapshot";
      data: { runId: string; context: CapturedContext };
    }
  | {
      type: "tokenUsage";
      data: { runId: string; model: string; usage: TokenUsage };
    }
  | { type: "runtimeEvent"; data: { record: AgentEventRecord } }
  | {
      type: "toolCall";
      data: {
        runId: string;
        callId: string;
        tool: string;
        description: string;
        arguments: Record<string, unknown>;
      };
    }
  | {
      type: "subagentStarted";
      data: {
        runId: string;
        subagentId: string;
        parentSubagentId?: string;
        description: string;
        readOnly: boolean;
        depth: number;
        timestampMs: number;
      };
    }
  | {
      type: "subagentProgress";
      data: {
        runId: string;
        subagentId: string;
        kind: string;
        content: string;
        timestampMs: number;
      };
    }
  | {
      type: "subagentToolCall";
      data: {
        runId: string;
        subagentId: string;
        callId: string;
        tool: string;
        description: string;
        arguments: Record<string, unknown>;
        timestampMs: number;
      };
    }
  | {
      type: "subagentToolResult";
      data: {
        runId: string;
        subagentId: string;
        callId: string;
        tool: string;
        success: boolean;
        result: string;
        timestampMs: number;
      };
    }
  | {
      type: "subagentFinished";
      data: {
        runId: string;
        subagentId: string;
        success: boolean;
        summary: string;
        timestampMs: number;
      };
    };

export interface AskUserAnswerItem {
  header?: string;
  selected: string[];
  userSupplement?: boolean;
}

/** 与 Rust Runtime `ChatMessage` 对齐 */
export interface ChatMessage {
  id: string;
  sessionId: string;
  role: Role;
  content: string;
  reasoning?: string;
  workTimeline?: WorkTimelineItem[];
  toolActivities?: ToolActivity[];
  askUserAnswer?: AskUserAnswerItem[];
  /** Ephemeral UI status from backend (not persisted). e.g. "analyzing_images" */
  activityStatus?: string;
  /** Soft-inject into an in-flight assistant turn (not a new unanswered user turn). */
  injected?: boolean;
  /** UI-only structured payload used by the local /context diagnostic message. */
  environmentContext?: CapturedContext;
  status: MessageStatus;
  timestamp: number;
  /** Cached estimate for completed persisted messages. */
  estimatedTokens?: number;
  /** UI-side completion time used to freeze the processing duration. */
  completedAt?: number;
  /** UI-only: files the agent offered this turn. */
  sharedFiles?: SharedFileOffer[];
  /** UI-only: proxied preview URLs the agent offered this turn. */
  sharedUrls?: SharedUrlOffer[];
}

export interface SharedFileOffer {
  offerId: string;
  path: string;
  absolutePath?: string;
  name: string;
  mime: string;
  size: number;
  workspaceId?: string;
}

export interface SharedUrlOffer {
  offerId: string;
  label: string;
  originUrl: string;
  publicUrl: string;
}

export interface FileOfferEvent {
  sessionId?: string;
  session_id?: string;
  offerId?: string;
  offer_id?: string;
  path: string;
  absolutePath?: string;
  absolute_path?: string;
  name: string;
  mime?: string;
  size?: number;
  workspaceId?: string;
  workspace_id?: string;
}

export interface UrlOfferEvent {
  sessionId?: string;
  session_id?: string;
  offerId?: string;
  offer_id?: string;
  label?: string;
  originUrl?: string;
  origin_url?: string;
  publicUrl?: string;
  public_url?: string;
}

export type WorkTimelineItem =
  | { type: "reasoning"; id: string; content: string }
  | { type: "content"; id: string; content: string }
  | { type: "tool"; id: string; toolActivityId: string };

export interface ChatSendRequest {
  message: string;
  sessionId?: string;
  workspaceId?: string;
  quickAsk?: boolean;
  /** Per-conversation overrides; absent values fall back to global settings. */
  modelId?: string;
  modelProvider?: string;
  chatMode?: "ask" | "agent" | "plan";
  toolApprovalMode?: "ask" | "auto" | "alwaysAllow";
  /** Skip complexity auto-plan (used after Approve & execute). */
  skipAutoPlan?: boolean;
  /** Approve & execute continuation: drives the turn but never persists a
   * user bubble / history entry for the approval message. */
  resumePlan?: boolean;
}

export interface OfficeContext {
  app: string;
  isForeground: boolean;
  documentPath?: string;
  documentName?: string;
  selectedText?: string;
  selectionStart?: number;
  selectionEnd?: number;
  documentTitle?: string;
  pageCount?: number;
  activeSheet?: string;
  cellAddress?: string;
  slideIndex?: number;
  slideCount?: number;
  trackChangesEnabled?: boolean;
  pendingRevisions?: number;
}

/** 与 Rust `RequestContext` 对齐 — overlay 唤起时采集的上下文 */
export interface CapturedContext {
  selection?: string;
  selectedFiles?: string[];
  selectedImages?: string[];
  activeWindow?: string;
  activeFile?: string;
  workspace?: { name: string; root: string };
  clipboard?: string;
  gitStatus?: string;
  lastShellExecution?: string;
  ideContext?: IDEContext;
  officeContext?: OfficeContext;
}

export interface CursorPosition {
  line: number;
  column: number;
}

export interface IDEContext {
  ide: string;
  activeFile?: string;
  workspace?: string;
  language?: string;
  selection?: string;
  cursor?: CursorPosition;
}

export interface ChatSendResponse {
  sessionId: string;
  userMessageId: string;
  assistantMessageId: string;
  agentRunId?: string;
}

export interface ChatCancelRequest {
  messageId: string;
}

export interface ChatStartedEvent {
  sessionId: string;
  userMessage: ChatMessage;
  assistantMessage: ChatMessage;
}

export interface ChatDeltaEvent {
  sessionId: string;
  messageId: string;
  delta: string;
}

export interface ChatReasoningEvent {
  sessionId: string;
  messageId: string;
  content: string;
}

export interface ChatStatusEvent {
  sessionId: string;
  messageId: string;
  kind: string;
}

export interface ChatUserContentEvent {
  sessionId: string;
  messageId: string;
  content: string;
}

export interface ChatFinishedEvent {
  sessionId: string;
  messageId: string;
  content: string;
  reasoning?: string;
  finishReason?: string;
}

export interface ChatSessionTitleUpdatedEvent {
  sessionId: string;
  title: string;
}

export interface ChatErrorEvent {
  sessionId: string;
  messageId: string;
  message: string;
}

export interface ChatContextNoticeEvent {
  sessionId: string;
  kind: "approaching-limit" | "compacted" | string;
  message: string;
  usageRatio: number;
  foldedMessages?: number;
  estimatedTokens?: number;
  contextWindowTokens?: number;
}

export interface ContextUsageSnapshot {
  usageRatio: number;
  estimatedTokens: number;
  contextWindowTokens: number;
}

export interface ContextUsageRequest {
  sessionId?: string;
  draftMessage?: string;
  context?: CapturedContext;
  /** Active chat model — caps the context window (1M toggle ≠ every model). */
  modelId?: string;
}

export interface ContextUsageResponse {
  usageRatio: number;
  estimatedTokens: number;
  contextWindowTokens: number;
}

export interface ChatHistoryRequest {
  sessionId?: string;
}

export interface ChatHistoryResponse {
  sessionId: string;
  messages: ChatMessage[];
}

export interface ChatSessionSummary {
  sessionId: string;
  workspaceId?: string;
  preview: string;
  messageCount: number;
  turnCount: number;
  estimatedTokens: number;
  updatedAt: number;
}

export interface ListChatSessionsResponse {
  sessions: ChatSessionSummary[];
}

export interface ModelThinkingVariant {
  id: string;
  label: string;
  recommended?: boolean;
}

export interface ChatModelInfo {
  id: string;
  ownedBy: string;
  /** Stable provider key for UI icons (e.g. `"deepseek"`). */
  provider: string;
  /** Human-readable label for pickers (e.g. Gemini 3.1 Pro High). */
  displayName?: string;
  /** Alternate thinking tiers for the same model family (High / Low / Agent). */
  thinkingVariants?: ModelThinkingVariant[];
}

export interface AskUserOption {
  label: string;
  description?: string;
}

export interface AskUserQuestion {
  header: string;
  question: string;
  options: AskUserOption[];
  multiSelect?: boolean;
}

/** UI-only display shape for a rendered AskUser option row (adds slug/skip for picker use). */
export type AskDisplayOption = {
  label: string;
  slug: string;
  description?: string;
  isSkip?: boolean;
};

export interface AskUserEvent {
  sessionId: string;
  requestId: string;
  questions: AskUserQuestion[];
}

export interface RespondAskUserRequest {
  requestId: string;
  answer: string;
}

export interface PathPermissionEvent {
  sessionId: string;
  requestId: string;
  path: string;
  operation: "read" | "write" | string;
  toolName: string;
}

export interface RespondPathPermissionRequest {
  requestId: string;
  decision: PathPermissionDecision;
}

export type PathPermissionDecision = "allow_once" | "allow_always" | "deny";

export type ToolApprovalDecision = "allow_once" | "allow_session" | "deny";

export interface ToolPreviewPayload {
  path: string;
  affectedPaths?: string[];
  kind: string;
  oldText?: string | null;
  newText?: string | null;
  unifiedDiff: string;
}

export interface ToolApprovalEvent {
  sessionId: string;
  requestId: string;
  toolName: string;
  title: string;
  arguments?: Record<string, unknown>;
  preview?: ToolPreviewPayload | null;
}

export interface ToolApprovalSession {
  requestId: string;
  toolName: string;
  title: string;
  preview?: ToolPreviewPayload | null;
}

export interface RespondToolApprovalRequest {
  requestId: string;
  decision: ToolApprovalDecision;
}

export interface InteractionResolvedEvent {
  requestId: string;
  kind: "ask_user" | "path_permission" | "tool_approval";
}

export interface PlanModeChangedEvent {
  sessionId: string;
  active: boolean;
  /** How the plan was entered: "auto" (agent complexity detection) or "manual". */
  source?: "auto" | "manual";
}

export interface CheckpointInfo {
  turn: number;
  time: number;
  prompt: string;
  files: Array<{ path: string; content?: string | null }>;
  userMessageId?: string | null;
}

export type RewindRestoreMode = "code" | "conversation" | "both";

export interface RewindSessionRequest {
  sessionId: string;
  turn: number;
  restore: RewindRestoreMode;
}

export interface RewindSessionResponse {
  restoredFiles: number;
  truncatedMessages: boolean;
}

export interface ToolActivityEvent {
  sessionId: string;
  messageId: string;
  activityId: string;
  subagentId?: string;
  parentActivityId?: string;
  toolName: string;
  title: string;
  kind: string;
  detail?: string;
  arguments?: Record<string, unknown>;
  result?: string;
  preview?: ToolPreviewPayload | null;
  success?: boolean;
  status: "running" | "done" | "error" | string;
}

export interface ToolActivity {
  id: string;
  /** Child agent identity, when this tool was executed by a sub-agent. */
  subagentId?: string;
  /** Parent run_subagent activity used to render child work as one execution card. */
  parentActivityId?: string;
  toolName: string;
  title: string;
  kind: string;
  detail?: string;
  arguments?: Record<string, unknown>;
  result?: string;
  success: boolean;
  status: "running" | "done" | "error";
  /** Pre-execution preview shown in the chat card while waiting for approval. */
  preview?: ToolPreviewPayload | null;
}

export interface TaskItem {
  content: string;
  status: string;
  activeForm?: string;
  level?: number;
}

export interface TaskListUpdatedEvent {
  sessionId: string;
  tasks: TaskItem[];
}

/** @deprecated 使用 ChatMessage */
export type Message = ChatMessage;
