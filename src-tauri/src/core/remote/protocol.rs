use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: i32 = 1;
pub const DEFAULT_PORT: u16 = 8787;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "hello")]
    Hello {
        #[serde(rename = "protocolVersion", default = "default_protocol_version")]
        protocol_version: i32,
        #[serde(rename = "deviceId")]
        device_id: String,
        credential: String,
        /// Part of the wire contract; accepted but not consumed yet.
        #[serde(rename = "appVersion", default)]
        #[allow(dead_code)]
        app_version: String,
    },
    #[serde(rename = "session.list")]
    SessionList {
        #[serde(rename = "requestId")]
        request_id: String,
    },
    #[serde(rename = "session.history")]
    SessionHistory {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    #[serde(rename = "session.delete")]
    SessionDelete {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    #[serde(rename = "chat.send")]
    ChatSend {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "sessionId")]
        session_id: Option<String>,
        message: String,
        #[serde(rename = "workspaceId")]
        workspace_id: Option<String>,
        #[serde(rename = "chatMode", default)]
        chat_mode: Option<String>,
        #[serde(rename = "toolApprovalMode", default)]
        tool_approval_mode: Option<String>,
        #[serde(rename = "chatModel", default)]
        chat_model: Option<String>,
        #[serde(rename = "chatModelProvider", default)]
        chat_model_provider: Option<String>,
    },
    #[serde(rename = "session.compose.get")]
    SessionComposeGet {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    #[serde(rename = "session.compose.set")]
    SessionComposeSet {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "chatMode", default)]
        chat_mode: Option<String>,
        #[serde(rename = "toolApprovalMode", default)]
        tool_approval_mode: Option<String>,
        #[serde(rename = "chatModel", default)]
        chat_model: Option<String>,
        #[serde(rename = "chatModelProvider", default)]
        chat_model_provider: Option<String>,
        #[serde(rename = "chatModelLabel", default)]
        chat_model_label: Option<String>,
        #[serde(rename = "reasoningEffort", default)]
        reasoning_effort: Option<String>,
    },
    #[serde(rename = "context.usage")]
    ContextUsage {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "sessionId", default)]
        session_id: Option<String>,
        #[serde(rename = "draftMessage", default)]
        draft_message: Option<String>,
        #[serde(rename = "modelId", default)]
        model_id: Option<String>,
    },
    #[serde(rename = "models.list")]
    ModelsList {
        #[serde(rename = "requestId")]
        request_id: String,
    },
    #[serde(rename = "plan.approve")]
    PlanApprove {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    #[serde(rename = "chat.cancel")]
    ChatCancel {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "messageId")]
        message_id: String,
    },
    #[serde(rename = "approval.respond")]
    ApprovalRespond {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "approvalRequestId")]
        approval_request_id: String,
        decision: String,
    },
    #[serde(rename = "ask.respond")]
    AskRespond {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "askRequestId")]
        ask_request_id: String,
        answer: String,
    },
    /// App-level keep-alive reply to server `ping` (proxies often drop WS pings).
    #[serde(rename = "pong")]
    Pong {
        /// Echoed timestamp from the wire contract; nothing reads it.
        #[allow(dead_code)]
        ts: i64,
    },
    #[serde(rename = "workspace.snapshot")]
    WorkspaceSnapshot {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "sessionId")]
        session_id: Option<String>,
    },
    #[serde(rename = "workspace.readFile")]
    WorkspaceReadFile {
        #[serde(rename = "requestId")]
        request_id: String,
        path: String,
        #[serde(rename = "maxBytes", default = "default_max_bytes")]
        max_bytes: i32,
        #[serde(rename = "sessionId", default)]
        session_id: Option<String>,
        #[serde(rename = "workspaceId", default)]
        workspace_id: Option<String>,
        /// "text" (default) returns UTF-8 content; "download" returns one base64 slice.
        #[serde(default)]
        mode: Option<String>,
        /// Download-mode byte offset. Defaults to 0; each RPC returns one slice.
        #[serde(default)]
        offset: Option<u64>,
        /// Requested slice length in bytes; server caps this at 512KB.
        #[serde(default)]
        length: Option<u64>,
    },
    #[serde(rename = "workspace.files")]
    WorkspaceFiles {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "sessionId", default)]
        session_id: Option<String>,
        #[serde(rename = "workspaceId", default)]
        workspace_id: Option<String>,
    },
    #[serde(rename = "skills.list")]
    SkillsList {
        #[serde(rename = "requestId")]
        request_id: String,
    },
    #[serde(rename = "mcp.list")]
    McpList {
        #[serde(rename = "requestId")]
        request_id: String,
    },
    #[serde(rename = "file.upload.begin")]
    FileUploadBegin {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "sessionId", default)]
        session_id: Option<String>,
        #[serde(rename = "workspaceId", default)]
        workspace_id: Option<String>,
        #[serde(rename = "fileName")]
        file_name: String,
        size: u64,
        #[serde(default)]
        #[allow(dead_code)]
        mime: Option<String>,
    },
    #[serde(rename = "file.upload.chunk")]
    FileUploadChunk {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "uploadId")]
        upload_id: String,
        offset: u64,
        #[serde(rename = "dataBase64")]
        data_base64: String,
    },
    #[serde(rename = "file.upload.finish")]
    FileUploadFinish {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "uploadId")]
        upload_id: String,
    },
    #[serde(rename = "file.upload.abort")]
    FileUploadAbort {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "uploadId")]
        upload_id: String,
    },
    #[serde(rename = "file.download.begin")]
    FileDownloadBegin {
        #[serde(rename = "requestId")]
        request_id: String,
        path: String,
        #[serde(rename = "sessionId", default)]
        session_id: Option<String>,
        #[serde(rename = "workspaceId", default)]
        workspace_id: Option<String>,
    },
}

fn default_protocol_version() -> i32 {
    PROTOCOL_VERSION
}

fn default_max_bytes() -> i32 {
    200_000
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "hello.ok")]
    HelloOk {
        #[serde(rename = "protocolVersion")]
        protocol_version: i32,
        #[serde(rename = "serverName")]
        server_name: String,
        #[serde(rename = "serverVersion", skip_serializing_if = "Option::is_none")]
        server_version: Option<String>,
    },
    #[serde(rename = "hello.error")]
    HelloError { code: String, message: String },
    #[serde(rename = "event")]
    Event {
        name: String,
        data: serde_json::Map<String, Value>,
    },
    #[serde(rename = "rpc.result")]
    RpcResult {
        #[serde(rename = "requestId")]
        request_id: String,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    #[serde(rename = "ping")]
    Ping { ts: i64 },
}

impl ServerMessage {
    pub fn hello_ok(version: Option<String>) -> Self {
        Self::HelloOk {
            protocol_version: PROTOCOL_VERSION,
            server_name: "Anya".into(),
            server_version: version,
        }
    }

    pub fn hello_error(code: &str, message: impl Into<String>) -> Self {
        Self::HelloError {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn rpc_ok(request_id: impl Into<String>, data: Value) -> Self {
        Self::RpcResult {
            request_id: request_id.into(),
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn rpc_err(request_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::RpcResult {
            request_id: request_id.into(),
            ok: false,
            data: None,
            error: Some(error.into()),
        }
    }
}
