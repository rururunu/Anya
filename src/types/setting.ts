export type ColorScheme = "dark" | "light";

export const LIGHT_COLOR_SCHEMES = new Set<ColorScheme>(["light"]);

/** localStorage key so boot splash can paint the right theme before settings IPC. */
export const COLOR_SCHEME_CACHE_KEY = "anya.colorScheme";

export function normalizeColorScheme(value: unknown): ColorScheme {
  if (value === "paper" || value === "light" || value === "cream" || value === "frost") {
    return "light";
  }
  if (
    value === "dark" ||
    value === "system" ||
    value === "auto" ||
    value === "default" ||
    value === "nocturne" ||
    value === "blue-black" ||
    value === "midnight" ||
    value === "forest" ||
    value === "rose" ||
    value === "ocean" ||
    value === "graphite" ||
    value === "ember" ||
    value === "teal" ||
    value === "ghost-pastel"
  ) {
    return "dark";
  }
  // Missing / unknown → light (app default)
  return "light";
}

export function readCachedColorScheme(): ColorScheme {
  try {
    return normalizeColorScheme(localStorage.getItem(COLOR_SCHEME_CACHE_KEY));
  } catch {
    return "light";
  }
}

export function isLightColorScheme(scheme: ColorScheme): boolean {
  return LIGHT_COLOR_SCHEMES.has(scheme);
}

export type AppLanguage = "zh-CN" | "en-US" | "ja-JP" | "ru-RU" | "de-DE" | "fr-FR" | "ko-KR";

export type ReasoningEffort = "disabled" | "high" | "max";

export type ReasoningLanguage = "auto" | "zh" | "en";

/** How agent tool/work cards are shown in chat. */
export type AgentWorkDisplay = "detailed" | "compact";

export type WebSearchProvider = "serper" | "tavily";

export type SemanticSearchModel =
  | "multilingual-e5-small"
  | "bge-small-zh-v1.5"
  | "bge-small-en-v1.5"
  | "jina-embeddings-v2-base-code"
  | "bge-m3";

export type SemanticSearchBackend = "api" | "local";

export type SemanticSearchState =
  | { status: "idle" }
  | { status: "downloading" }
  | { status: "ready" }
  | { status: "error"; message: string };

export interface SemanticSearchConfig {
  enabled: boolean;
  backend: SemanticSearchBackend;
  model: SemanticSearchModel;
  apiBaseUrl: string;
  apiKey: string;
  apiModel: string;
}

export type ToolApprovalMode = "ask" | "auto" | "alwaysAllow";

/** Chat interaction mode: Agent can mutate; Ask exposes read-only tools only. */
export type ChatMode = "agent" | "ask" | "plan";

export function normalizeChatMode(value: unknown): ChatMode {
  if (value === "ask" || value === "plan" || value === "agent") {
    return value;
  }
  return "agent";
}

/** Settings sidebar / search category ids. */
export type CategoryId =
  | "appearance"
  | "ai"
  | "memory"
  | "search"
  | "agent"
  | "mcp"
  | "skills"
  | "plugins"
  | "workspace"
  | "history"
  | "usage"
  | "about"
  | "provider"
  | "rag";

/** First item in the settings sidebar (Appearance). */
export const DEFAULT_SETTINGS_CATEGORY: CategoryId = "appearance";

export interface LspServerConfig {
  id: string;
  languages: string[];
  command: string;
  args?: string[];
  enabled?: boolean;
}

export interface McpServerConfig {
  id: string;
  /** Display name from catalog or manual entry; falls back to id in UI. */
  title?: string;
  description?: string;
  command: string;
  args?: string[];
  env?: Array<[string, string]>;
  enabled?: boolean;
  /** Remote icon URL captured at install time (Smithery / catalog). */
  iconUrl?: string;
  /** Stable registry identity, e.g. Smithery `qualifiedName`. */
  qualifiedName?: string;
  /** Upstream registry record id (Smithery server.id). */
  registryId?: string;
  homepage?: string;
  /** `smithery` | `catalog` | `manual` */
  source?: string;
}

export type ProviderApiProtocol = "chatCompletions" | "responses";

export const DEFAULT_PROVIDER_API_PROTOCOL: ProviderApiProtocol = "chatCompletions";

export function normalizeProviderApiProtocol(value: unknown): ProviderApiProtocol {
  return value === "responses" ? "responses" : "chatCompletions";
}

export interface CustomProviderConfig {
  id: string;
  name: string;
  baseUrl: string;
  apiKey: string;
  /** Newline-separated model IDs (legacy comma-separated still accepted when loading). */
  models: string;
  /** Optional preset template id (mimo / zhipu / …) for icons and defaults. */
  presetId?: string;
  /** `chatCompletions` (default) or `responses`. Grok thinking needs Responses. */
  apiProtocol?: ProviderApiProtocol;
}

export interface GeminiOAuthSettings {
  clientId: string;
  clientSecret: string;
  accessToken: string;
  refreshToken: string;
  expiresAt: number;
  email: string;
  projectId: string;
}

export interface GeminiAuthStatus {
  loggedIn: boolean;
  email: string;
  hasClientSecret: boolean;
  clientId: string;
}

export const DEFAULT_GEMINI_OAUTH_CLIENT_ID = "";

export const DEFAULT_GEMINI_OAUTH_CLIENT_SECRET = "";

export function defaultGeminiOAuthSettings(): GeminiOAuthSettings {
  return {
    clientId: DEFAULT_GEMINI_OAUTH_CLIENT_ID,
    clientSecret: DEFAULT_GEMINI_OAUTH_CLIENT_SECRET,
    accessToken: "",
    refreshToken: "",
    expiresAt: 0,
    email: "",
    projectId: "",
  };
}

export interface AppSettings {
  colorScheme: ColorScheme;
  language: AppLanguage;
  deepseekApiKey: string;
  geminiOauth: GeminiOAuthSettings;
  memoryEnabled: boolean;
  mem0ApiKey: string;
  mem0UserId: string;
  mem0BaseUrl: string;
  webSearchEnabled: boolean;
  webSearchProvider: WebSearchProvider;
  serperApiKey: string;
  tavilyApiKey: string;
  toolApprovalMode: ToolApprovalMode;
  chatMode: ChatMode;
  lspEnabled: boolean;
  lspServers: LspServerConfig[];
  mcpServers: McpServerConfig[];
  /** Smithery API key for hosted MCP (query `api_key`, avoids local browser OAuth). */
  smitheryApiKey: string;
  /** Built-in skill names opted in for the agent. Empty = none enabled. */
  enabledBuiltinSkills: string[];
  opacity: number;
  chromeFrostedGlass: boolean;
  chatModel: string;
  chatModelProvider: string;
  multimodalModel: string;
  multimodalModelProvider: string;
  multimodalSplitAnalysis: boolean;
  /** Use 1M-token context window for compaction / turn budgets. */
  largeContextEnabled: boolean;
  reasoningEffort: ReasoningEffort;
  reasoningLanguage: ReasoningLanguage;
  /** Pass reasoning_content back on tool-call turns (thinking + tools protocol). */
  passToolReasoning: boolean;
  /**
   * Keep thinking enabled on agent-loop rounds after tools (default on).
   * Turn off to skip thinking on continuation rounds and save tokens.
   */
  continueThinkingAfterTools: boolean;
  /** Display reasoning content supplied by the model in chat. */
  showReasoning: boolean;
  /** detailed = shell/diff inline; compact = fold into process details. */
  agentWorkDisplay: AgentWorkDisplay;
  multiModelCollaboration: boolean;
  collaborationModels: string[];
  /** Prefer YAGNI / minimal diffs via an optional system prompt block. */
  minimalCoding: boolean;
  zoom: number;
  /** WebView2 hardware acceleration. A full app restart is required to apply changes. */
  hardwareAccelerationEnabled: boolean;
  primaryHotkey: string;
  /** Listen for double-tap primary shortcut globally. */
  primaryHotkeyEnabled: boolean;
  secondaryHotkey: string;
  /** Listen for secondary chord shortcut globally. */
  secondaryHotkeyEnabled: boolean;
  customProviders: CustomProviderConfig[];
  /** Show AI button on PixPin pin windows. */
  pixpinPinAiEnabled: boolean;
  /** Show AI button on Snipaste pin windows. */
  snipastePinAiEnabled: boolean;
  /** Semantic workspace search via embeddings (API or local model). */
  semanticSearchEnabled: boolean;
  semanticSearchBackend: SemanticSearchBackend;
  semanticSearchModel: SemanticSearchModel;
  semanticSearchApiBaseUrl: string;
  semanticSearchApiKey: string;
  semanticSearchApiModel: string;
  /** First-run welcome wizard completed. */
  onboardingCompleted: boolean;
}

export interface AppSettingsPatch {
  colorScheme?: ColorScheme;
  language?: AppLanguage;
  deepseekApiKey?: string;
  geminiOauth?: GeminiOAuthSettings;
  memoryEnabled?: boolean;
  mem0ApiKey?: string;
  mem0UserId?: string;
  mem0BaseUrl?: string;
  webSearchEnabled?: boolean;
  webSearchProvider?: WebSearchProvider;
  serperApiKey?: string;
  tavilyApiKey?: string;
  toolApprovalMode?: ToolApprovalMode;
  chatMode?: ChatMode;
  lspEnabled?: boolean;
  lspServers?: LspServerConfig[];
  mcpServers?: McpServerConfig[];
  smitheryApiKey?: string;
  enabledBuiltinSkills?: string[];
  opacity?: number;
  chromeFrostedGlass?: boolean;
  chatModel?: string;
  chatModelProvider?: string;
  multimodalModel?: string;
  multimodalModelProvider?: string;
  multimodalSplitAnalysis?: boolean;
  largeContextEnabled?: boolean;
  reasoningEffort?: ReasoningEffort;
  reasoningLanguage?: ReasoningLanguage;
  passToolReasoning?: boolean;
  continueThinkingAfterTools?: boolean;
  showReasoning?: boolean;
  agentWorkDisplay?: AgentWorkDisplay;
  multiModelCollaboration?: boolean;
  collaborationModels?: string[];
  minimalCoding?: boolean;
  zoom?: number;
  hardwareAccelerationEnabled?: boolean;
  primaryHotkey?: string;
  primaryHotkeyEnabled?: boolean;
  secondaryHotkey?: string;
  secondaryHotkeyEnabled?: boolean;
  customProviders?: CustomProviderConfig[];
  pixpinPinAiEnabled?: boolean;
  snipastePinAiEnabled?: boolean;
  semanticSearchEnabled?: boolean;
  semanticSearchBackend?: SemanticSearchBackend;
  semanticSearchModel?: SemanticSearchModel;
  semanticSearchApiBaseUrl?: string;
  semanticSearchApiKey?: string;
  semanticSearchApiModel?: string;
  onboardingCompleted?: boolean;
}

export interface ModelSelection {
  id: string;
  provider: string;
}

export interface SelectOption<T extends string> {
  value: T;
  label: Partial<Record<AppLanguage, string>> & Pick<Record<AppLanguage, string>, "en-US">;
}

export function localizedOptionLabel<T extends string>(
  option: SelectOption<T>,
  language: AppLanguage,
) {
  return option.label[language] ?? option.label["en-US"];
}

export const colorSchemeOptions: SelectOption<ColorScheme>[] = [
  {
    value: "dark",
    label: { "zh-CN": "深色", "en-US": "Dark" },
  },
  {
    value: "light",
    label: { "zh-CN": "浅色", "en-US": "Light" },
  },
];

export const languageOptions: SelectOption<AppLanguage>[] = [
  {
    value: "zh-CN",
    label: {
      "zh-CN": "简体中文",
      "en-US": "Simplified Chinese",
      "ja-JP": "簡体字中国語",
      "ru-RU": "Китайский (упрощенный)",
      "de-DE": "Chinesisch (vereinfacht)",
      "fr-FR": "Chinois simplifié",
      "ko-KR": "중국어(간체)",
    },
  },
  {
    value: "en-US",
    label: { "zh-CN": "English", "en-US": "English" },
  },
  { value: "ja-JP", label: { "en-US": "Japanese", "ja-JP": "日本語" } },
  { value: "ru-RU", label: { "en-US": "Russian", "ru-RU": "Русский" } },
  { value: "de-DE", label: { "en-US": "German", "de-DE": "Deutsch" } },
  { value: "fr-FR", label: { "en-US": "French", "fr-FR": "Français" } },
  { value: "ko-KR", label: { "en-US": "Korean", "ko-KR": "한국어" } },
];

export const reasoningEffortOptions: SelectOption<ReasoningEffort>[] = [
  {
    value: "disabled",
    label: {
      "zh-CN": "关闭思考",
      "en-US": "Disabled",
      "ja-JP": "無効",
      "ru-RU": "Отключено",
      "de-DE": "Deaktiviert",
      "fr-FR": "Désactivé",
      "ko-KR": "사용 안 함",
    },
  },
  {
    value: "high",
    label: {
      "zh-CN": "高",
      "en-US": "High",
      "ja-JP": "高",
      "ru-RU": "Высокая",
      "de-DE": "Hoch",
      "fr-FR": "Élevé",
      "ko-KR": "높음",
    },
  },
  {
    value: "max",
    label: {
      "zh-CN": "最高",
      "en-US": "Max",
      "ja-JP": "最大",
      "ru-RU": "Максимальная",
      "de-DE": "Maximal",
      "fr-FR": "Maximum",
      "ko-KR": "최대",
    },
  },
];

export const reasoningLanguageOptions: SelectOption<ReasoningLanguage>[] = [
  {
    value: "auto",
    label: {
      "zh-CN": "自动",
      "en-US": "Auto",
      "ja-JP": "自動",
      "ru-RU": "Авто",
      "de-DE": "Automatisch",
      "fr-FR": "Automatique",
      "ko-KR": "자동",
    },
  },
  {
    value: "zh",
    label: {
      "zh-CN": "中文",
      "en-US": "Chinese",
      "ja-JP": "中国語",
      "ru-RU": "Китайский",
      "de-DE": "Chinesisch",
      "fr-FR": "Chinois",
      "ko-KR": "중국어",
    },
  },
  {
    value: "en",
    label: { "zh-CN": "English", "en-US": "English" },
  },
];

export const webSearchProviderOptions: SelectOption<WebSearchProvider>[] = [
  {
    value: "serper",
    label: { "zh-CN": "Serper", "en-US": "Serper" },
  },
  {
    value: "tavily",
    label: { "zh-CN": "Tavily", "en-US": "Tavily" },
  },
];

export const semanticSearchModelOptions: SelectOption<SemanticSearchModel>[] = [
  {
    value: "multilingual-e5-small",
    label: {
      "zh-CN": "多语言 E5-Small（推荐，约 120MB）",
      "en-US": "Multilingual E5-Small (recommended, ~120MB)",
    },
  },
  {
    value: "bge-small-zh-v1.5",
    label: { "zh-CN": "BGE-Small 中文（约 95MB）", "en-US": "BGE-Small Chinese (~95MB)" },
  },
  {
    value: "bge-small-en-v1.5",
    label: { "zh-CN": "BGE-Small 英文（约 130MB）", "en-US": "BGE-Small English (~130MB)" },
  },
  {
    value: "jina-embeddings-v2-base-code",
    label: { "zh-CN": "Jina 代码嵌入（约 500MB）", "en-US": "Jina Code embeddings (~500MB)" },
  },
  {
    value: "bge-m3",
    label: {
      "zh-CN": "BGE-M3 多语言（约 2.3GB，较慢）",
      "en-US": "BGE-M3 multilingual (~2.3GB, slower)",
    },
  },
];

export const toolApprovalModeOptions: SelectOption<ToolApprovalMode>[] = [
  {
    value: "ask",
    label: { "zh-CN": "询问", "en-US": "Ask" },
  },
  {
    value: "auto",
    label: { "zh-CN": "自动", "en-US": "Auto" },
  },
  {
    value: "alwaysAllow",
    label: { "zh-CN": "一律允许", "en-US": "Always allow" },
  },
];

export const agentWorkDisplayOptions: SelectOption<AgentWorkDisplay>[] = [
  {
    value: "detailed",
    label: { "zh-CN": "详细显示", "en-US": "Detailed" },
  },
  {
    value: "compact",
    label: { "zh-CN": "轻量显示", "en-US": "Compact" },
  },
];

export const zoomOptions: SelectOption<string>[] = [
  {
    value: "80",
    label: { "zh-CN": "80%", "en-US": "80%" },
  },
  {
    value: "90",
    label: { "zh-CN": "90%", "en-US": "90%" },
  },
  {
    value: "100",
    label: {
      "zh-CN": "100% (默认)",
      "en-US": "100% (Default)",
      "ja-JP": "100%（既定）",
      "ru-RU": "100% (по умолчанию)",
      "de-DE": "100% (Standard)",
      "fr-FR": "100 % (par défaut)",
      "ko-KR": "100% (기본값)",
    },
  },
  {
    value: "110",
    label: { "zh-CN": "110%", "en-US": "110%" },
  },
  {
    value: "120",
    label: { "zh-CN": "120%", "en-US": "120%" },
  },
  {
    value: "130",
    label: { "zh-CN": "130%", "en-US": "130%" },
  },
  {
    value: "140",
    label: { "zh-CN": "140%", "en-US": "140%" },
  },
  {
    value: "150",
    label: { "zh-CN": "150%", "en-US": "150%" },
  },
  {
    value: "175",
    label: { "zh-CN": "175%", "en-US": "175%" },
  },
  {
    value: "200",
    label: { "zh-CN": "200%", "en-US": "200%" },
  },
];
