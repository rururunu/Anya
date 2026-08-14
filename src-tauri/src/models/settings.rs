use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ColorScheme {
    #[serde(
        rename = "dark",
        alias = "system",
        alias = "auto",
        alias = "default",
        alias = "nocturne",
        alias = "blue-black",
        alias = "dark",
        alias = "midnight",
        alias = "forest",
        alias = "rose",
        alias = "ocean",
        alias = "graphite",
        alias = "ember",
        alias = "teal",
        alias = "ghost-pastel"
    )]
    Dark,
    #[serde(rename = "light", alias = "paper", alias = "cream", alias = "frost")]
    Light,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AppLanguage {
    #[default]
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
    #[serde(rename = "ja-JP")]
    JaJp,
    #[serde(rename = "ru-RU")]
    RuRu,
    #[serde(rename = "de-DE")]
    DeDe,
    #[serde(rename = "fr-FR")]
    FrFr,
    #[serde(rename = "ko-KR")]
    KoKr,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    #[default]
    Disabled,
    High,
    Max,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningLanguage {
    #[default]
    Auto,
    Zh,
    En,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum WebSearchProvider {
    #[default]
    Serper,
    Tavily,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ToolApprovalMode {
    #[default]
    Ask,
    Auto,
    AlwaysAllow,
}

/// How agent tool cards (shell / diffs) are shown in the chat timeline.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentWorkDisplay {
    #[default]
    Detailed,
    Compact,
}

/// Chat interaction mode: Agent can mutate; Ask is read-only; Plan drafts
/// first and blocks writers until the user approves.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ChatMode {
    #[default]
    Agent,
    Ask,
    Plan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LspServerConfig {
    pub id: String,
    pub languages: Vec<String>,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<(String, String)>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Remote icon URL captured at install time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    /// Stable registry identity (e.g. Smithery qualifiedName).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qualified_name: Option<String>,
    /// Upstream registry record id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// `smithery` | `catalog` | `manual`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    /// Comma-separated or newline-separated model IDs (stored as raw text).
    pub models: String,
    /// Optional preset template id used for icons / known defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GeminiOAuthSettings {
    #[serde(default = "default_gemini_oauth_client_id")]
    pub client_id: String,
    #[serde(default = "default_gemini_oauth_client_secret")]
    pub client_secret: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    /// Unix timestamp (seconds) when `access_token` expires.
    #[serde(default)]
    pub expires_at: i64,
    #[serde(default)]
    pub email: String,
    /// Cloud Code companion project from `loadCodeAssist`.
    #[serde(default)]
    pub project_id: String,
}

impl Default for GeminiOAuthSettings {
    fn default() -> Self {
        Self {
            client_id: default_gemini_oauth_client_id(),
            client_secret: default_gemini_oauth_client_secret(),
            access_token: String::new(),
            refresh_token: String::new(),
            expires_at: 0,
            email: String::new(),
            project_id: String::new(),
        }
    }
}

impl GeminiOAuthSettings {
    pub fn is_logged_in(&self) -> bool {
        !self.access_token.trim().is_empty() || !self.refresh_token.trim().is_empty()
    }
}

fn default_gemini_oauth_client_id() -> String {
    String::new()
}

fn default_gemini_oauth_client_secret() -> String {
    String::new()
}

/// Local embedding model used for optional semantic workspace search.
/// The model is downloaded lazily on first enable (never bundled).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SemanticSearchModel {
    #[default]
    #[serde(rename = "multilingual-e5-small")]
    MultilingualE5Small,
    #[serde(rename = "bge-small-zh-v1.5")]
    BGESmallZHV15,
    #[serde(rename = "bge-small-en-v1.5")]
    BGESmallENV15,
    #[serde(rename = "jina-embeddings-v2-base-code")]
    JinaEmbeddingsV2BaseCode,
    #[serde(rename = "bge-m3")]
    BGEM3,
}

/// Embedding backend for semantic workspace search.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SemanticSearchBackend {
    /// OpenAI-compatible `/embeddings` endpoint (SiliconFlow / OpenAI / local).
    #[default]
    Api,
    /// Local ONNX model downloaded lazily via fastembed.
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub color_scheme: ColorScheme,
    pub language: AppLanguage,
    #[serde(default)]
    pub deepseek_api_key: String,
    #[serde(default)]
    pub gemini_oauth: GeminiOAuthSettings,
    #[serde(default = "default_memory_enabled")]
    pub memory_enabled: bool,
    #[serde(default)]
    pub mem0_api_key: String,
    #[serde(default = "default_mem0_user_id")]
    pub mem0_user_id: String,
    #[serde(default = "default_mem0_base_url")]
    pub mem0_base_url: String,
    #[serde(default)]
    pub web_search_enabled: bool,
    #[serde(default)]
    pub web_search_provider: WebSearchProvider,
    #[serde(default)]
    pub serper_api_key: String,
    #[serde(default)]
    pub tavily_api_key: String,
    #[serde(default)]
    pub tool_approval_mode: ToolApprovalMode,
    #[serde(default)]
    pub chat_mode: ChatMode,
    #[serde(default)]
    pub lsp_enabled: bool,
    #[serde(default)]
    pub lsp_servers: Vec<LspServerConfig>,
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
    /// Smithery API key — preferred auth for hosted Smithery MCP (avoids broken local OAuth).
    #[serde(default)]
    pub smithery_api_key: String,
    /// Built-in skill names that are enabled for the agent (`load_skill` / shortcut tools).
    /// Empty by default — users opt in from Settings → Skills → Built-in.
    #[serde(default)]
    pub enabled_builtin_skills: Vec<String>,
    #[serde(default = "default_opacity")]
    pub opacity: u32,
    /// Frosted-glass titlebar and sidebars on the workbench.
    #[serde(default)]
    pub chrome_frosted_glass: bool,
    #[serde(default = "default_chat_model")]
    pub chat_model: String,
    #[serde(default)]
    pub chat_model_provider: String,
    #[serde(default = "default_multimodal_model")]
    pub multimodal_model: String,
    #[serde(default)]
    pub multimodal_model_provider: String,
    #[serde(default)]
    pub reasoning_effort: ReasoningEffort,
    #[serde(default)]
    pub reasoning_language: ReasoningLanguage,
    /// When true, assistant turns that contain `tool_calls` include prior
    /// reasoning text in later API history (required by thinking + tools).
    #[serde(default = "default_true")]
    pub pass_tool_reasoning: bool,
    /// When true (default), keep the session thinking effort on every
    /// agent-loop round after tools. When false, later rounds skip thinking
    /// to save tokens; history may still carry earlier reasoning blocks.
    #[serde(default = "default_true")]
    pub continue_thinking_after_tools: bool,
    /// Controls whether reasoning supplied by the model is rendered in chat.
    #[serde(default = "default_true")]
    pub show_reasoning: bool,
    /// detailed = shell/diff inline in chat; compact = fold into process details.
    #[serde(default)]
    pub agent_work_display: AgentWorkDisplay,
    /// Allow the main agent to delegate work to user-selected models.
    #[serde(default)]
    pub multi_model_collaboration: bool,
    #[serde(default)]
    pub collaboration_models: Vec<String>,
    /// Inject the optional minimal-coding ladder into each agent turn.
    #[serde(default)]
    pub minimal_coding: bool,
    /// When true, file writes outside the workspace may be approved via path permission.
    /// Default false = hard deny (sandbox).
    #[serde(default)]
    pub allow_outside_workspace_writes: bool,
    /// When true, shell runs under restricted limits (Job Object, scrubbed env, shorter timeout).
    #[serde(default)]
    pub restricted_shell: bool,
    /// Absolute ceiling for one foreground shell command, in seconds (min 5).
    /// Deliberately generous: a command that keeps making progress should be
    /// allowed to finish, so this is a safety net rather than the usual way a
    /// command ends. Stuck commands are caught by `shell_stall_timeout_secs`.
    #[serde(default = "default_shell_timeout_secs")]
    pub shell_timeout_secs: u64,
    /// How long a foreground command may make no progress at all — no new
    /// output and no CPU consumed by its process tree — before it is treated
    /// as stuck, in seconds (min 5).
    #[serde(default = "default_shell_stall_timeout_secs")]
    pub shell_stall_timeout_secs: u64,
    /// Automatically run one lightweight build/test verification pass after
    /// successful file mutations in an agent turn.
    #[serde(default = "default_true")]
    pub auto_verify_after_edits: bool,
    /// One-time hint flag for users migrated from settings without
    /// `restrictedShell`. Frontend can display and clear it.
    #[serde(default)]
    pub pending_restricted_shell_upgrade_notice: bool,
    #[serde(default = "default_true")]
    pub multimodal_split_analysis: bool,
    /// When true, use a 1M-token context window for compaction / turn budgets.
    #[serde(default = "default_true")]
    pub large_context_enabled: bool,
    #[serde(default = "default_zoom")]
    pub zoom: u32,
    /// WebView2 GPU rendering. Off by default for broader driver compatibility; changing requires restart.
    #[serde(default = "default_false")]
    pub hardware_acceleration_enabled: bool,
    /// Primary overlay shortcut modifier, activated by a double tap.
    #[serde(default = "default_primary_hotkey")]
    pub primary_hotkey: String,
    /// When false, double-tap primary shortcut is not listened for globally.
    #[serde(default = "default_true")]
    pub primary_hotkey_enabled: bool,
    /// Secondary overlay shortcut, e.g. `Ctrl+Alt+Space` (recorded in Settings).
    #[serde(default = "default_secondary_hotkey")]
    pub secondary_hotkey: String,
    /// When false, the secondary chord is not listened for globally.
    #[serde(default = "default_true")]
    pub secondary_hotkey_enabled: bool,
    #[serde(default)]
    pub custom_providers: Vec<CustomProviderConfig>,
    /// Show an AI button on PixPin pin windows (bottom-right).
    #[serde(default = "default_true")]
    pub pixpin_pin_ai_enabled: bool,
    /// Show an AI button on Snipaste pin windows (bottom-right).
    #[serde(default = "default_true")]
    pub snipaste_pin_ai_enabled: bool,
    /// Semantic workspace search via embeddings. Off by default; enabling loads
    /// the chosen backend (API or local model).
    #[serde(default)]
    pub semantic_search_enabled: bool,
    #[serde(default)]
    pub semantic_search_backend: SemanticSearchBackend,
    #[serde(default)]
    pub semantic_search_model: SemanticSearchModel,
    #[serde(default)]
    pub semantic_search_api_base_url: String,
    #[serde(default)]
    pub semantic_search_api_key: String,
    #[serde(default)]
    pub semantic_search_api_model: String,
    /// First-run welcome wizard. Missing from older settings files → treat as done.
    #[serde(default = "default_onboarding_completed_existing")]
    pub onboarding_completed: bool,
}

fn default_onboarding_completed_existing() -> bool {
    // Existing installs that predate this field should skip the welcome flow.
    true
}

fn default_chat_model() -> String {
    String::new()
}

fn default_multimodal_model() -> String {
    "gpt-4o".to_string()
}

fn default_zoom() -> u32 {
    100
}

fn default_opacity() -> u32 {
    100
}

fn default_memory_enabled() -> bool {
    true
}
fn default_mem0_user_id() -> String {
    "peek-user".to_string()
}
fn default_mem0_base_url() -> String {
    "https://api.mem0.ai/v1".to_string()
}

fn default_shell_timeout_secs() -> u64 {
    3600
}

fn default_shell_stall_timeout_secs() -> u64 {
    120
}

fn default_secondary_hotkey() -> String {
    crate::services::hotkey::DEFAULT_SECONDARY_HOTKEY.to_string()
}

fn default_primary_hotkey() -> String {
    crate::services::hotkey::DEFAULT_PRIMARY_HOTKEY.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsPatch {
    pub color_scheme: Option<ColorScheme>,
    pub language: Option<AppLanguage>,
    pub deepseek_api_key: Option<String>,
    pub gemini_oauth: Option<GeminiOAuthSettings>,
    pub memory_enabled: Option<bool>,
    pub mem0_api_key: Option<String>,
    pub mem0_user_id: Option<String>,
    pub mem0_base_url: Option<String>,
    pub web_search_enabled: Option<bool>,
    pub web_search_provider: Option<WebSearchProvider>,
    pub serper_api_key: Option<String>,
    pub tavily_api_key: Option<String>,
    pub tool_approval_mode: Option<ToolApprovalMode>,
    pub chat_mode: Option<ChatMode>,
    pub lsp_enabled: Option<bool>,
    pub lsp_servers: Option<Vec<LspServerConfig>>,
    pub mcp_servers: Option<Vec<McpServerConfig>>,
    pub smithery_api_key: Option<String>,
    pub enabled_builtin_skills: Option<Vec<String>>,
    pub opacity: Option<u32>,
    pub chrome_frosted_glass: Option<bool>,
    pub chat_model: Option<String>,
    pub chat_model_provider: Option<String>,
    pub multimodal_model: Option<String>,
    pub multimodal_model_provider: Option<String>,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub reasoning_language: Option<ReasoningLanguage>,
    pub pass_tool_reasoning: Option<bool>,
    pub continue_thinking_after_tools: Option<bool>,
    pub show_reasoning: Option<bool>,
    pub agent_work_display: Option<AgentWorkDisplay>,
    pub multi_model_collaboration: Option<bool>,
    pub collaboration_models: Option<Vec<String>>,
    pub minimal_coding: Option<bool>,
    pub allow_outside_workspace_writes: Option<bool>,
    pub restricted_shell: Option<bool>,
    pub shell_timeout_secs: Option<u64>,
    pub shell_stall_timeout_secs: Option<u64>,
    pub auto_verify_after_edits: Option<bool>,
    pub pending_restricted_shell_upgrade_notice: Option<bool>,
    pub multimodal_split_analysis: Option<bool>,
    pub large_context_enabled: Option<bool>,
    pub zoom: Option<u32>,
    pub hardware_acceleration_enabled: Option<bool>,
    pub primary_hotkey: Option<String>,
    pub primary_hotkey_enabled: Option<bool>,
    pub secondary_hotkey: Option<String>,
    pub secondary_hotkey_enabled: Option<bool>,
    pub custom_providers: Option<Vec<CustomProviderConfig>>,
    pub pixpin_pin_ai_enabled: Option<bool>,
    pub snipaste_pin_ai_enabled: Option<bool>,
    pub semantic_search_enabled: Option<bool>,
    pub semantic_search_backend: Option<SemanticSearchBackend>,
    pub semantic_search_model: Option<SemanticSearchModel>,
    pub semantic_search_api_base_url: Option<String>,
    pub semantic_search_api_key: Option<String>,
    pub semantic_search_api_model: Option<String>,
    pub onboarding_completed: Option<bool>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            color_scheme: ColorScheme::Light,
            language: AppLanguage::ZhCn,
            deepseek_api_key: String::new(),
            gemini_oauth: GeminiOAuthSettings::default(),
            memory_enabled: default_memory_enabled(),
            mem0_api_key: String::new(),
            mem0_user_id: default_mem0_user_id(),
            mem0_base_url: default_mem0_base_url(),
            web_search_enabled: false,
            web_search_provider: WebSearchProvider::default(),
            serper_api_key: String::new(),
            tavily_api_key: String::new(),
            tool_approval_mode: ToolApprovalMode::default(),
            chat_mode: ChatMode::default(),
            lsp_enabled: false,
            lsp_servers: default_lsp_servers(),
            mcp_servers: Vec::new(),
            smithery_api_key: String::new(),
            enabled_builtin_skills: Vec::new(),
            opacity: 100,
            chrome_frosted_glass: false,
            chat_model: default_chat_model(),
            chat_model_provider: String::new(),
            multimodal_model: default_multimodal_model(),
            multimodal_model_provider: String::new(),
            reasoning_effort: ReasoningEffort::default(),
            reasoning_language: ReasoningLanguage::default(),
            pass_tool_reasoning: true,
            continue_thinking_after_tools: true,
            show_reasoning: true,
            agent_work_display: AgentWorkDisplay::default(),
            multi_model_collaboration: false,
            collaboration_models: Vec::new(),
            minimal_coding: false,
            allow_outside_workspace_writes: false,
            restricted_shell: true,
            shell_timeout_secs: default_shell_timeout_secs(),
            shell_stall_timeout_secs: default_shell_stall_timeout_secs(),
            auto_verify_after_edits: true,
            pending_restricted_shell_upgrade_notice: false,
            multimodal_split_analysis: true,
            large_context_enabled: true,
            zoom: 100,
            hardware_acceleration_enabled: false,
            primary_hotkey: default_primary_hotkey(),
            primary_hotkey_enabled: true,
            secondary_hotkey: default_secondary_hotkey(),
            secondary_hotkey_enabled: true,
            custom_providers: Vec::new(),
            pixpin_pin_ai_enabled: true,
            snipaste_pin_ai_enabled: true,
            semantic_search_enabled: false,
            semantic_search_backend: SemanticSearchBackend::default(),
            semantic_search_model: SemanticSearchModel::default(),
            semantic_search_api_base_url: String::new(),
            semantic_search_api_key: String::new(),
            semantic_search_api_model: String::new(),
            onboarding_completed: false,
        }
    }
}

fn default_lsp_servers() -> Vec<LspServerConfig> {
    vec![
        LspServerConfig {
            id: "rust".into(),
            languages: vec!["rust".into()],
            command: "rust-analyzer".into(),
            args: Vec::new(),
            enabled: true,
        },
        LspServerConfig {
            id: "typescript".into(),
            languages: vec![
                "typescript".into(),
                "javascript".into(),
                "tsx".into(),
                "jsx".into(),
            ],
            command: "typescript-language-server".into(),
            args: vec!["--stdio".into()],
            enabled: true,
        },
    ]
}

impl AppSettings {
    pub fn merge(&self, patch: AppSettingsPatch) -> Self {
        Self {
            color_scheme: patch.color_scheme.unwrap_or(self.color_scheme),
            language: patch.language.unwrap_or(self.language),
            deepseek_api_key: patch
                .deepseek_api_key
                .unwrap_or_else(|| self.deepseek_api_key.clone()),
            gemini_oauth: patch
                .gemini_oauth
                .unwrap_or_else(|| self.gemini_oauth.clone()),
            memory_enabled: patch.memory_enabled.unwrap_or(self.memory_enabled),
            mem0_api_key: patch
                .mem0_api_key
                .unwrap_or_else(|| self.mem0_api_key.clone()),
            mem0_user_id: patch
                .mem0_user_id
                .unwrap_or_else(|| self.mem0_user_id.clone()),
            mem0_base_url: patch
                .mem0_base_url
                .unwrap_or_else(|| self.mem0_base_url.clone()),
            web_search_enabled: patch.web_search_enabled.unwrap_or(self.web_search_enabled),
            web_search_provider: patch
                .web_search_provider
                .unwrap_or(self.web_search_provider),
            serper_api_key: patch
                .serper_api_key
                .unwrap_or_else(|| self.serper_api_key.clone()),
            tavily_api_key: patch
                .tavily_api_key
                .unwrap_or_else(|| self.tavily_api_key.clone()),
            tool_approval_mode: patch.tool_approval_mode.unwrap_or(self.tool_approval_mode),
            chat_mode: patch.chat_mode.unwrap_or(self.chat_mode),
            lsp_enabled: patch.lsp_enabled.unwrap_or(self.lsp_enabled),
            lsp_servers: patch
                .lsp_servers
                .unwrap_or_else(|| self.lsp_servers.clone()),
            mcp_servers: patch
                .mcp_servers
                .unwrap_or_else(|| self.mcp_servers.clone()),
            smithery_api_key: patch
                .smithery_api_key
                .unwrap_or_else(|| self.smithery_api_key.clone()),
            enabled_builtin_skills: patch
                .enabled_builtin_skills
                .unwrap_or_else(|| self.enabled_builtin_skills.clone()),
            opacity: patch.opacity.unwrap_or(self.opacity),
            chrome_frosted_glass: patch
                .chrome_frosted_glass
                .unwrap_or(self.chrome_frosted_glass),
            chat_model: patch.chat_model.unwrap_or_else(|| self.chat_model.clone()),
            chat_model_provider: patch
                .chat_model_provider
                .unwrap_or_else(|| self.chat_model_provider.clone()),
            multimodal_model: patch
                .multimodal_model
                .unwrap_or_else(|| self.multimodal_model.clone()),
            multimodal_model_provider: patch
                .multimodal_model_provider
                .unwrap_or_else(|| self.multimodal_model_provider.clone()),
            reasoning_effort: patch.reasoning_effort.unwrap_or(self.reasoning_effort),
            reasoning_language: patch.reasoning_language.unwrap_or(self.reasoning_language),
            pass_tool_reasoning: patch
                .pass_tool_reasoning
                .unwrap_or(self.pass_tool_reasoning),
            continue_thinking_after_tools: patch
                .continue_thinking_after_tools
                .unwrap_or(self.continue_thinking_after_tools),
            show_reasoning: patch.show_reasoning.unwrap_or(self.show_reasoning),
            agent_work_display: patch.agent_work_display.unwrap_or(self.agent_work_display),
            multi_model_collaboration: patch
                .multi_model_collaboration
                .unwrap_or(self.multi_model_collaboration),
            collaboration_models: patch
                .collaboration_models
                .unwrap_or_else(|| self.collaboration_models.clone()),
            minimal_coding: patch.minimal_coding.unwrap_or(self.minimal_coding),
            allow_outside_workspace_writes: patch
                .allow_outside_workspace_writes
                .unwrap_or(self.allow_outside_workspace_writes),
            restricted_shell: patch.restricted_shell.unwrap_or(self.restricted_shell),
            shell_timeout_secs: patch
                .shell_timeout_secs
                .unwrap_or(self.shell_timeout_secs)
                .max(5),
            shell_stall_timeout_secs: patch
                .shell_stall_timeout_secs
                .unwrap_or(self.shell_stall_timeout_secs)
                .max(5),
            auto_verify_after_edits: patch
                .auto_verify_after_edits
                .unwrap_or(self.auto_verify_after_edits),
            pending_restricted_shell_upgrade_notice: patch
                .pending_restricted_shell_upgrade_notice
                .unwrap_or(self.pending_restricted_shell_upgrade_notice),
            multimodal_split_analysis: patch
                .multimodal_split_analysis
                .unwrap_or(self.multimodal_split_analysis),
            large_context_enabled: patch
                .large_context_enabled
                .unwrap_or(self.large_context_enabled),
            zoom: patch.zoom.unwrap_or(self.zoom),
            hardware_acceleration_enabled: patch
                .hardware_acceleration_enabled
                .unwrap_or(self.hardware_acceleration_enabled),
            primary_hotkey: patch
                .primary_hotkey
                .map(|value| crate::services::hotkey::normalize_primary_hotkey(&value))
                .unwrap_or_else(|| self.primary_hotkey.clone()),
            primary_hotkey_enabled: patch
                .primary_hotkey_enabled
                .unwrap_or(self.primary_hotkey_enabled),
            secondary_hotkey: patch
                .secondary_hotkey
                .map(|value| crate::services::hotkey::normalize_hotkey(&value))
                .unwrap_or_else(|| self.secondary_hotkey.clone()),
            secondary_hotkey_enabled: patch
                .secondary_hotkey_enabled
                .unwrap_or(self.secondary_hotkey_enabled),
            custom_providers: patch
                .custom_providers
                .unwrap_or_else(|| self.custom_providers.clone()),
            pixpin_pin_ai_enabled: patch
                .pixpin_pin_ai_enabled
                .unwrap_or(self.pixpin_pin_ai_enabled),
            snipaste_pin_ai_enabled: patch
                .snipaste_pin_ai_enabled
                .unwrap_or(self.snipaste_pin_ai_enabled),
            semantic_search_enabled: patch
                .semantic_search_enabled
                .unwrap_or(self.semantic_search_enabled),
            semantic_search_backend: patch
                .semantic_search_backend
                .unwrap_or(self.semantic_search_backend),
            semantic_search_model: patch
                .semantic_search_model
                .unwrap_or(self.semantic_search_model),
            semantic_search_api_base_url: patch
                .semantic_search_api_base_url
                .unwrap_or_else(|| self.semantic_search_api_base_url.clone()),
            semantic_search_api_key: patch
                .semantic_search_api_key
                .unwrap_or_else(|| self.semantic_search_api_key.clone()),
            semantic_search_api_model: patch
                .semantic_search_api_model
                .unwrap_or_else(|| self.semantic_search_api_model.clone()),
            onboarding_completed: patch
                .onboarding_completed
                .unwrap_or(self.onboarding_completed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, AppSettingsPatch, ColorScheme};

    #[test]
    fn app_default_color_scheme_is_light() {
        assert_eq!(AppSettings::default().color_scheme, ColorScheme::Light);
    }

    #[test]
    fn legacy_settings_default_to_showing_reasoning() {
        let settings: AppSettings = serde_json::from_value(serde_json::json!({
            "colorScheme": "blue-black",
            "language": "zh-CN"
        }))
        .expect("legacy settings should deserialize");

        assert_eq!(settings.color_scheme, ColorScheme::Dark);
        assert!(settings.show_reasoning);
        assert!(settings.continue_thinking_after_tools);
        assert!(!settings.multi_model_collaboration);
        assert!(settings.collaboration_models.is_empty());
        assert!(!settings.minimal_coding);
        assert!(settings.chat_model_provider.is_empty());
        assert!(settings.multimodal_model_provider.is_empty());
        assert!(!settings.hardware_acceleration_enabled);
        assert!(settings.onboarding_completed);
    }

    #[test]
    fn legacy_light_palette_migrates_without_custom_accent() {
        let settings: AppSettings = serde_json::from_value(serde_json::json!({
            "colorScheme": "frost",
            "customAccentColor": "#ff00ff",
            "language": "zh-CN"
        }))
        .expect("legacy palette should deserialize");

        assert_eq!(settings.color_scheme, ColorScheme::Light);
        let serialized = serde_json::to_value(settings).expect("settings should serialize");
        assert_eq!(serialized["colorScheme"], "light");
        assert!(serialized.get("customAccentColor").is_none());
    }

    #[test]
    fn legacy_system_theme_migrates_to_dark() {
        let settings: AppSettings = serde_json::from_value(serde_json::json!({
            "colorScheme": "system",
            "language": "zh-CN"
        }))
        .expect("legacy system theme should deserialize");

        assert_eq!(settings.color_scheme, ColorScheme::Dark);
        let serialized = serde_json::to_value(settings).expect("settings should serialize");
        assert_eq!(serialized["colorScheme"], "dark");
    }

    #[test]
    fn show_reasoning_patch_is_optional_and_mergeable() {
        let settings = AppSettings::default();
        assert!(settings.merge(AppSettingsPatch::default()).show_reasoning);

        let patch = AppSettingsPatch {
            show_reasoning: Some(false),
            ..AppSettingsPatch::default()
        };
        assert!(!settings.merge(patch).show_reasoning);
    }

    #[test]
    fn hardware_acceleration_defaults_off_and_can_be_enabled() {
        let settings = AppSettings::default();
        assert!(!settings.hardware_acceleration_enabled);
        let patch = AppSettingsPatch {
            hardware_acceleration_enabled: Some(true),
            ..AppSettingsPatch::default()
        };
        assert!(settings.merge(patch).hardware_acceleration_enabled);
    }

    #[test]
    fn model_provider_patch_is_optional_and_mergeable() {
        let settings = AppSettings::default();
        let patch = AppSettingsPatch {
            chat_model_provider: Some("provider-a".into()),
            multimodal_model_provider: Some("provider-b".into()),
            ..AppSettingsPatch::default()
        };
        let merged = settings.merge(patch);
        assert_eq!(merged.chat_model_provider, "provider-a");
        assert_eq!(merged.multimodal_model_provider, "provider-b");
    }
}
