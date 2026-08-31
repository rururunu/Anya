import type { AppLanguage } from "@/types/setting";

export const settingsFieldIds = [
  "colorScheme",
  "language",
  "zoom",
  "hardwareAccelerationEnabled",
  "opacity",
  "chromeFrostedGlass",
  "primaryHotkey",
  "secondaryHotkey",
  "deepseekApiKey",
  "defaultModel",
  "multimodalModel",
  "imageModel",
  "multimodalSplitAnalysis",
  "largeContextEnabled",
  "reasoningEffort",
  "reasoningLanguage",
  "showReasoning",
  "passToolReasoning",
  "continueThinkingAfterTools",
  "memoryEnabled",
  "mem0ApiKey",
  "mem0UserId",
  "mem0BaseUrl",
  "webSearchEnabled",
  "webSearchProvider",
  "serperApiKey",
  "tavilyApiKey",
  "semanticSearchEnabled",
  "semanticSearchModel",
  "toolApprovalMode",
  "agentWorkDisplay",
  "lspEnabled",
  "multiModelCollaboration",
  "minimalCoding",
  "pixpinPinAiEnabled",
  "snipastePinAiEnabled",
  "appName",
  "appVersion",
  "appIdentifier",
] as const;
export type SettingFieldId = (typeof settingsFieldIds)[number];

export const settingsFieldHelpIds = [
  "hardwareAccelerationEnabled",
  "chromeFrostedGlass",
  "primaryHotkey",
  "secondaryHotkey",
  "passToolReasoning",
  "continueThinkingAfterTools",
] as const;
export type SettingHelpFieldId = (typeof settingsFieldHelpIds)[number];

type CategoryKey =
  | "appearance"
  | "ai"
  | "image"
  | "memory"
  | "search"
  | "agent"
  | "mcp"
  | "skills"
  | "plugins"
  | "workspace"
  | "history"
  | "archive"
  | "about"
  | "provider"
  | "rag";
type GroupKey =
  | "themeLanguage"
  | "window"
  | "hotkeys"
  | "performance"
  | "modelSelection"
  | "imageGeneration"
  | "context"
  | "reasoning"
  | "memory"
  | "mem0"
  | "webSearch"
  | "searchKeys"
  | "agentSafety"
  | "agentDisplay"
  | "agentCapabilities"
  | "plugins"
  | "about";
type PageDescKey = "appearance" | "ai" | "image" | "agent" | "memory" | "search" | "plugins";
type HistoryKey =
  | "search"
  | "title"
  | "selectAll"
  | "deleteSelected"
  | "clearAll"
  | "empty"
  | "deleteGroup"
  | "messages"
  | "open"
  | "archiveLabel"
  | "publicGroup"
  | "yesterday"
  | "cancel"
  | "deleteLabel";
type HistoryConfirmKey =
  | "deleteTitle"
  | "deleteSelectedDesc"
  | "deleteGroupTitle"
  | "deleteGroupDesc"
  | "deleteSingleDesc"
  | "clearTitle"
  | "clearDesc"
  | "deleteAllLabel";

type ImageKey =
  | "title"
  | "description"
  | "providersTitle"
  | "addProvider"
  | "urlPlaceholder"
  | "namePlaceholder"
  | "modelsHint"
  | "noModels"
  | "modelTitle"
  | "templatesTitle"
  | "templatesHint"
  | "addTemplate"
  | "templateName"
  | "templateNamePlaceholder"
  | "templatePrompt"
  | "templatePromptPlaceholder"
  | "templateExample"
  | "templateExampleHint"
  | "templatePickImage"
  | "templateRemoveImage"
  | "templateDelete"
  | "templateEmpty";

type RagKey =
  | "title"
  | "description"
  | "enableLabel"
  | "enableHint"
  | "backendLabel"
  | "backendApi"
  | "backendLocal"
  | "apiBaseUrlLabel"
  | "apiBaseUrlHint"
  | "apiKeyLabel"
  | "apiModelLabel"
  | "apiModelPlaceholder"
  | "fetchModels"
  | "fetchingModels"
  | "testConnection"
  | "testing"
  | "testOk"
  | "testOkDetail"
  | "modelLabel"
  | "modelHint"
  | "save"
  | "saving"
  | "idle"
  | "on"
  | "downloading"
  | "ready"
  | "error"
  | "unsaved"
  | "fetchOk"
  | "fetchFail"
  | "incomplete";

type ArchiveKey =
  | "title"
  | "description"
  | "conversations"
  | "workspaces"
  | "emptyConversations"
  | "emptyWorkspaces"
  | "noMatchingConversations"
  | "noMatchingWorkspaces"
  | "search"
  | "searchSubmit"
  | "searchClear"
  | "selectAll"
  | "restoreSelected"
  | "deleteSelected"
  | "restore"
  | "delete"
  | "deleteConversation"
  | "deleteWorkspace"
  | "deleteSelectedConversations"
  | "deleteSelectedWorkspaces"
  | "deleteSelectedMixed";

export type SettingsI18nKey =
  | "settings.title"
  | "settings.minimize"
  | "settings.close"
  | "settings.sidebarLabel"
  | "settings.searchPlaceholder"
  | "settings.apiKeyPlaceholder"
  | "settings.empty"
  | "settings.hotkey.record"
  | "settings.hotkey.recording"
  | "settings.hotkey.reset"
  | "settings.hotkey.listenOn"
  | "settings.hotkey.listenOff"
  | "settings.hotkey.toggleListen"
  | "settings.provider.deepseek"
  | "settings.provider.gemini"
  | "settings.provider.geminiDescription"
  | "settings.provider.geminiClientId"
  | "settings.provider.geminiClientSecret"
  | "settings.provider.geminiClientSecretHint"
  | "settings.provider.geminiAccount"
  | "settings.provider.geminiLogin"
  | "settings.provider.geminiLogout"
  | "settings.provider.geminiLoggingIn"
  | "settings.provider.geminiCancelLogin"
  | "settings.provider.geminiSignedIn"
  | "settings.provider.geminiSignedOut"
  | "settings.provider.geminiImportCredentials"
  | "settings.provider.custom"
  | "settings.provider.title"
  | "settings.provider.description"
  | "settings.provider.apiKey"
  | "settings.provider.getApiKey"
  | "settings.provider.baseUrl"
  | "settings.provider.apiProtocol"
  | "settings.provider.apiProtocolHint"
  | "settings.provider.apiProtocolGrokHint"
  | "settings.provider.apiProtocolChatCompletions"
  | "settings.provider.apiProtocolResponses"
  | "settings.provider.apiProtocolAnthropic"
  | "settings.provider.modelProtocol"
  | "settings.provider.modelProtocolUnset"
  | "settings.provider.modelProtocolChatCompletions"
  | "settings.provider.modelProtocolResponses"
  | "settings.provider.modelProtocolAnthropic"
  | "settings.provider.modelsList"
  | "settings.provider.modelsPlaceholder"
  | "settings.provider.addModel"
  | "settings.provider.modelsEmpty"
  | "settings.provider.removeModel"
  | "settings.provider.enableModel"
  | "settings.provider.disableModel"
  | "settings.provider.configured"
  | "settings.provider.notConfigured"
  | "settings.provider.presets"
  | "settings.provider.presetHint"
  | "settings.provider.addBlank"
  | "settings.provider.modelsHint"
  | "settings.provider.fetchModels"
  | "settings.provider.fetchingModels"
  | "settings.provider.fetchModelsFailed"
  | "settings.provider.urlPlaceholder"
  | "settings.provider.save"
  | "settings.provider.saved"
  | "settings.provider.back"
  | "settings.provider.name"
  | "settings.provider.namePlaceholder"
  | "settings.provider.add"
  | "settings.provider.delete"
  | "settings.provider.deleteConfirm"
  | `settings.categories.${CategoryKey}`
  | `settings.groups.${GroupKey}`
  | `settings.pages.${PageDescKey}.description`
  | `settings.fields.${SettingFieldId}.title`
  | `settings.fields.${SettingFieldId}.description`
  | `settings.fields.${SettingHelpFieldId}.help`
  | `settings.history.${HistoryKey}`
  | `settings.historyConfirm.${HistoryConfirmKey}`
  | `settings.archive.${ArchiveKey}`
  | `settings.rag.${RagKey}`
  | `settings.image.${ImageKey}`;

export const settingsEn: Record<SettingsI18nKey, string> = {
  "settings.title": "Settings",
  "settings.minimize": "Minimize",
  "settings.close": "Close",
  "settings.sidebarLabel": "Categories",
  "settings.searchPlaceholder": "Search settings",
  "settings.apiKeyPlaceholder": "sk-...",
  "settings.empty": "No matching settings found",
  "settings.hotkey.record": "Record",
  "settings.hotkey.recording": "Press shortcut…",
  "settings.hotkey.reset": "Restore default",
  "settings.hotkey.listenOn": "Listening",
  "settings.hotkey.listenOff": "Not listening",
  "settings.hotkey.toggleListen": "Toggle global shortcut listening",

  "settings.provider.deepseek": "DeepSeek Provider",
  "settings.provider.gemini": "Gemini Provider",
  "settings.provider.geminiDescription":
    "Sign in with Google via Antigravity to use Gemini models (Cloud Code).",
  "settings.provider.geminiClientId": "OAuth Client ID",
  "settings.provider.geminiClientSecret": "OAuth Client Secret",
  "settings.provider.geminiClientSecretHint":
    "From Google Cloud → Credentials → your Desktop OAuth client. Kept on this device only.",
  "settings.provider.geminiAccount": "Google account",
  "settings.provider.geminiLogin": "Sign in with Google",
  "settings.provider.geminiLogout": "Sign out",
  "settings.provider.geminiLoggingIn": "Waiting for browser…",
  "settings.provider.geminiCancelLogin": "Cancel",
  "settings.provider.geminiSignedIn": "Signed in",
  "settings.provider.geminiSignedOut": "Not signed in",
  "settings.provider.geminiImportCredentials": "Import client_secret JSON",
  "settings.provider.custom": "Custom Provider",
  "settings.provider.title": "Provider Configurations",
  "settings.provider.description":
    "Configure DeepSeek, Gemini, or OpenAI-compatible vendors (MiMo, Kimi, GLM, MiniMax, Ark).",
  "settings.provider.apiKey": "API Key",
  "settings.provider.getApiKey": "Get a key:",
  "settings.provider.baseUrl": "Base URL",
  "settings.provider.apiProtocol": "API protocol",
  "settings.provider.apiProtocolHint":
    "Chat Completions is the OpenAI-compatible default. Choose Responses for Grok and other vendors that only stream reasoning on /v1/responses, or Anthropic for /v1/messages. Each model can override this below.",
  "settings.provider.apiProtocolGrokHint":
    "xAI Grok does not return thinking on Chat Completions. Switch this provider to Responses to show the reasoning process.",
  "settings.provider.apiProtocolChatCompletions": "Chat Completions (/v1/chat/completions)",
  "settings.provider.apiProtocolResponses": "Responses (/v1/responses)",
  "settings.provider.apiProtocolAnthropic": "Anthropic (/v1/messages)",
  "settings.provider.modelProtocol": "Model protocol",
  "settings.provider.modelProtocolUnset": "Unset",
  "settings.provider.modelProtocolChatCompletions": "Chat Completions",
  "settings.provider.modelProtocolResponses": "Responses",
  "settings.provider.modelProtocolAnthropic": "Anthropic",
  "settings.provider.modelsList": "Model List",
  "settings.provider.modelsPlaceholder": "Model ID, e.g. gpt-4o",
  "settings.provider.addModel": "Add",
  "settings.provider.modelsEmpty": "No models yet.",
  "settings.provider.removeModel": "Remove model",
  "settings.provider.enableModel": "Enable model",
  "settings.provider.disableModel": "Disable model",
  "settings.provider.configured": "Configured",
  "settings.provider.notConfigured": "Not configured",
  "settings.provider.presets": "Add provider",
  "settings.provider.presetHint":
    "Same card layout as above — pick a vendor to prefill URL and models, then enter your API key.",
  "settings.provider.addBlank": "Custom / blank",
  "settings.provider.modelsHint":
    "Press Enter or Add. You can paste several IDs separated by commas.",
  "settings.provider.fetchModels": "Fetch from API",
  "settings.provider.fetchingModels": "Fetching…",
  "settings.provider.fetchModelsFailed": "Could not list models from this endpoint.",
  "settings.provider.urlPlaceholder": "https://api.example.com/v1",
  "settings.provider.save": "Save Settings",
  "settings.provider.saved": "Settings saved successfully",
  "settings.provider.back": "Back",
  "settings.provider.name": "Provider Name",
  "settings.provider.namePlaceholder": "e.g. Ollama, OpenRouter",
  "settings.provider.add": "Add",
  "settings.provider.delete": "Delete Provider",
  "settings.provider.deleteConfirm": "Are you sure you want to delete this provider?",

  "settings.categories.appearance": "Appearance",
  "settings.categories.ai": "Model",
  "settings.categories.image": "Image",
  "settings.categories.memory": "Memory",
  "settings.categories.search": "Search",
  "settings.categories.agent": "Agent",
  "settings.categories.mcp": "MCP",
  "settings.categories.skills": "Skills",
  "settings.categories.plugins": "Pin tools",
  "settings.categories.workspace": "Workspace",
  "settings.categories.history": "History",
  "settings.categories.archive": "Archive",
  "settings.categories.about": "About",
  "settings.categories.provider": "Provider",
  "settings.categories.rag": "RAG Search",

  "settings.rag.title": "RAG Search",
  "settings.rag.description":
    "Semantic workspace search so the agent can find relevant code across files, via an OpenAI-compatible embeddings API or a local model.",
  "settings.rag.enableLabel": "Enable semantic search",
  "settings.rag.enableHint": "Nothing is downloaded or requested while off.",
  "settings.rag.backendLabel": "Embedding backend",
  "settings.rag.backendApi": "API (OpenAI-compatible)",
  "settings.rag.backendLocal": "Local model",
  "settings.rag.apiBaseUrlLabel": "API base URL",
  "settings.rag.apiBaseUrlHint": "Requests go to this URL's /embeddings endpoint.",
  "settings.rag.apiKeyLabel": "API Key",
  "settings.rag.apiModelLabel": "Model",
  "settings.rag.apiModelPlaceholder": "Qwen/Qwen3-VL-Embedding-8B or BAAI/bge-m3",
  "settings.rag.fetchModels": "Fetch models",
  "settings.rag.fetchingModels": "Fetching…",
  "settings.rag.testConnection": "Test connection",
  "settings.rag.testing": "Testing…",
  "settings.rag.testOk": "Connected",
  "settings.rag.testOkDetail": "Connected · dim {dim}",
  "settings.rag.modelLabel": "Local model",
  "settings.rag.modelHint": "Downloaded on first apply, then works offline.",
  "settings.rag.save": "Save & apply",
  "settings.rag.saving": "Saving…",
  "settings.rag.idle": "Off",
  "settings.rag.on": "On",
  "settings.rag.downloading": "Downloading model…",
  "settings.rag.ready": "Ready",
  "settings.rag.error": "Error",
  "settings.rag.unsaved": "Unsaved changes",
  "settings.rag.fetchOk": "Fetched {count} models",
  "settings.rag.fetchFail": "Could not fetch models",
  "settings.rag.incomplete": "Fill in the API URL, key, and model.",

  "settings.groups.themeLanguage": "Theme & language",
  "settings.groups.window": "Window",
  "settings.groups.hotkeys": "Shortcuts",
  "settings.groups.performance": "Performance",
  "settings.groups.modelSelection": "Models",
  "settings.groups.imageGeneration": "Model",
  "settings.groups.context": "Context",
  "settings.groups.reasoning": "Reasoning",
  "settings.groups.memory": "Memory",
  "settings.groups.mem0": "mem0",
  "settings.groups.webSearch": "Web search",
  "settings.groups.searchKeys": "API keys",
  "settings.groups.agentSafety": "Safety",
  "settings.groups.agentDisplay": "Display",
  "settings.groups.agentCapabilities": "Capabilities",
  "settings.groups.plugins": "Badges",
  "settings.groups.about": "Application",

  "settings.pages.appearance.description": "Theme, window, and the shortcuts that wake Anya.",
  "settings.pages.ai.description":
    "Default chat model, vision fallback, and reasoning. Configure API keys under Provider. Image generation is under Image.",
  "settings.pages.image.description":
    "Dedicated Images providers and models. Chat providers are never used for generate_image.",
  "settings.pages.agent.description": "Tool approval, work display, and coding behavior.",
  "settings.pages.memory.description":
    "Long-term preferences and project conventions. Leave the API key empty to use local memory.",
  "settings.pages.search.description":
    "Give the model web_search. Use RAG Search for workspace semantics.",
  "settings.pages.plugins.description":
    "Show an AI badge on PixPin / Snipaste pins to attach the image to a message.",

  "settings.image.title": "Image generation",
  "settings.image.description":
    "Chat providers are not used here. Add an Images API on this page. Official example: Base URL https://api.openai.com/v1, model gpt-image-2.",
  "settings.image.providersTitle": "Images providers",
  "settings.image.addProvider": "Add Images provider",
  "settings.image.urlPlaceholder": "https://api.openai.com/v1",
  "settings.image.namePlaceholder": "e.g. OpenAI Images",
  "settings.image.modelsHint": "Only these IDs appear in the picker below. Do not add chat models.",
  "settings.image.noModels": "Add a model ID on an Images provider first (e.g. gpt-image-2).",
  "settings.image.modelTitle": "Image model",
  "settings.image.templatesTitle": "Style templates",
  "settings.image.templatesHint":
    "Custom styles appear in Image mode. Hover the question mark in the list to preview the prompt. An example image enables image-to-image.",
  "settings.image.addTemplate": "Add template",
  "settings.image.templateName": "Name",
  "settings.image.templateNamePlaceholder": "e.g. Film still",
  "settings.image.templatePrompt": "Style prompt",
  "settings.image.templatePromptPlaceholder":
    "Appended to generate_image, e.g. cinematic lighting, 35mm, shallow depth of field",
  "settings.image.templateExample": "Example image",
  "settings.image.templateExampleHint":
    "Optional. Used as the image-to-image reference when this template is selected (composer attachments take priority).",
  "settings.image.templatePickImage": "Choose image",
  "settings.image.templateRemoveImage": "Remove",
  "settings.image.templateDelete": "Delete template",
  "settings.image.templateEmpty": "No custom templates yet.",

  "settings.fields.colorScheme.title": "Color Scheme",
  "settings.fields.colorScheme.description": "Choose light or dark.",
  "settings.fields.language.title": "Language",
  "settings.fields.language.description": "Choose the display language for the interface.",
  "settings.fields.zoom.title": "Interface Zoom",
  "settings.fields.zoom.description":
    "Adjust the scale of interface elements and fonts for high-DPI displays.",
  "settings.fields.hardwareAccelerationEnabled.title": "Hardware acceleration",
  "settings.fields.hardwareAccelerationEnabled.description":
    "Requires a full app restart to take effect.",
  "settings.fields.hardwareAccelerationEnabled.help":
    "Use the GPU for WebView2 rendering. Disabled by default for broader driver compatibility.",
  "settings.fields.opacity.title": "Opacity",
  "settings.fields.opacity.description":
    "Adjust window opacity and enable frosted glass background.",
  "settings.fields.chromeFrostedGlass.title": "Frosted glass chrome",
  "settings.fields.chromeFrostedGlass.description":
    "The first time you turn this on, the app restarts.",
  "settings.fields.chromeFrostedGlass.help":
    "Use a translucent frosted-glass titlebar and sidebars. The conversation area stays opaque.",
  "settings.fields.primaryHotkey.title": "Primary shortcut",
  "settings.fields.primaryHotkey.description":
    "Two quick taps on the modifier. Default: double Alt.",
  "settings.fields.primaryHotkey.help":
    "Record the modifier to open Anya. Turn off the switch to stop listening for this global shortcut.",
  "settings.fields.secondaryHotkey.title": "Secondary shortcut",
  "settings.fields.secondaryHotkey.description":
    "Backup for apps that steal double-Alt. Default: Ctrl+Alt+Space.",
  "settings.fields.secondaryHotkey.help":
    "Record a backup shortcut for apps that steal double-Alt (e.g. IDEA). Turn off the switch to stop listening.",
  "settings.fields.deepseekApiKey.title": "API Key",
  "settings.fields.deepseekApiKey.description":
    "Used for DeepSeek chat requests. Stored locally only. Get a key: https://platform.deepseek.com/api_keys",
  "settings.fields.defaultModel.title": "Default model",
  "settings.fields.defaultModel.description":
    "The model used for new chats. The list is loaded from configured providers.",
  "settings.fields.multimodalModel.title": "Multimodal model",
  "settings.fields.multimodalModel.description":
    "Fallback vision model used only when the primary chat model cannot see images (e.g. DeepSeek-R1). Gemini / GPT-4o already see images natively and ignore this.",
  "settings.fields.imageModel.title": "Image model",
  "settings.fields.imageModel.description":
    "Used by generate_image (default gpt-image-2). Choose from Images providers on this page.",
  "settings.fields.multimodalSplitAnalysis.title": "Split multimodal analysis",
  "settings.fields.multimodalSplitAnalysis.description":
    "For text-only primary models: the multimodal model describes the image, then the primary model answers and runs tools. Not used when the primary model already supports vision (e.g. Gemini).",
  "settings.fields.largeContextEnabled.title": "1M context window",
  "settings.fields.largeContextEnabled.description":
    "Raise the compaction / turn budget ceiling to 1,000,000 tokens. The effective window is still capped by the selected model's native limit (unknown models stay at 256k).",
  "settings.fields.reasoningEffort.title": "Reasoning Effort",
  "settings.fields.reasoningEffort.description":
    "Preferred thinking depth. The input bar shows each model's official levels and names (DeepSeek: off/low/high/max; GPT: none/minimal/low/…; Grok: low/medium/high/xhigh). Unsupported values are mapped.",
  "settings.fields.reasoningLanguage.title": "Reasoning Language",
  "settings.fields.reasoningLanguage.description":
    "Preferences for visible reasoning and final answer language (transient injection).",
  "settings.fields.showReasoning.title": "Show reasoning process",
  "settings.fields.showReasoning.description":
    "Show model reasoning in chat when the provider supplies it.",
  "settings.fields.passToolReasoning.title": "Pass tool-turn reasoning",
  "settings.fields.passToolReasoning.description": "Turning off may cause request errors.",
  "settings.fields.passToolReasoning.help":
    "Include reasoning text on assistant turns with tool calls. Needed for thinking + tools.",
  "settings.fields.continueThinkingAfterTools.title": "Continue thinking after tools",
  "settings.fields.continueThinkingAfterTools.description": "Turn off to save tokens.",
  "settings.fields.continueThinkingAfterTools.help":
    "Keep thinking on for every agent round after tools (default). Turn off to skip thinking on later rounds.",
  "settings.fields.memoryEnabled.title": "Enable memory",
  "settings.fields.memoryEnabled.description":
    "Recall relevant memories and save durable preferences and project rules.",
  "settings.fields.mem0ApiKey.title": "mem0 API Key",
  "settings.fields.mem0ApiKey.description":
    "Connects to mem0 and is stored only in local settings. Leave empty for local memory. Get a key: https://app.mem0.ai/dashboard/api-keys",
  "settings.fields.mem0UserId.title": "User ID",
  "settings.fields.mem0UserId.description":
    "A stable identifier used to isolate memories for this user.",
  "settings.fields.mem0BaseUrl.title": "Base URL",
  "settings.fields.mem0BaseUrl.description":
    "The mem0 API endpoint for compatible services or a self-hosted gateway.",
  "settings.fields.webSearchEnabled.title": "Enable web search",
  "settings.fields.webSearchEnabled.description":
    "When enabled with a valid provider API key, expose web_search to the model.",
  "settings.fields.webSearchProvider.title": "Search provider",
  "settings.fields.webSearchProvider.description":
    "Choose Serper or Tavily as the web search backend.",
  "settings.fields.serperApiKey.title": "Serper API Key",
  "settings.fields.serperApiKey.description":
    "Used for Serper search. Stored locally only. Get a key: https://serper.dev",
  "settings.fields.tavilyApiKey.title": "Tavily API Key",
  "settings.fields.tavilyApiKey.description":
    "Used for Tavily search. Stored locally only. Get a key: https://app.tavily.com",
  "settings.fields.semanticSearchEnabled.title": "Semantic workspace search",
  "settings.fields.semanticSearchEnabled.description":
    "Index workspace files with a local embedding model so the agent can find relevant code across files. The model downloads on first enable (offline after).",
  "settings.fields.semanticSearchModel.title": "Embedding model",
  "settings.fields.semanticSearchModel.description":
    "Model used for semantic ranking. Larger models are more accurate but slower to download and run.",
  "settings.fields.toolApprovalMode.title": "Tool approval mode",
  "settings.fields.toolApprovalMode.description":
    "Always allow still blocks dangerous shell via rules.",
  "settings.fields.agentWorkDisplay.title": "Agent work display",
  "settings.fields.agentWorkDisplay.description":
    "Detailed shows shell and code diffs inline in the chat timeline. Compact folds them into process details (collapsed by default). Read tools always stay in process details.",
  "settings.fields.lspEnabled.title": "Enable LSP",
  "settings.fields.lspEnabled.description":
    "Expose the lsp tool (hover / definition / diagnostics) when language servers are available.",
  "settings.fields.multiModelCollaboration.title": "Multi-model collaboration",
  "settings.fields.multiModelCollaboration.description":
    "Allow the main Agent to delegate suitable tasks to child Agents using the selected available models.",
  "settings.fields.minimalCoding.title": "Minimal coding mode",
  "settings.fields.minimalCoding.description":
    "When enabled, each turn injects a YAGNI ladder: reuse existing code, prefer stdlib/native features, and write the smallest correct change without cutting safety or validation.",
  "settings.fields.pixpinPinAiEnabled.title": "PixPin pin badge",
  "settings.fields.pixpinPinAiEnabled.description":
    "When a PixPin pin is on screen, show a small AI badge at its bottom-right. Click to open Anya with that image attached.",
  "settings.fields.snipastePinAiEnabled.title": "Snipaste pin badge",
  "settings.fields.snipastePinAiEnabled.description":
    "When a Snipaste pin is on screen, show a small AI badge at its bottom-right. Click to open Anya with that image attached.",
  "settings.fields.appName.title": "Application Name",
  "settings.fields.appName.description": "The installed application name.",
  "settings.fields.appVersion.title": "Version",
  "settings.fields.appVersion.description": "The current application version.",
  "settings.fields.appIdentifier.title": "Identifier",
  "settings.fields.appIdentifier.description": "The unique application identifier.",

  "settings.history.search": "Search history chats...",
  "settings.history.title": "Chat History",
  "settings.history.selectAll": "Select all",
  "settings.history.deleteSelected": "Delete selected ({count})",
  "settings.history.clearAll": "Clear all",
  "settings.history.empty": "No history found",
  "settings.history.deleteGroup": "Delete all chats in this group",
  "settings.history.messages": "{count} messages",
  "settings.history.open": "Open",
  "settings.history.archiveLabel": "Archive",
  "settings.history.publicGroup": "Public",
  "settings.history.yesterday": "Yesterday",
  "settings.history.cancel": "Cancel",
  "settings.history.deleteLabel": "Delete",

  "settings.historyConfirm.deleteTitle": "Delete chats",
  "settings.historyConfirm.deleteSelectedDesc":
    "Delete the selected {count} chats? This cannot be undone.",
  "settings.historyConfirm.deleteGroupTitle": "Delete group chats",
  "settings.historyConfirm.deleteGroupDesc":
    "Delete all {count} chats in \u201c{name}\u201d? This cannot be undone.",
  "settings.historyConfirm.deleteSingleDesc": "Delete this chat? This cannot be undone.",
  "settings.historyConfirm.clearTitle": "Clear history",
  "settings.historyConfirm.clearDesc": "Clear all chat history? This cannot be undone.",
  "settings.historyConfirm.deleteAllLabel": "Delete all",

  "settings.archive.title": "Archive",
  "settings.archive.description":
    "Archived conversations and workspaces stay on this device until you restore or delete them.",
  "settings.archive.conversations": "Conversations",
  "settings.archive.workspaces": "Workspaces",
  "settings.archive.emptyConversations": "No archived conversations.",
  "settings.archive.emptyWorkspaces": "No archived workspaces.",
  "settings.archive.noMatchingConversations": "No matching archived conversations.",
  "settings.archive.noMatchingWorkspaces": "No matching archived workspaces.",
  "settings.archive.search": "Search archived conversations and workspaces",
  "settings.archive.searchSubmit": "Search",
  "settings.archive.searchClear": "Clear",
  "settings.archive.selectAll": "Select all",
  "settings.archive.restoreSelected": "Restore selected ({count})",
  "settings.archive.deleteSelected": "Delete selected ({count})",
  "settings.archive.restore": "Restore",
  "settings.archive.delete": "Delete",
  "settings.archive.deleteConversation":
    "Permanently delete this conversation? This cannot be undone.",
  "settings.archive.deleteWorkspace":
    'Permanently delete workspace "{name}"? Project files will not be deleted.',
  "settings.archive.deleteSelectedConversations":
    "Permanently delete {count} archived conversations? This cannot be undone.",
  "settings.archive.deleteSelectedWorkspaces":
    "Permanently delete {count} archived workspaces? Project files will not be deleted.",
  "settings.archive.deleteSelectedMixed":
    "Permanently delete {sessionCount} conversations and {workspaceCount} workspaces? Project files will not be deleted.",
};

export const settingsLocales: Record<AppLanguage, Partial<Record<SettingsI18nKey, string>>> = {
  "en-US": settingsEn,
  "zh-CN": {
    "settings.title": "设置",
    "settings.minimize": "最小化",
    "settings.close": "关闭",
    "settings.sidebarLabel": "分类",
    "settings.searchPlaceholder": "搜索设置",
    "settings.empty": "未找到匹配的设置项",
    "settings.hotkey.record": "录制",
    "settings.hotkey.recording": "请按下快捷键…",
    "settings.hotkey.reset": "恢复默认",
    "settings.hotkey.listenOn": "正在监听",
    "settings.hotkey.listenOff": "未监听",
    "settings.hotkey.toggleListen": "开关全局快捷键监听",

    "settings.categories.appearance": "外观",
    "settings.categories.ai": "模型",
    "settings.categories.image": "生图",
    "settings.categories.memory": "记忆",
    "settings.categories.search": "搜索",
    "settings.categories.agent": "代理",
    "settings.categories.mcp": "MCP",
    "settings.categories.skills": "技能",
    "settings.categories.plugins": "贴图工具",
    "settings.categories.workspace": "工作区",
    "settings.categories.history": "历史",
    "settings.categories.archive": "归档",
    "settings.categories.about": "关于",
    "settings.categories.provider": "提供商",
    "settings.categories.rag": "RAG 检索",

    "settings.rag.title": "RAG 检索",
    "settings.rag.description":
      "语义化工作区检索，让 agent 能跨文件找到相关代码。可用 OpenAI 兼容的 embeddings API 或本地模型。",
    "settings.rag.enableLabel": "启用语义检索",
    "settings.rag.enableHint": "关闭时不下载、不发起任何请求。",
    "settings.rag.backendLabel": "嵌入后端",
    "settings.rag.backendApi": "API（OpenAI 兼容）",
    "settings.rag.backendLocal": "本地模型",
    "settings.rag.apiBaseUrlLabel": "API 地址",
    "settings.rag.apiBaseUrlHint": "请求会发到该地址的 /embeddings 接口。",
    "settings.rag.apiKeyLabel": "API Key",
    "settings.rag.apiModelLabel": "模型",
    "settings.rag.apiModelPlaceholder": "Qwen/Qwen3-VL-Embedding-8B 或 BAAI/bge-m3",
    "settings.rag.fetchModels": "拉取模型",
    "settings.rag.fetchingModels": "拉取中…",
    "settings.rag.testConnection": "测试连接",
    "settings.rag.testing": "测试中…",
    "settings.rag.testOk": "连接成功",
    "settings.rag.testOkDetail": "连接成功 · 维度 {dim}",
    "settings.rag.modelLabel": "本地模型",
    "settings.rag.modelHint": "首次应用时下载，之后离线可用。",
    "settings.rag.save": "保存并应用",
    "settings.rag.saving": "保存中…",
    "settings.rag.idle": "已关闭",
    "settings.rag.on": "已开启",
    "settings.rag.downloading": "模型下载中…",
    "settings.rag.ready": "已就绪",
    "settings.rag.error": "错误",
    "settings.rag.unsaved": "有未保存的更改",
    "settings.rag.fetchOk": "已拉取 {count} 个模型",
    "settings.rag.fetchFail": "无法拉取模型",
    "settings.rag.incomplete": "请填写 API 地址、API Key 与模型。",

    "settings.provider.deepseek": "DeepSeek 提供商",
    "settings.provider.gemini": "Gemini 提供商",
    "settings.provider.geminiDescription":
      "通过 Antigravity 使用 Google 账号登录后即可调用 Gemini（Cloud Code）。",
    "settings.provider.geminiClientId": "OAuth Client ID",
    "settings.provider.geminiClientSecret": "OAuth Client Secret",
    "settings.provider.geminiClientSecretHint":
      "来自 Google Cloud → 凭据 → Desktop OAuth 客户端，仅保存在本机。",
    "settings.provider.geminiAccount": "Google 账号",
    "settings.provider.geminiLogin": "使用 Google 登录",
    "settings.provider.geminiLogout": "退出登录",
    "settings.provider.geminiLoggingIn": "等待浏览器授权…",
    "settings.provider.geminiCancelLogin": "取消",
    "settings.provider.geminiSignedIn": "已登录",
    "settings.provider.geminiSignedOut": "未登录",
    "settings.provider.geminiImportCredentials": "导入 client_secret JSON",
    "settings.provider.custom": "自定义提供商",
    "settings.provider.title": "模型提供商配置",
    "settings.provider.description":
      "配置 DeepSeek、Gemini，或小米 MiMo / Kimi / 智谱 / MiniMax / 火山方舟等 OpenAI 兼容厂商。",
    "settings.provider.apiKey": "API Key",
    "settings.provider.getApiKey": "获取地址：",
    "settings.provider.baseUrl": "Base URL",
    "settings.provider.apiProtocol": "API 协议",
    "settings.provider.apiProtocolHint":
      "默认使用 OpenAI 兼容的 Chat Completions。Grok 等只在 /v1/responses 上流式返回思考过程的厂商请改选 Responses；需要 /v1/messages 的请选 Anthropic。下方可为单个模型覆盖。",
    "settings.provider.apiProtocolGrokHint":
      "xAI Grok 的 Chat Completions 不会返回思考过程。请把该厂商改成 Responses，才能看到推理内容。",
    "settings.provider.apiProtocolChatCompletions": "Chat Completions（/v1/chat/completions）",
    "settings.provider.apiProtocolResponses": "Responses（/v1/responses）",
    "settings.provider.apiProtocolAnthropic": "Anthropic（/v1/messages）",
    "settings.provider.modelProtocol": "模型协议",
    "settings.provider.modelProtocolUnset": "未设定",
    "settings.provider.modelProtocolChatCompletions": "Chat Completions",
    "settings.provider.modelProtocolResponses": "Responses",
    "settings.provider.modelProtocolAnthropic": "Anthropic",
    "settings.provider.modelsList": "模型列表",
    "settings.provider.modelsPlaceholder": "模型 ID，例如 gpt-4o",
    "settings.provider.addModel": "添加",
    "settings.provider.modelsEmpty": "暂无模型",
    "settings.provider.removeModel": "移除模型",
    "settings.provider.enableModel": "启用模型",
    "settings.provider.disableModel": "禁用模型",
    "settings.provider.configured": "已配置",
    "settings.provider.notConfigured": "未配置",
    "settings.provider.presets": "添加提供商",
    "settings.provider.presetHint": "与上方卡片同款布局 — 点选厂商预填地址与模型，再填写 API Key。",
    "settings.provider.addBlank": "自定义 / 空白",
    "settings.provider.modelsHint": "回车或点添加。也可粘贴多个 ID，用逗号分隔。",
    "settings.provider.fetchModels": "从接口拉取",
    "settings.provider.fetchingModels": "拉取中…",
    "settings.provider.fetchModelsFailed": "无法从该接口列出模型。",
    "settings.provider.urlPlaceholder": "https://api.example.com/v1",
    "settings.provider.save": "保存设置",
    "settings.provider.saved": "设置保存成功",
    "settings.provider.back": "返回",
    "settings.provider.name": "提供商名称",
    "settings.provider.namePlaceholder": "例如 Ollama, OpenRouter",
    "settings.provider.add": "添加",
    "settings.provider.delete": "删除提供商",
    "settings.provider.deleteConfirm": "确定要删除此提供商吗？",

    "settings.groups.themeLanguage": "主题与语言",
    "settings.groups.window": "窗口",
    "settings.groups.hotkeys": "快捷键",
    "settings.groups.performance": "性能",
    "settings.groups.modelSelection": "模型选择",
    "settings.groups.imageGeneration": "模型",
    "settings.groups.context": "上下文",
    "settings.groups.reasoning": "推理",
    "settings.groups.memory": "总开关",
    "settings.groups.mem0": "mem0",
    "settings.groups.webSearch": "联网搜索",
    "settings.groups.searchKeys": "密钥",
    "settings.groups.agentSafety": "安全",
    "settings.groups.agentDisplay": "显示",
    "settings.groups.agentCapabilities": "能力",
    "settings.groups.plugins": "角标",
    "settings.groups.about": "应用信息",

    "settings.pages.appearance.description": "主题、窗口与唤起快捷键。",
    "settings.pages.ai.description":
      "默认对话模型、视觉回退与推理行为。API Key 在「提供商」中配置。生图请到「生图」。",
    "settings.pages.image.description":
      "独立的生图提供商与模型。generate_image 不会使用聊天提供商。",
    "settings.pages.agent.description": "工具审批、过程展示与编码行为。",
    "settings.pages.memory.description": "长期偏好与项目约定。留空 API Key 时使用本地记忆。",
    "settings.pages.search.description": "给模型提供 web_search。工作区语义检索请到「RAG 检索」。",
    "settings.pages.plugins.description":
      "在 PixPin / Snipaste 贴图右下角显示 AI 角标，点击后把图片附加到消息。",

    "settings.image.title": "生图",
    "settings.image.description":
      "聊天提供商不会用在这里。请在本页添加 Images API。官方示例：Base URL https://api.openai.com/v1，模型 gpt-image-2。",
    "settings.image.providersTitle": "生图提供商",
    "settings.image.addProvider": "添加生图提供商",
    "settings.image.urlPlaceholder": "https://api.openai.com/v1",
    "settings.image.namePlaceholder": "例如：OpenAI Images",
    "settings.image.modelsHint": "只有这里列出的模型会出现在下方选择器里，不要填聊天模型。",
    "settings.image.noModels": "请先在生图提供商里添加模型 ID（例如 gpt-image-2）。",
    "settings.image.modelTitle": "生图模型",
    "settings.image.templatesTitle": "风格模板",
    "settings.image.templatesHint":
      "自定义风格会出现在生图模式的风格列表里。列表项后的问号可预览提示词。添加例图后可做图生图。",
    "settings.image.addTemplate": "添加模板",
    "settings.image.templateName": "名称",
    "settings.image.templateNamePlaceholder": "例如：电影静帧",
    "settings.image.templatePrompt": "风格提示词",
    "settings.image.templatePromptPlaceholder": "会附加到生图请求，例如：电影感布光、35mm、浅景深",
    "settings.image.templateExample": "例图",
    "settings.image.templateExampleHint":
      "可选。选中该模板时作为图生图参考（输入框里贴的图优先）。",
    "settings.image.templatePickImage": "选择图片",
    "settings.image.templateRemoveImage": "移除",
    "settings.image.templateDelete": "删除模板",
    "settings.image.templateEmpty": "还没有自定义模板。",

    "settings.fields.colorScheme.title": "颜色主题",
    "settings.fields.colorScheme.description": "选择浅色或深色。",
    "settings.fields.language.title": "界面语言",
    "settings.fields.language.description": "界面语言。",
    "settings.fields.zoom.title": "界面缩放",
    "settings.fields.zoom.description": "调整软件界面大小。",
    "settings.fields.hardwareAccelerationEnabled.title": "硬件加速",
    "settings.fields.hardwareAccelerationEnabled.description": "更改后需完整重启。",
    "settings.fields.hardwareAccelerationEnabled.help":
      "使用 GPU 加速 WebView2 界面渲染。默认关闭以提升驱动兼容性。",
    "settings.fields.opacity.title": "透明度",
    "settings.fields.opacity.description": "调整窗口背景透明度与毛玻璃效果。",
    "settings.fields.chromeFrostedGlass.title": "毛玻璃顶栏与侧栏",
    "settings.fields.chromeFrostedGlass.description": "首次开启会重启应用。",
    "settings.fields.chromeFrostedGlass.help": "顶栏和侧栏使用半透明毛玻璃，对话区域保持实色。",
    "settings.fields.primaryHotkey.title": "主快捷键",
    "settings.fields.primaryHotkey.description": "连按两下修饰键唤起。默认双击 Alt。",
    "settings.fields.primaryHotkey.help":
      "录制用于打开 Anya 的修饰键。关闭开关后不再监听该全局快捷键。",
    "settings.fields.secondaryHotkey.title": "副快捷键",
    "settings.fields.secondaryHotkey.description":
      "给会抢走双击 Alt 的应用备用。默认 Ctrl+Alt+Space。",
    "settings.fields.secondaryHotkey.help":
      "为会抢走双击 Alt 的应用（如 IDEA）录制备用快捷键。关闭开关后不再监听该全局快捷键。",
    "settings.fields.deepseekApiKey.title": "API Key",
    "settings.fields.deepseekApiKey.description":
      "用于 DeepSeek 请求，密钥仅保存在本机。获取地址：https://platform.deepseek.com/api_keys",
    "settings.fields.defaultModel.title": "默认模型",
    "settings.fields.defaultModel.description": "新对话默认使用的模型。列表从已配置的提供商获取。",
    "settings.fields.multimodalModel.title": "多模态模型",
    "settings.fields.multimodalModel.description":
      "仅当主模型本身不能看图时（如 DeepSeek-R1）才用作视觉回退；Gemini / GPT-4o 等原生识图模型不会走此路径。",
    "settings.fields.imageModel.title": "生图模型",
    "settings.fields.imageModel.description":
      "generate_image 使用（默认 gpt-image-2）。只从本页的生图提供商里选。",
    "settings.fields.multimodalSplitAnalysis.title": "多模态分步分析",
    "settings.fields.multimodalSplitAnalysis.description":
      "面向无视觉能力的主模型：多模态模型先描述图片，再由主模型作答与调用工具。主模型已支持识图（如 Gemini）时不会启用。",
    "settings.fields.largeContextEnabled.title": "1M 上下文",
    "settings.fields.largeContextEnabled.description":
      "将压缩与单轮预算上限提到约 100 万 token。实际窗口仍受当前模型原生上限约束（未知模型按 256k）。",
    "settings.fields.reasoningEffort.title": "思考深度",
    "settings.fields.reasoningEffort.description":
      "默认思考强度。输入栏会按当前模型显示官方档位和名称（DeepSeek：关闭/低/高/最高；GPT：none/minimal/low…；Grok：low/medium/high/xhigh）。不支持的值会自动映射。",
    "settings.fields.reasoningLanguage.title": "推理语言",
    "settings.fields.reasoningLanguage.description": "推理文本与回答语言偏好",
    "settings.fields.showReasoning.title": "显示思考过程",
    "settings.fields.showReasoning.description": "模型提供推理内容时，在聊天中显示思考过程。",
    "settings.fields.passToolReasoning.title": "工具轮次回传推理",
    "settings.fields.passToolReasoning.description": "关闭可能导致请求失败。",
    "settings.fields.passToolReasoning.help":
      "含 tool_calls 的 assistant 轮次会把推理内容带回 API。思考 + 工具协议需要此项。",
    "settings.fields.continueThinkingAfterTools.title": "续轮思考",
    "settings.fields.continueThinkingAfterTools.description": "关闭可节省 token。",
    "settings.fields.continueThinkingAfterTools.help":
      "工具执行后的每一轮继续开启思考（默认开启）。关闭后后续轮次跳过思考。",
    "settings.fields.memoryEnabled.title": "启用记忆",
    "settings.fields.memoryEnabled.description":
      "自动召回相关记忆，并按规则保存长期偏好与项目约定。",
    "settings.fields.mem0ApiKey.title": "mem0 API Key",
    "settings.fields.mem0ApiKey.description":
      "用于连接 mem0，密钥仅保存在本机设置中。留空时使用本地记忆。获取地址：https://app.mem0.ai/dashboard/api-keys",
    "settings.fields.mem0UserId.title": "User ID",
    "settings.fields.mem0UserId.description": "稳定的用户标识，用于隔离不同用户的记忆。",
    "settings.fields.mem0BaseUrl.title": "Base URL",
    "settings.fields.mem0BaseUrl.description": "mem0 API 地址，可配置兼容服务或自托管网关。",
    "settings.fields.webSearchEnabled.title": "启用联网搜索",
    "settings.fields.webSearchEnabled.description":
      "开启后，在已配置对应 Provider API Key 时向模型提供 web_search。",
    "settings.fields.webSearchProvider.title": "搜索 Provider",
    "settings.fields.webSearchProvider.description": "选择 Serper 或 Tavily 作为联网搜索后端。",
    "settings.fields.serperApiKey.title": "Serper API Key",
    "settings.fields.serperApiKey.description":
      "用于 Serper 搜索，密钥仅保存在本机设置中。获取地址：https://serper.dev",
    "settings.fields.tavilyApiKey.title": "Tavily API Key",
    "settings.fields.tavilyApiKey.description":
      "用于 Tavily 搜索，密钥仅保存在本机设置中。获取地址：https://app.tavily.com",
    "settings.fields.semanticSearchEnabled.title": "语义工作区检索",
    "settings.fields.semanticSearchEnabled.description":
      "用本地嵌入模型索引工作区文件，让 agent 能跨文件检索相关代码。模型在首次开启时下载（之后离线可用）。",
    "settings.fields.semanticSearchModel.title": "嵌入模型",
    "settings.fields.semanticSearchModel.description":
      "用于语义排序的模型。更大的模型更准确，但下载与推理更慢。",
    "settings.fields.toolApprovalMode.title": "工具审批模式",
    "settings.fields.toolApprovalMode.description": "一律允许仍会拦截危险 shell。",
    "settings.fields.agentWorkDisplay.title": "工作过程显示",
    "settings.fields.agentWorkDisplay.description":
      "详细显示：命令与代码 diff 直接穿插在对话时间线中。轻量显示：收入过程详情并默认折叠；读取类工具始终在过程详情中。",
    "settings.fields.lspEnabled.title": "启用 LSP",
    "settings.fields.lspEnabled.description":
      "启用后可使用 lsp 工具（hover / definition / diagnostics）。",
    "settings.fields.multiModelCollaboration.title": "多模型协同",
    "settings.fields.multiModelCollaboration.description":
      "允许主 Agent 将适合的任务交给使用所选模型的子 Agent。",
    "settings.fields.minimalCoding.title": "精简编码模式",
    "settings.fields.minimalCoding.description":
      "开启后，每轮对话注入 YAGNI 决策阶梯：优先复用现有代码与标准库/原生能力，只写最小正确改动，且不砍安全与校验。",
    "settings.fields.pixpinPinAiEnabled.title": "PixPin 贴图角标",
    "settings.fields.pixpinPinAiEnabled.description":
      "检测到 PixPin 贴图时，在其右下角显示 AI 角标。点击后打开 Anya 并将该图片附加到消息中。",
    "settings.fields.snipastePinAiEnabled.title": "Snipaste 贴图角标",
    "settings.fields.snipastePinAiEnabled.description":
      "检测到 Snipaste 贴图时，在其右下角显示 AI 角标。点击后打开 Anya 并将该图片附加到消息中。",
    "settings.fields.appName.title": "Application Name",
    "settings.fields.appName.description": "应用名称",
    "settings.fields.appVersion.title": "Version",
    "settings.fields.appVersion.description": "版本号。",
    "settings.fields.appIdentifier.title": "Identifier",
    "settings.fields.appIdentifier.description": "标识符。",

    "settings.history.search": "搜索历史对话...",
    "settings.history.title": "历史记录管理",
    "settings.history.selectAll": "全选",
    "settings.history.deleteSelected": "删除选中 ({count})",
    "settings.history.clearAll": "清空全部",
    "settings.history.empty": "暂无历史对话",
    "settings.history.deleteGroup": "删除该分组的所有对话",
    "settings.history.messages": "{count} 条消息",
    "settings.history.open": "打开",
    "settings.history.archiveLabel": "归档",
    "settings.history.publicGroup": "公共记录",
    "settings.history.yesterday": "昨天",
    "settings.history.cancel": "取消",
    "settings.history.deleteLabel": "删除",

    "settings.historyConfirm.deleteTitle": "删除对话",
    "settings.historyConfirm.deleteSelectedDesc":
      "确定删除选中的 {count} 个对话吗？此操作不可撤销。",
    "settings.historyConfirm.deleteGroupTitle": "删除分组对话",
    "settings.historyConfirm.deleteGroupDesc":
      "确定删除\u201c{name}\u201d分组下的全部 {count} 个对话吗？此操作不可撤销。",
    "settings.historyConfirm.deleteSingleDesc": "确定删除这个对话吗？此操作不可撤销。",
    "settings.historyConfirm.clearTitle": "清空历史记录",
    "settings.historyConfirm.clearDesc": "确定要清空所有历史对话吗？此操作不可逆！",
    "settings.historyConfirm.deleteAllLabel": "全部删除",

    "settings.archive.title": "归档",
    "settings.archive.description": "归档的对话和工作区会留在本机，直到你恢复或永久删除。",
    "settings.archive.conversations": "对话",
    "settings.archive.workspaces": "工作区",
    "settings.archive.emptyConversations": "暂无归档对话。",
    "settings.archive.emptyWorkspaces": "暂无归档工作区。",
    "settings.archive.noMatchingConversations": "没有匹配的归档对话。",
    "settings.archive.noMatchingWorkspaces": "没有匹配的归档工作区。",
    "settings.archive.search": "搜索归档的对话和工作区",
    "settings.archive.searchSubmit": "搜索",
    "settings.archive.searchClear": "清除",
    "settings.archive.selectAll": "全选",
    "settings.archive.restoreSelected": "恢复选中 ({count})",
    "settings.archive.deleteSelected": "删除选中 ({count})",
    "settings.archive.restore": "恢复",
    "settings.archive.delete": "删除",
    "settings.archive.deleteConversation": "永久删除这个对话？此操作无法撤销。",
    "settings.archive.deleteWorkspace": "永久删除工作区“{name}”？项目文件不会被删除。",
    "settings.archive.deleteSelectedConversations":
      "永久删除选中的 {count} 个对话？此操作无法撤销。",
    "settings.archive.deleteSelectedWorkspaces":
      "永久删除选中的 {count} 个工作区？项目文件不会被删除。",
    "settings.archive.deleteSelectedMixed":
      "永久删除 {sessionCount} 个对话和 {workspaceCount} 个工作区？项目文件不会被删除。",
  },
  "ja-JP": {
    "settings.title": "設定",
    "settings.minimize": "最小化",
    "settings.close": "閉じる",
    "settings.sidebarLabel": "カテゴリー",
    "settings.searchPlaceholder": "設定を検索",
    "settings.empty": "一致する設定がありません",

    "settings.categories.appearance": "外観",
    "settings.categories.ai": "モデル",
    "settings.categories.image": "画像生成",
    "settings.categories.memory": "記憶",
    "settings.categories.search": "検索",
    "settings.categories.agent": "代理",
    "settings.categories.mcp": "MCP",
    "settings.categories.skills": "技能",
    "settings.categories.workspace": "ワークスペース",
    "settings.categories.history": "履歴",
    "settings.categories.archive": "アーカイブ",
    "settings.categories.about": "情報",
    "settings.categories.provider": "プロバイダー",
    "settings.provider.deepseek": "DeepSeek プロバイダー",
    "settings.provider.custom": "カスタムプロバイダー",
    "settings.provider.title": "プロバイダー設定",
    "settings.provider.description":
      "DeepSeek API またはカスタムの OpenAI 互換プロバイダーを設定します。",
    "settings.provider.apiKey": "API キー",
    "settings.provider.getApiKey": "取得先：",
    "settings.provider.baseUrl": "ベース URL",
    "settings.provider.modelsList": "モデルリスト",
    "settings.provider.modelsPlaceholder": "モデルID（例: gpt-4o）",
    "settings.provider.addModel": "追加",
    "settings.provider.modelsEmpty": "モデルなし",
    "settings.provider.removeModel": "モデルを削除",
    "settings.provider.enableModel": "モデルを有効化",
    "settings.provider.disableModel": "モデルを無効化",
    "settings.provider.configured": "設定済み",
    "settings.provider.notConfigured": "未設定",
    "settings.provider.presets": "クイック追加",
    "settings.provider.presetHint":
      "ベンダーを選ぶと Base URL とモデル候補が入ります。API Key を入力してください。",
    "settings.provider.addBlank": "空白",
    "settings.provider.modelsHint": "Enter または追加。カンマ区切りで複数貼り付け可。",
    "settings.provider.urlPlaceholder": "https://api.example.com/v1",
    "settings.provider.save": "設定を保存",
    "settings.provider.saved": "設定を保存しました",
    "settings.provider.back": "戻る",
    "settings.provider.name": "プロバイダー名",
    "settings.provider.namePlaceholder": "例: Ollama, OpenRouter",
    "settings.provider.add": "追加",
    "settings.provider.delete": "プロバイダーを削除",
    "settings.provider.deleteConfirm": "このプロバイダーを削除しますか？",

    "settings.groups.themeLanguage": "テーマと言語",
    "settings.groups.window": "ウィンドウ",
    "settings.groups.hotkeys": "ショートカット",
    "settings.groups.performance": "パフォーマンス",
    "settings.groups.modelSelection": "モデル",
    "settings.groups.imageGeneration": "モデル",
    "settings.groups.context": "コンテキスト",
    "settings.groups.reasoning": "推論",
    "settings.groups.memory": "メモリ",
    "settings.groups.mem0": "mem0",
    "settings.groups.webSearch": "ウェブ検索",
    "settings.groups.searchKeys": "API キー",
    "settings.groups.agentSafety": "安全",
    "settings.groups.agentDisplay": "表示",
    "settings.groups.agentCapabilities": "機能",
    "settings.groups.plugins": "バッジ",
    "settings.groups.about": "アプリケーション",

    "settings.fields.colorScheme.title": "カラーテーマ",
    "settings.fields.colorScheme.description": "アプリ全体の配色を選択します。",
    "settings.fields.language.title": "言語",
    "settings.fields.language.description": "インターフェースの表示言語を選択します。",
    "settings.fields.zoom.title": "表示倍率",
    "settings.fields.zoom.description": "画面要素とフォントの大きさを調整します。",
    "settings.fields.opacity.title": "不透明度",
    "settings.fields.opacity.description": "ウィンドウの透明度とすりガラス効果を調整します。",
    "settings.fields.deepseekApiKey.title": "API キー",
    "settings.fields.deepseekApiKey.description":
      "DeepSeek のチャットリクエストに使用し、ローカルにのみ保存されます。取得先：https://platform.deepseek.com/api_keys",
    "settings.fields.defaultModel.title": "既定のモデル",
    "settings.fields.defaultModel.description":
      "新しいチャットで使用するモデルです。一覧は DeepSeek API から取得します。",
    "settings.fields.multimodalModel.title": "マルチモーダルモデル",
    "settings.fields.multimodalModel.description":
      "画像などのビジョン入力を含む場合に自動的に使用されるモデルです（例: gpt-4o）。",
    "settings.fields.imageModel.title": "画像生成モデル",
    "settings.fields.imageModel.description":
      "generate_image が使うモデルです（既定 gpt-image-2）。Base URL は Images API（公式例: https://api.openai.com/v1）にしてください。",
    "settings.fields.reasoningEffort.title": "推論の強度",
    "settings.fields.reasoningEffort.description":
      "優先する思考の深さ。入力バーはモデル公式の段階名を表示し、非対応値は自動で割り当てます。",
    "settings.fields.reasoningLanguage.title": "回答言語",
    "settings.fields.reasoningLanguage.description": "推論テキストと最終回答の言語設定です。",
    "settings.fields.passToolReasoning.title": "ツール回合の推論を返す",
    "settings.fields.passToolReasoning.description":
      "tool_calls 付きの assistant ターンで reasoning_content を API に戻します（Thinking + ツールで必須。オフにすると 400 になる場合があります）。",
    "settings.fields.continueThinkingAfterTools.title": "ツール後も思考を続ける",
    "settings.fields.continueThinkingAfterTools.description":
      "ツール実行後の各ラウンドでも思考を続けます（既定でオン）。オフにすると後続ラウンドの思考を省略してトークンを節約します。",
    "settings.fields.memoryEnabled.title": "メモリを有効化",
    "settings.fields.memoryEnabled.description":
      "関連するメモリを呼び出し、長期的な設定やプロジェクトルールを保存します。",
    "settings.fields.mem0ApiKey.title": "mem0 API キー",
    "settings.fields.mem0ApiKey.description":
      "mem0 への接続に使用します。空欄の場合はローカルメモリを使用します。取得先：https://app.mem0.ai/dashboard/api-keys",
    "settings.fields.mem0UserId.title": "ユーザー ID",
    "settings.fields.mem0UserId.description": "ユーザーごとのメモリを分離する安定した識別子です。",
    "settings.fields.mem0BaseUrl.title": "ベース URL",
    "settings.fields.mem0BaseUrl.description":
      "互換サービスまたはセルフホストゲートウェイの mem0 API URL です。",
    "settings.fields.webSearchEnabled.title": "ウェブ検索を有効化",
    "settings.fields.webSearchEnabled.description":
      "有効化し、選択したプロバイダーの API キーがある場合に web_search をモデルへ提供します。",
    "settings.fields.webSearchProvider.title": "検索プロバイダー",
    "settings.fields.webSearchProvider.description": "Serper または Tavily を選択します。",
    "settings.fields.serperApiKey.title": "Serper API キー",
    "settings.fields.serperApiKey.description":
      "Serper 検索用。ローカルにのみ保存されます。取得先：https://serper.dev",
    "settings.fields.tavilyApiKey.title": "Tavily API キー",
    "settings.fields.tavilyApiKey.description":
      "Tavily 検索用。ローカルにのみ保存されます。取得先：https://app.tavily.com",
    "settings.fields.toolApprovalMode.title": "ツール承認",
    "settings.fields.toolApprovalMode.description":
      "Ask / Auto / 常に許可。危険な shell は引き続きブロックされます。",
    "settings.fields.lspEnabled.title": "LSP を有効化",
    "settings.fields.lspEnabled.description":
      "言語サーバーが利用可能な場合に診断と定義ジャンプを提供します。",
    "settings.fields.appName.title": "アプリケーション名",
    "settings.fields.appName.description": "インストールされているアプリケーションの名前です。",
    "settings.fields.appVersion.title": "バージョン",
    "settings.fields.appVersion.description": "現在のアプリケーションバージョンです。",
    "settings.fields.appIdentifier.title": "識別子",
    "settings.fields.appIdentifier.description": "アプリケーション固有の識別子です。",

    "settings.history.search": "チャット履歴を検索...",
    "settings.history.title": "チャット履歴",
    "settings.history.selectAll": "すべて選択",
    "settings.history.deleteSelected": "選択項目を削除 ({count})",
    "settings.history.clearAll": "すべて消去",
    "settings.history.empty": "履歴がありません",
    "settings.history.deleteGroup": "このグループのチャットをすべて削除",
    "settings.history.messages": "{count} 件のメッセージ",
    "settings.history.open": "開く",
    "settings.history.publicGroup": "パブリック",
    "settings.history.yesterday": "昨日",
    "settings.history.cancel": "キャンセル",
    "settings.history.deleteLabel": "削除",

    "settings.historyConfirm.deleteTitle": "チャットを削除",
    "settings.historyConfirm.deleteSelectedDesc":
      "選択した {count} 件のチャットを削除しますか？元に戻せません。",
    "settings.historyConfirm.deleteGroupTitle": "グループのチャットを削除",
    "settings.historyConfirm.deleteGroupDesc":
      "「{name}」の {count} 件のチャットをすべて削除しますか？元に戻せません。",
    "settings.historyConfirm.deleteSingleDesc": "このチャットを削除しますか？元に戻せません。",
    "settings.historyConfirm.clearTitle": "履歴を消去",
    "settings.historyConfirm.clearDesc": "すべてのチャット履歴を消去しますか？元に戻せません。",
    "settings.historyConfirm.deleteAllLabel": "すべて削除",
  },
  "ru-RU": {
    "settings.title": "Настройки",
    "settings.minimize": "Свернуть",
    "settings.close": "Закрыть",
    "settings.sidebarLabel": "Категории",
    "settings.searchPlaceholder": "Поиск настроек",
    "settings.empty": "Настройки не найдены",

    "settings.categories.appearance": "Тема",
    "settings.categories.ai": "Модель",
    "settings.categories.image": "Изображения",
    "settings.categories.memory": "Память",
    "settings.categories.search": "Поиск",
    "settings.categories.agent": "Агент",
    "settings.categories.mcp": "MCP",
    "settings.categories.skills": "Скиллы",
    "settings.categories.workspace": "Рабочая область",
    "settings.categories.history": "История",
    "settings.categories.archive": "Архив",
    "settings.categories.about": "О программе",
    "settings.categories.provider": "Провайдер",
    "settings.provider.deepseek": "Провайдер DeepSeek",
    "settings.provider.custom": "Пользовательский провайдер",
    "settings.provider.title": "Настройки провайдеров",
    "settings.provider.description":
      "Настройте DeepSeek API или другие OpenAI-совместимые провайдеры.",
    "settings.provider.apiKey": "API ключ",
    "settings.provider.getApiKey": "Получить ключ:",
    "settings.provider.baseUrl": "Базовый URL",
    "settings.provider.modelsList": "Список моделей",
    "settings.provider.modelsPlaceholder": "ID модели, напр. gpt-4o",
    "settings.provider.addModel": "Добавить",
    "settings.provider.modelsEmpty": "Моделей пока нет",
    "settings.provider.removeModel": "Удалить модель",
    "settings.provider.enableModel": "Включить модель",
    "settings.provider.disableModel": "Отключить модель",
    "settings.provider.configured": "Настроен",
    "settings.provider.notConfigured": "Не настроен",
    "settings.provider.presets": "Быстрое добавление",
    "settings.provider.presetHint":
      "Выберите вендора, чтобы заполнить Base URL и модели, затем вставьте API-ключ.",
    "settings.provider.addBlank": "Пустой",
    "settings.provider.modelsHint":
      "Enter или Добавить. Можно вставить несколько ID через запятую.",
    "settings.provider.urlPlaceholder": "https://api.example.com/v1",
    "settings.provider.save": "Сохранить настройки",
    "settings.provider.saved": "Настройки успешно сохранены",
    "settings.provider.back": "Назад",
    "settings.provider.name": "Имя провайдера",
    "settings.provider.namePlaceholder": "например, Ollama, OpenRouter",
    "settings.provider.add": "Добавить",
    "settings.provider.delete": "Удалить провайдера",
    "settings.provider.deleteConfirm": "Удалить этого провайдера?",

    "settings.groups.themeLanguage": "Тема и язык",
    "settings.groups.window": "Окно",
    "settings.groups.hotkeys": "Ярлыки",
    "settings.groups.performance": "Производительность",
    "settings.groups.modelSelection": "Модели",
    "settings.groups.imageGeneration": "Модель",
    "settings.groups.context": "Контекст",
    "settings.groups.reasoning": "Рассуждения",
    "settings.groups.memory": "Память",
    "settings.groups.mem0": "mem0",
    "settings.groups.webSearch": "Веб-поиск",
    "settings.groups.searchKeys": "API-ключи",
    "settings.groups.agentSafety": "Безопасность",
    "settings.groups.agentDisplay": "Отображение",
    "settings.groups.agentCapabilities": "Возможности",
    "settings.groups.plugins": "Значки",
    "settings.groups.about": "Приложение",

    "settings.fields.colorScheme.title": "Цветовая схема",
    "settings.fields.colorScheme.description": "Выберите общую цветовую тему приложения.",
    "settings.fields.language.title": "Язык",
    "settings.fields.language.description": "Выберите язык интерфейса.",
    "settings.fields.zoom.title": "Масштаб интерфейса",
    "settings.fields.zoom.description": "Настройте размер элементов и шрифтов.",
    "settings.fields.opacity.title": "Прозрачность",
    "settings.fields.opacity.description": "Настройте прозрачность окна и эффект матового стекла.",
    "settings.fields.deepseekApiKey.title": "Ключ API",
    "settings.fields.deepseekApiKey.description":
      "Используется для запросов DeepSeek и хранится только локально. Получить ключ: https://platform.deepseek.com/api_keys",
    "settings.fields.defaultModel.title": "Модель по умолчанию",
    "settings.fields.defaultModel.description":
      "Модель для новых чатов. Список загружается через API DeepSeek.",
    "settings.fields.multimodalModel.title": "Мультимодальная модель",
    "settings.fields.multimodalModel.description":
      "Модель, автоматически используемая при обработке изображений или мультимедиа (например, gpt-4o).",
    "settings.fields.imageModel.title": "Модель генерации изображений",
    "settings.fields.imageModel.description":
      "Используется generate_image (по умолчанию gpt-image-2). Base URL — Images API, например https://api.openai.com/v1, не чат-хост.",
    "settings.fields.reasoningEffort.title": "Глубина рассуждений",
    "settings.fields.reasoningEffort.description":
      "Предпочтительная глубина рассуждений. В чате показываются официальные уровни текущей модели.",
    "settings.fields.reasoningLanguage.title": "Язык ответа",
    "settings.fields.reasoningLanguage.description": "Язык видимых рассуждений и итогового ответа.",
    "settings.fields.passToolReasoning.title": "Возврат reasoning в tool-ходах",
    "settings.fields.passToolReasoning.description":
      "Передавать reasoning_content для assistant-ходов с tool_calls (нужно для DeepSeek thinking + tools; выключение может дать 400).",
    "settings.fields.continueThinkingAfterTools.title": "Продолжать рассуждения после tools",
    "settings.fields.continueThinkingAfterTools.description":
      "Сохранять рассуждения в каждом раунде после tools (по умолчанию вкл.). Выключите, чтобы пропускать рассуждения и экономить токены.",
    "settings.fields.memoryEnabled.title": "Включить память",
    "settings.fields.memoryEnabled.description":
      "Находит связанную память и сохраняет долгосрочные предпочтения и правила проекта.",
    "settings.fields.mem0ApiKey.title": "Ключ API mem0",
    "settings.fields.mem0ApiKey.description":
      "Подключает mem0. Оставьте пустым для локальной памяти. Получить ключ: https://app.mem0.ai/dashboard/api-keys",
    "settings.fields.mem0UserId.title": "ID пользователя",
    "settings.fields.mem0UserId.description":
      "Постоянный идентификатор для разделения памяти пользователей.",
    "settings.fields.mem0BaseUrl.title": "Базовый URL",
    "settings.fields.mem0BaseUrl.description":
      "Адрес API mem0 для совместимого сервиса или собственного шлюза.",
    "settings.fields.webSearchEnabled.title": "Включить веб-поиск",
    "settings.fields.webSearchEnabled.description":
      "При включении и наличии ключа выбранного провайдера предоставляет модели web_search.",
    "settings.fields.webSearchProvider.title": "Провайдер поиска",
    "settings.fields.webSearchProvider.description": "Выберите Serper или Tavily.",
    "settings.fields.serperApiKey.title": "Ключ API Serper",
    "settings.fields.serperApiKey.description":
      "Для поиска Serper. Хранится только локально. Получить ключ: https://serper.dev",
    "settings.fields.tavilyApiKey.title": "Ключ API Tavily",
    "settings.fields.tavilyApiKey.description":
      "Для поиска Tavily. Хранится только локально. Получить ключ: https://app.tavily.com",
    "settings.fields.toolApprovalMode.title": "Одобрение инструментов",
    "settings.fields.toolApprovalMode.description":
      "Ask / Auto / Always allow. Опасный shell всё ещё блокируется.",
    "settings.fields.lspEnabled.title": "Включить LSP",
    "settings.fields.lspEnabled.description":
      "Диагностика и переход к определению при доступных серверах.",
    "settings.fields.appName.title": "Название приложения",
    "settings.fields.appName.description": "Название установленного приложения.",
    "settings.fields.appVersion.title": "Версия",
    "settings.fields.appVersion.description": "Текущая версия приложения.",
    "settings.fields.appIdentifier.title": "Идентификатор",
    "settings.fields.appIdentifier.description": "Уникальный идентификатор приложения.",

    "settings.history.search": "Поиск в истории...",
    "settings.history.title": "История чатов",
    "settings.history.selectAll": "Выбрать все",
    "settings.history.deleteSelected": "Удалить выбранные ({count})",
    "settings.history.clearAll": "Очистить все",
    "settings.history.empty": "История пуста",
    "settings.history.deleteGroup": "Удалить все чаты группы",
    "settings.history.messages": "Сообщений: {count}",
    "settings.history.open": "Открыть",
    "settings.history.publicGroup": "Общие",
    "settings.history.yesterday": "Вчера",
    "settings.history.cancel": "Отмена",
    "settings.history.deleteLabel": "Удалить",

    "settings.historyConfirm.deleteTitle": "Удалить чаты",
    "settings.historyConfirm.deleteSelectedDesc":
      "Удалить выбранные чаты ({count})? Это действие нельзя отменить.",
    "settings.historyConfirm.deleteGroupTitle": "Удалить чаты группы",
    "settings.historyConfirm.deleteGroupDesc":
      "Удалить все чаты ({count}) в группе «{name}»? Это действие нельзя отменить.",
    "settings.historyConfirm.deleteSingleDesc": "Удалить этот чат? Это действие нельзя отменить.",
    "settings.historyConfirm.clearTitle": "Очистить историю",
    "settings.historyConfirm.clearDesc":
      "Очистить всю историю чатов? Это действие нельзя отменить.",
    "settings.historyConfirm.deleteAllLabel": "Удалить все",
  },
  "de-DE": {
    "settings.title": "Einstellungen",
    "settings.minimize": "Minimieren",
    "settings.close": "Schließen",
    "settings.sidebarLabel": "Kategorien",
    "settings.searchPlaceholder": "Einstellungen durchsuchen",
    "settings.empty": "Keine passenden Einstellungen",

    "settings.categories.appearance": "Design",
    "settings.categories.ai": "Modell",
    "settings.categories.image": "Bilder",
    "settings.categories.memory": "Gedächtnis",
    "settings.categories.search": "Suche",
    "settings.categories.agent": "Agent",
    "settings.categories.mcp": "MCP",
    "settings.categories.skills": "Skills",
    "settings.categories.workspace": "Arbeitsbereich",
    "settings.categories.history": "Verlauf",
    "settings.categories.archive": "Archiv",
    "settings.categories.about": "Info",
    "settings.categories.provider": "Anbieter",
    "settings.provider.deepseek": "DeepSeek-Anbieter",
    "settings.provider.custom": "Benutzerdefinierter Anbieter",
    "settings.provider.title": "Anbieterkonfigurationen",
    "settings.provider.description":
      "Konfigurieren Sie die DeepSeek-API oder benutzerdefinierte OpenAI-kompatible Anbieter.",
    "settings.provider.apiKey": "API-Schlüssel",
    "settings.provider.getApiKey": "Key unter:",
    "settings.provider.baseUrl": "Basis-URL",
    "settings.provider.modelsList": "Modellliste",
    "settings.provider.modelsPlaceholder": "Modell-ID, z.B. gpt-4o",
    "settings.provider.addModel": "Hinzufügen",
    "settings.provider.modelsEmpty": "Noch keine Modelle",
    "settings.provider.removeModel": "Modell entfernen",
    "settings.provider.enableModel": "Modell aktivieren",
    "settings.provider.disableModel": "Modell deaktivieren",
    "settings.provider.configured": "Konfiguriert",
    "settings.provider.notConfigured": "Nicht konfiguriert",
    "settings.provider.presets": "Schnell hinzufügen",
    "settings.provider.presetHint":
      "Anbieter wählen, um Base URL und Modelle vorzufüllen — dann API-Key einfügen.",
    "settings.provider.addBlank": "Leer",
    "settings.provider.modelsHint": "Enter oder Hinzufügen. Mehrere IDs per Komma einfügen.",
    "settings.provider.urlPlaceholder": "https://api.example.com/v1",
    "settings.provider.save": "Einstellungen speichern",
    "settings.provider.saved": "Einstellungen gespeichert",
    "settings.provider.back": "Zurück",
    "settings.provider.name": "Anbietername",
    "settings.provider.namePlaceholder": "z.B. Ollama, OpenRouter",
    "settings.provider.add": "Hinzufügen",
    "settings.provider.delete": "Anbieter löschen",
    "settings.provider.deleteConfirm": "Diesen Anbieter wirklich löschen?",

    "settings.groups.themeLanguage": "Thema & Sprache",
    "settings.groups.window": "Fenster",
    "settings.groups.hotkeys": "Tastenkürzel",
    "settings.groups.performance": "Leistung",
    "settings.groups.modelSelection": "Modelle",
    "settings.groups.imageGeneration": "Modell",
    "settings.groups.context": "Kontext",
    "settings.groups.reasoning": "Reasoning",
    "settings.groups.memory": "Gedächtnis",
    "settings.groups.mem0": "mem0",
    "settings.groups.webSearch": "Websuche",
    "settings.groups.searchKeys": "API-Schlüssel",
    "settings.groups.agentSafety": "Sicherheit",
    "settings.groups.agentDisplay": "Anzeige",
    "settings.groups.agentCapabilities": "Funktionen",
    "settings.groups.plugins": "Badges",
    "settings.groups.about": "Anwendung",

    "settings.fields.colorScheme.title": "Farbschema",
    "settings.fields.colorScheme.description":
      "Wählen Sie das allgemeine Farbschema der Anwendung.",
    "settings.fields.language.title": "Sprache",
    "settings.fields.language.description": "Wählen Sie die Anzeigesprache der Oberfläche.",
    "settings.fields.zoom.title": "Oberflächenskalierung",
    "settings.fields.zoom.description": "Passen Sie die Größe von Elementen und Schrift an.",
    "settings.fields.opacity.title": "Deckkraft",
    "settings.fields.opacity.description": "Passen Sie Fenstertransparenz und Milchglaseffekt an.",
    "settings.fields.deepseekApiKey.title": "API-Schlüssel",
    "settings.fields.deepseekApiKey.description":
      "Wird für DeepSeek-Anfragen verwendet und nur lokal gespeichert. Key unter: https://platform.deepseek.com/api_keys",
    "settings.fields.defaultModel.title": "Standardmodell",
    "settings.fields.defaultModel.description":
      "Modell für neue Chats. Die Liste wird über die DeepSeek-API geladen.",
    "settings.fields.multimodalModel.title": "Multimodales Modell",
    "settings.fields.multimodalModel.description":
      "Das Modell, das automatisch bei Bild- oder multimodalen Eingaben verwendet wird (z. B. gpt-4o).",
    "settings.fields.imageModel.title": "Bildmodell",
    "settings.fields.imageModel.description":
      "Wird von generate_image genutzt (Standard gpt-image-2). Base URL muss die Images API sein, z. B. https://api.openai.com/v1 — nicht der Chat-Host.",
    "settings.fields.reasoningEffort.title": "Denkintensität",
    "settings.fields.reasoningEffort.description":
      "Bevorzugte Denktiefe. Die Eingabeleiste zeigt die offiziellen Stufen des aktuellen Modells.",
    "settings.fields.reasoningLanguage.title": "Antwortsprache",
    "settings.fields.reasoningLanguage.description":
      "Sprache für sichtbare Gedankengänge und endgültige Antworten.",
    "settings.fields.passToolReasoning.title": "Tool-Runden-Reasoning zurückgeben",
    "settings.fields.passToolReasoning.description":
      "reasoning_content bei Assistant-Turns mit tool_calls mitsenden (für DeepSeek Thinking + Tools nötig; Aus kann zu 400 führen).",
    "settings.fields.continueThinkingAfterTools.title": "Weiterdenken nach Tools",
    "settings.fields.continueThinkingAfterTools.description":
      "Thinking in jeder Agent-Runde nach Tools belassen (Standard an). Ausschalten spart Tokens, indem spaetere Runden ohne Thinking laufen.",
    "settings.fields.memoryEnabled.title": "Speicher aktivieren",
    "settings.fields.memoryEnabled.description":
      "Ruft relevante Erinnerungen ab und speichert dauerhafte Einstellungen und Projektregeln.",
    "settings.fields.mem0ApiKey.title": "mem0-API-Schlüssel",
    "settings.fields.mem0ApiKey.description":
      "Verbindet mem0. Leer lassen, um lokalen Speicher zu verwenden. Key unter: https://app.mem0.ai/dashboard/api-keys",
    "settings.fields.mem0UserId.title": "Benutzer-ID",
    "settings.fields.mem0UserId.description":
      "Stabile Kennung zur Trennung der Erinnerungen verschiedener Benutzer.",
    "settings.fields.mem0BaseUrl.title": "Basis-URL",
    "settings.fields.mem0BaseUrl.description":
      "mem0-API-Adresse für kompatible Dienste oder ein selbst gehostetes Gateway.",
    "settings.fields.webSearchEnabled.title": "Websuche aktivieren",
    "settings.fields.webSearchEnabled.description":
      "Stellt web_search dem Modell bereit, wenn aktiviert und der API-Schlüssel des gewählten Anbieters gesetzt ist.",
    "settings.fields.webSearchProvider.title": "Suchanbieter",
    "settings.fields.webSearchProvider.description": "Wählen Sie Serper oder Tavily.",
    "settings.fields.serperApiKey.title": "Serper-API-Schlüssel",
    "settings.fields.serperApiKey.description":
      "Für Serper-Suche. Nur lokal gespeichert. Key unter: https://serper.dev",
    "settings.fields.tavilyApiKey.title": "Tavily-API-Schlüssel",
    "settings.fields.tavilyApiKey.description":
      "Für Tavily-Suche. Nur lokal gespeichert. Key unter: https://app.tavily.com",
    "settings.fields.toolApprovalMode.title": "Werkzeugfreigabe",
    "settings.fields.toolApprovalMode.description":
      "Ask / Auto / Always allow. Gefährliche Shells bleiben blockiert.",
    "settings.fields.lspEnabled.title": "LSP aktivieren",
    "settings.fields.lspEnabled.description":
      "Diagnosen und Definitionssprünge bei verfügbaren Sprachservern.",
    "settings.fields.appName.title": "Anwendungsname",
    "settings.fields.appName.description": "Name der installierten Anwendung.",
    "settings.fields.appVersion.title": "Version",
    "settings.fields.appVersion.description": "Aktuelle Anwendungsversion.",
    "settings.fields.appIdentifier.title": "Kennung",
    "settings.fields.appIdentifier.description": "Eindeutige Anwendungskennung.",

    "settings.history.search": "Chatverlauf durchsuchen...",
    "settings.history.title": "Chatverlauf",
    "settings.history.selectAll": "Alle auswählen",
    "settings.history.deleteSelected": "Auswahl löschen ({count})",
    "settings.history.clearAll": "Alles löschen",
    "settings.history.empty": "Kein Verlauf gefunden",
    "settings.history.deleteGroup": "Alle Chats dieser Gruppe löschen",
    "settings.history.messages": "{count} Nachrichten",
    "settings.history.open": "Öffnen",
    "settings.history.publicGroup": "Öffentlich",
    "settings.history.yesterday": "Gestern",
    "settings.history.cancel": "Abbrechen",
    "settings.history.deleteLabel": "Löschen",

    "settings.historyConfirm.deleteTitle": "Chats löschen",
    "settings.historyConfirm.deleteSelectedDesc":
      "Die ausgewählten {count} Chats löschen? Dies kann nicht rückgängig gemacht werden.",
    "settings.historyConfirm.deleteGroupTitle": "Gruppenchats löschen",
    "settings.historyConfirm.deleteGroupDesc":
      "Alle {count} Chats in „{name}“ löschen? Dies kann nicht rückgängig gemacht werden.",
    "settings.historyConfirm.deleteSingleDesc":
      "Diesen Chat löschen? Dies kann nicht rückgängig gemacht werden.",
    "settings.historyConfirm.clearTitle": "Verlauf löschen",
    "settings.historyConfirm.clearDesc":
      "Den gesamten Chatverlauf löschen? Dies kann nicht rückgängig gemacht werden.",
    "settings.historyConfirm.deleteAllLabel": "Alle löschen",
  },
  "fr-FR": {
    "settings.title": "Paramètres",
    "settings.minimize": "Réduire",
    "settings.close": "Fermer",
    "settings.sidebarLabel": "Catégories",
    "settings.searchPlaceholder": "Rechercher dans les paramètres",
    "settings.empty": "Aucun paramètre correspondant",

    "settings.categories.appearance": "Thème",
    "settings.categories.ai": "Modèle",
    "settings.categories.image": "Images",
    "settings.categories.memory": "Mémoire",
    "settings.categories.search": "Recherche",
    "settings.categories.agent": "Agent",
    "settings.categories.mcp": "MCP",
    "settings.categories.skills": "Skills",
    "settings.categories.workspace": "Espace de travail",
    "settings.categories.history": "Historique",
    "settings.categories.archive": "Archive",
    "settings.categories.about": "À propos",
    "settings.categories.provider": "Fournisseur",
    "settings.provider.deepseek": "Fournisseur DeepSeek",
    "settings.provider.custom": "Fournisseur personnalisé",
    "settings.provider.title": "Configurations du fournisseur",
    "settings.provider.description":
      "Configurez l'API DeepSeek ou des fournisseurs personnalisés compatibles OpenAI.",
    "settings.provider.apiKey": "Clé API",
    "settings.provider.getApiKey": "Obtenir une clé :",
    "settings.provider.baseUrl": "URL de base",
    "settings.provider.modelsList": "Liste des modèles",
    "settings.provider.modelsPlaceholder": "ID du modèle, ex. gpt-4o",
    "settings.provider.addModel": "Ajouter",
    "settings.provider.modelsEmpty": "Aucun modèle",
    "settings.provider.removeModel": "Retirer le modèle",
    "settings.provider.enableModel": "Activer le modèle",
    "settings.provider.disableModel": "Désactiver le modèle",
    "settings.provider.configured": "Configuré",
    "settings.provider.notConfigured": "Non configuré",
    "settings.provider.presets": "Ajout rapide",
    "settings.provider.presetHint":
      "Choisissez un fournisseur pour préremplir l’URL et les modèles, puis collez la clé API.",
    "settings.provider.addBlank": "Vide",
    "settings.provider.modelsHint": "Entrée ou Ajouter. Plusieurs ID séparés par des virgules.",
    "settings.provider.urlPlaceholder": "https://api.example.com/v1",
    "settings.provider.save": "Enregistrer les paramètres",
    "settings.provider.saved": "Paramètres enregistrés",
    "settings.provider.back": "Retour",
    "settings.provider.name": "Nom du fournisseur",
    "settings.provider.namePlaceholder": "ex. Ollama, OpenRouter",
    "settings.provider.add": "Ajouter",
    "settings.provider.delete": "Supprimer le fournisseur",
    "settings.provider.deleteConfirm": "Supprimer ce fournisseur ?",

    "settings.groups.themeLanguage": "Thème et langue",
    "settings.groups.window": "Fenêtre",
    "settings.groups.hotkeys": "Raccourcis",
    "settings.groups.performance": "Performances",
    "settings.groups.modelSelection": "Modèles",
    "settings.groups.imageGeneration": "Modèle",
    "settings.groups.context": "Contexte",
    "settings.groups.reasoning": "Raisonnement",
    "settings.groups.memory": "Mémoire",
    "settings.groups.mem0": "mem0",
    "settings.groups.webSearch": "Recherche web",
    "settings.groups.searchKeys": "Clés API",
    "settings.groups.agentSafety": "Sécurité",
    "settings.groups.agentDisplay": "Affichage",
    "settings.groups.agentCapabilities": "Capacités",
    "settings.groups.plugins": "Badges",
    "settings.groups.about": "Application",

    "settings.fields.colorScheme.title": "Thème de couleurs",
    "settings.fields.colorScheme.description":
      "Choisissez le thème de couleurs général de l\u2019application.",
    "settings.fields.language.title": "Langue",
    "settings.fields.language.description":
      "Choisissez la langue d\u2019affichage de l\u2019interface.",
    "settings.fields.zoom.title": "Zoom de l\u2019interface",
    "settings.fields.zoom.description": "Ajustez la taille des éléments et des polices.",
    "settings.fields.opacity.title": "Opacité",
    "settings.fields.opacity.description":
      "Ajustez la transparence et l\u2019effet de verre dépoli.",
    "settings.fields.deepseekApiKey.title": "Clé API",
    "settings.fields.deepseekApiKey.description":
      "Utilisée pour les requêtes DeepSeek et stockée uniquement en local. Obtenir une clé : https://platform.deepseek.com/api_keys",
    "settings.fields.defaultModel.title": "Modèle par défaut",
    "settings.fields.defaultModel.description":
      "Modèle utilisé pour les nouvelles discussions. La liste vient de l\u2019API DeepSeek.",
    "settings.fields.multimodalModel.title": "Modèle multimodal",
    "settings.fields.multimodalModel.description":
      "Le modèle utilisé automatiquement pour les entrées visuelles ou multimodales (ex. gpt-4o).",
    "settings.fields.imageModel.title": "Modèle d’image",
    "settings.fields.imageModel.description":
      "Utilisé par generate_image (défaut gpt-image-2). L’URL de base doit être l’API Images, p. ex. https://api.openai.com/v1, pas l’hôte de chat.",
    "settings.fields.reasoningEffort.title": "Intensité du raisonnement",
    "settings.fields.reasoningEffort.description":
      "Profondeur de raisonnement préférée. La barre d'entrée affiche les niveaux officiels du modèle actuel.",
    "settings.fields.reasoningLanguage.title": "Langue de réponse",
    "settings.fields.reasoningLanguage.description":
      "Langue du raisonnement visible et de la réponse finale.",
    "settings.fields.passToolReasoning.title": "Renvoyer le raisonnement des tours d\u2019outils",
    "settings.fields.passToolReasoning.description":
      "Inclure reasoning_content sur les tours assistant avec tool_calls (requis pour DeepSeek thinking + tools ; le désactiver peut provoquer une erreur 400).",
    "settings.fields.continueThinkingAfterTools.title": "Continuer a reflechir apres les outils",
    "settings.fields.continueThinkingAfterTools.description":
      "Garder la reflexion active a chaque tour apres les outils (active par defaut). Desactiver pour economiser des tokens.",
    "settings.fields.memoryEnabled.title": "Activer la mémoire",
    "settings.fields.memoryEnabled.description":
      "Rappelle les souvenirs pertinents et enregistre les préférences durables et les règles du projet.",
    "settings.fields.mem0ApiKey.title": "Clé API mem0",
    "settings.fields.mem0ApiKey.description":
      "Connecte mem0. Laissez vide pour utiliser la mémoire locale. Obtenir une clé : https://app.mem0.ai/dashboard/api-keys",
    "settings.fields.mem0UserId.title": "Identifiant utilisateur",
    "settings.fields.mem0UserId.description":
      "Identifiant stable permettant d\u2019isoler la mémoire de chaque utilisateur.",
    "settings.fields.mem0BaseUrl.title": "URL de base",
    "settings.fields.mem0BaseUrl.description":
      "Adresse de l\u2019API mem0 pour un service compatible ou une passerelle auto-hébergée.",
    "settings.fields.webSearchEnabled.title": "Activer la recherche web",
    "settings.fields.webSearchEnabled.description":
      "Expose web_search au modèle lorsque c\u2019est activé avec une clé API du fournisseur sélectionné.",
    "settings.fields.webSearchProvider.title": "Fournisseur de recherche",
    "settings.fields.webSearchProvider.description": "Choisissez Serper ou Tavily.",
    "settings.fields.serperApiKey.title": "Clé API Serper",
    "settings.fields.serperApiKey.description":
      "Pour la recherche Serper. Stockée uniquement en local. Obtenir une clé : https://serper.dev",
    "settings.fields.tavilyApiKey.title": "Clé API Tavily",
    "settings.fields.tavilyApiKey.description":
      "Pour la recherche Tavily. Stockée uniquement en local. Obtenir une clé : https://app.tavily.com",
    "settings.fields.toolApprovalMode.title": "Approbation des outils",
    "settings.fields.toolApprovalMode.description":
      "Ask / Auto / Always allow. Les shells dangereux restent bloqués.",
    "settings.fields.lspEnabled.title": "Activer LSP",
    "settings.fields.lspEnabled.description":
      "Diagnostics et définition si des serveurs sont disponibles.",
    "settings.fields.appName.title": "Nom de l\u2019application",
    "settings.fields.appName.description": "Nom de l\u2019application installée.",
    "settings.fields.appVersion.title": "Version",
    "settings.fields.appVersion.description": "Version actuelle de l\u2019application.",
    "settings.fields.appIdentifier.title": "Identifiant",
    "settings.fields.appIdentifier.description": "Identifiant unique de l\u2019application.",

    "settings.history.search": "Rechercher dans l\u2019historique...",
    "settings.history.title": "Historique des discussions",
    "settings.history.selectAll": "Tout sélectionner",
    "settings.history.deleteSelected": "Supprimer la sélection ({count})",
    "settings.history.clearAll": "Tout effacer",
    "settings.history.empty": "Aucun historique",
    "settings.history.deleteGroup": "Supprimer toutes les discussions de ce groupe",
    "settings.history.messages": "{count} messages",
    "settings.history.open": "Ouvrir",
    "settings.history.publicGroup": "Public",
    "settings.history.yesterday": "Hier",
    "settings.history.cancel": "Annuler",
    "settings.history.deleteLabel": "Supprimer",

    "settings.historyConfirm.deleteTitle": "Supprimer les discussions",
    "settings.historyConfirm.deleteSelectedDesc":
      "Supprimer les {count} discussions sélectionnées ? Cette action est irréversible.",
    "settings.historyConfirm.deleteGroupTitle": "Supprimer les discussions du groupe",
    "settings.historyConfirm.deleteGroupDesc":
      "Supprimer les {count} discussions de « {name} » ? Cette action est irréversible.",
    "settings.historyConfirm.deleteSingleDesc":
      "Supprimer cette discussion ? Cette action est irréversible.",
    "settings.historyConfirm.clearTitle": "Effacer l\u2019historique",
    "settings.historyConfirm.clearDesc":
      "Effacer tout l\u2019historique ? Cette action est irréversible.",
    "settings.historyConfirm.deleteAllLabel": "Tout supprimer",
  },
  "ko-KR": {
    "settings.title": "설정",
    "settings.minimize": "최소화",
    "settings.close": "닫기",
    "settings.sidebarLabel": "카테고리",
    "settings.searchPlaceholder": "설정 검색",
    "settings.empty": "일치하는 설정이 없습니다",

    "settings.categories.appearance": "테마",
    "settings.categories.ai": "모델",
    "settings.categories.image": "이미지 생성",
    "settings.categories.memory": "기억",
    "settings.categories.search": "검색",
    "settings.categories.agent": "에이전트",
    "settings.categories.mcp": "MCP",
    "settings.categories.skills": "스킬",
    "settings.categories.workspace": "작업 영역",
    "settings.categories.history": "기록",
    "settings.categories.archive": "보관함",
    "settings.categories.about": "정보",
    "settings.categories.provider": "제공자",
    "settings.provider.deepseek": "DeepSeek 제공자",
    "settings.provider.custom": "사용자 정의 제공자",
    "settings.provider.title": "모델 제공자 설정",
    "settings.provider.description":
      "DeepSeek API 또는 사용자 정의 OpenAI 호환 제공자를 설정합니다.",
    "settings.provider.apiKey": "API 키",
    "settings.provider.getApiKey": "발급 주소:",
    "settings.provider.baseUrl": "기본 URL",
    "settings.provider.modelsList": "모델 목록",
    "settings.provider.modelsPlaceholder": "모델 ID, 예: gpt-4o",
    "settings.provider.addModel": "추가",
    "settings.provider.modelsEmpty": "모델 없음",
    "settings.provider.removeModel": "모델 제거",
    "settings.provider.enableModel": "모델 활성화",
    "settings.provider.disableModel": "모델 비활성화",
    "settings.provider.configured": "구성됨",
    "settings.provider.notConfigured": "미구성",
    "settings.provider.presets": "빠른 추가",
    "settings.provider.presetHint":
      "공급자를 선택하면 Base URL과 모델이 채워집니다. API Key를 입력하세요.",
    "settings.provider.addBlank": "빈 항목",
    "settings.provider.modelsHint": "Enter 또는 추가. 쉼표로 여러 ID를 붙여넣을 수 있습니다.",
    "settings.provider.urlPlaceholder": "https://api.example.com/v1",
    "settings.provider.save": "설정 저장",
    "settings.provider.saved": "설정이 저장되었습니다",
    "settings.provider.back": "이전",
    "settings.provider.name": "제공자 이름",
    "settings.provider.namePlaceholder": "예: Ollama, OpenRouter",
    "settings.provider.add": "추가",
    "settings.provider.delete": "제공자 삭제",
    "settings.provider.deleteConfirm": "이 제공자를 삭제할까요?",

    "settings.groups.themeLanguage": "테마와 언어",
    "settings.groups.window": "창",
    "settings.groups.hotkeys": "단축키",
    "settings.groups.performance": "성능",
    "settings.groups.modelSelection": "모델",
    "settings.groups.imageGeneration": "모델",
    "settings.groups.context": "컨텍스트",
    "settings.groups.reasoning": "추론",
    "settings.groups.memory": "메모리",
    "settings.groups.mem0": "mem0",
    "settings.groups.webSearch": "웹 검색",
    "settings.groups.searchKeys": "API 키",
    "settings.groups.agentSafety": "보안",
    "settings.groups.agentDisplay": "표시",
    "settings.groups.agentCapabilities": "기능",
    "settings.groups.plugins": "배지",
    "settings.groups.about": "애플리케이션",

    "settings.fields.colorScheme.title": "색상 테마",
    "settings.fields.colorScheme.description": "앱 전체의 색상 테마를 선택합니다.",
    "settings.fields.language.title": "언어",
    "settings.fields.language.description": "인터페이스 표시 언어를 선택합니다.",
    "settings.fields.zoom.title": "인터페이스 확대/축소",
    "settings.fields.zoom.description": "화면 요소와 글꼴 크기를 조정합니다.",
    "settings.fields.opacity.title": "불투명도",
    "settings.fields.opacity.description": "창 투명도와 반투명 유리 효과를 조정합니다.",
    "settings.fields.deepseekApiKey.title": "API 키",
    "settings.fields.deepseekApiKey.description":
      "DeepSeek 요청에 사용하며 로컬에만 저장됩니다. 발급 주소: https://platform.deepseek.com/api_keys",
    "settings.fields.defaultModel.title": "기본 모델",
    "settings.fields.defaultModel.description":
      "새 채팅에 사용할 모델입니다. 목록은 DeepSeek API에서 가져옵니다.",
    "settings.fields.multimodalModel.title": "멀티모달 모델",
    "settings.fields.multimodalModel.description":
      "이미지 등 비주얼 입력이 포함될 때 자동으로 사용되는 모델입니다 (예: gpt-4o).",
    "settings.fields.imageModel.title": "이미지 생성 모델",
    "settings.fields.imageModel.description":
      "generate_image가 사용합니다(기본 gpt-image-2). Base URL은 Images API여야 합니다(예: https://api.openai.com/v1). 채팅 호스트가 아닙니다.",
    "settings.fields.reasoningEffort.title": "추론 강도",
    "settings.fields.reasoningEffort.description":
      "선호하는 사고 깊이입니다. 입력줄은 현재 모델의 공식 단계를 보여 주고, 미지원 값은 자동으로 맞춥니다.",
    "settings.fields.reasoningLanguage.title": "응답 언어",
    "settings.fields.reasoningLanguage.description": "표시되는 추론과 최종 응답의 언어 설정입니다.",
    "settings.fields.passToolReasoning.title": "도구 턴 추론 회수",
    "settings.fields.passToolReasoning.description":
      "tool_calls가 있는 assistant 턴에 reasoning_content를 API로 다시 보냅니다(DeepSeek Thinking + 도구에 필요; 끄면 400이 날 수 있음).",
    "settings.fields.continueThinkingAfterTools.title": "도구 이후에도 계속 사고",
    "settings.fields.continueThinkingAfterTools.description":
      "도구 실행 후 매 라운드에서도 사고를 유지합니다(기본 켜짐). 끄면 이후 라운드 사고를 건너뛰어 토큰을 절약합니다.",
    "settings.fields.memoryEnabled.title": "메모리 사용",
    "settings.fields.memoryEnabled.description":
      "관련 메모리를 불러오고 장기 환경설정과 프로젝트 규칙을 저장합니다.",
    "settings.fields.mem0ApiKey.title": "mem0 API 키",
    "settings.fields.mem0ApiKey.description":
      "mem0 연결에 사용합니다. 비워 두면 로컬 메모리를 사용합니다. 발급 주소: https://app.mem0.ai/dashboard/api-keys",
    "settings.fields.mem0UserId.title": "사용자 ID",
    "settings.fields.mem0UserId.description": "사용자별 메모리를 분리하는 고정 식별자입니다.",
    "settings.fields.mem0BaseUrl.title": "기본 URL",
    "settings.fields.mem0BaseUrl.description":
      "호환 서비스 또는 자체 호스팅 게이트웨이의 mem0 API 주소입니다.",
    "settings.fields.webSearchEnabled.title": "웹 검색 사용",
    "settings.fields.webSearchEnabled.description":
      "사용 설정하고 선택한 제공자의 API 키가 있으면 모델에 web_search를 제공합니다.",
    "settings.fields.webSearchProvider.title": "검색 제공자",
    "settings.fields.webSearchProvider.description": "Serper 또는 Tavily를 선택합니다.",
    "settings.fields.serperApiKey.title": "Serper API 키",
    "settings.fields.serperApiKey.description":
      "Serper 검색용. 로컬에만 저장됩니다. 발급 주소: https://serper.dev",
    "settings.fields.tavilyApiKey.title": "Tavily API 키",
    "settings.fields.tavilyApiKey.description":
      "Tavily 검색용. 로컬에만 저장됩니다. 발급 주소: https://app.tavily.com",
    "settings.fields.toolApprovalMode.title": "도구 승인",
    "settings.fields.toolApprovalMode.description":
      "Ask / Auto / Always allow. 위험한 shell은 계속 차단됩니다.",
    "settings.fields.lspEnabled.title": "LSP 사용",
    "settings.fields.lspEnabled.description": "언어 서버가 있으면 진단과 정의 이동을 제공합니다.",
    "settings.fields.appName.title": "애플리케이션 이름",
    "settings.fields.appName.description": "설치된 애플리케이션의 이름입니다.",
    "settings.fields.appVersion.title": "버전",
    "settings.fields.appVersion.description": "현재 애플리케이션 버전입니다.",
    "settings.fields.appIdentifier.title": "식별자",
    "settings.fields.appIdentifier.description": "애플리케이션의 고유 식별자입니다.",

    "settings.history.search": "채팅 기록 검색...",
    "settings.history.title": "채팅 기록",
    "settings.history.selectAll": "모두 선택",
    "settings.history.deleteSelected": "선택 항목 삭제 ({count})",
    "settings.history.clearAll": "모두 지우기",
    "settings.history.empty": "기록이 없습니다",
    "settings.history.deleteGroup": "이 그룹의 모든 채팅 삭제",
    "settings.history.messages": "메시지 {count}개",
    "settings.history.open": "열기",
    "settings.history.publicGroup": "공개",
    "settings.history.yesterday": "어제",
    "settings.history.cancel": "취소",
    "settings.history.deleteLabel": "삭제",

    "settings.historyConfirm.deleteTitle": "채팅 삭제",
    "settings.historyConfirm.deleteSelectedDesc":
      "선택한 채팅 {count}개를 삭제할까요? 되돌릴 수 없습니다.",
    "settings.historyConfirm.deleteGroupTitle": "그룹 채팅 삭제",
    "settings.historyConfirm.deleteGroupDesc":
      "'{name}'의 채팅 {count}개를 모두 삭제할까요? 되돌릴 수 없습니다.",
    "settings.historyConfirm.deleteSingleDesc": "이 채팅을 삭제할까요? 되돌릴 수 없습니다.",
    "settings.historyConfirm.clearTitle": "기록 지우기",
    "settings.historyConfirm.clearDesc": "모든 채팅 기록을 지울까요? 되돌릴 수 없습니다.",
    "settings.historyConfirm.deleteAllLabel": "모두 삭제",
  },
};

// `path` breadcrumbs and search `keywords` were only ever authored for zh-CN/en-US;
// other languages fell back to the (translated) field title / the en-US keywords verbatim.
// Preserved here exactly as the original computed `t` in Settings/index.vue behaved.
const settingsFieldPaths: Record<"zh-CN" | "en-US", Partial<Record<SettingFieldId, string>>> = {
  "en-US": {
    colorScheme: "Appearance › Color Scheme",
    language: "Appearance › Language",
    zoom: "Appearance › Interface Zoom",
    hardwareAccelerationEnabled: "Appearance › Hardware acceleration",
    opacity: "Appearance › Opacity",
    chromeFrostedGlass: "Appearance › Frosted glass chrome",
    primaryHotkey: "Appearance › Primary shortcut",
    secondaryHotkey: "Appearance › Secondary shortcut",
    deepseekApiKey: "AI › DeepSeek › API Key",
    defaultModel: "AI / DeepSeek / Default Model",
    multimodalModel: "AI / DeepSeek / Multimodal Model",
    imageModel: "Image / Image model",
    multimodalSplitAnalysis: "AI / DeepSeek / Split Multimodal Analysis",
    largeContextEnabled: "AI / Context / 1M Context Window",
    reasoningEffort: "AI › Reasoning › Reasoning Effort",
    reasoningLanguage: "AI › DeepSeek › Reasoning Language",
    showReasoning: "AI › Chat › Reasoning Display",
    passToolReasoning: "AI › Reasoning › Tool History",
    continueThinkingAfterTools: "AI › Reasoning › Continue After Tools",
    memoryEnabled: "Memory / mem0 / Enabled",
    mem0ApiKey: "Memory / mem0 / API Key",
    mem0UserId: "Memory / mem0 / User ID",
    mem0BaseUrl: "Memory / mem0 / Base URL",
    webSearchEnabled: "Web Search / Enabled",
    webSearchProvider: "Web Search / Provider",
    serperApiKey: "Web Search / Serper API Key",
    tavilyApiKey: "Web Search / Tavily API Key",
    semanticSearchEnabled: "Search / Semantic Workspace Search",
    semanticSearchModel: "Search / Embedding Model",
    toolApprovalMode: "Agent / Tool Approval",
    agentWorkDisplay: "Agent / Work Display",
    lspEnabled: "Agent / Language Server",
    multiModelCollaboration: "Agent / Multi-model collaboration",
    minimalCoding: "Agent / Minimal coding mode",
    pixpinPinAiEnabled: "Plugins / PixPin pin AI",
    snipastePinAiEnabled: "Plugins / Snipaste pin AI",
    appName: "About › Application Name",
    appVersion: "About › Version",
    appIdentifier: "About › Identifier",
  },
  "zh-CN": {
    colorScheme: "Appearance › Color Scheme",
    language: "Appearance › Language",
    zoom: "Appearance › Interface Zoom",
    hardwareAccelerationEnabled: "外观 › 硬件加速",
    opacity: "Appearance › Opacity",
    chromeFrostedGlass: "外观 › 毛玻璃顶栏与侧栏",
    primaryHotkey: "外观 › 主快捷键",
    secondaryHotkey: "外观 › 副快捷键",
    deepseekApiKey: "AI › DeepSeek › API Key",
    defaultModel: "AI / DeepSeek / 默认模型",
    multimodalModel: "AI / DeepSeek / 多模态模型",
    imageModel: "生图 / 生图模型",
    multimodalSplitAnalysis: "AI / DeepSeek / 多模态分步分析",
    largeContextEnabled: "AI / 上下文 / 1M 上下文",
    reasoningEffort: "AI › Reasoning › Reasoning Effort",
    reasoningLanguage: "AI › DeepSeek › Reasoning Language",
    showReasoning: "AI › 聊天 › 思考过程显示",
    passToolReasoning: "AI › 推理 › 工具历史",
    continueThinkingAfterTools: "AI › 推理 › 续轮思考",
    memoryEnabled: "记忆 / mem0 / 启用",
    mem0ApiKey: "记忆 / mem0 / API Key",
    mem0UserId: "记忆 / mem0 / User ID",
    mem0BaseUrl: "记忆 / mem0 / Base URL",
    webSearchEnabled: "联网搜索 / 启用",
    webSearchProvider: "联网搜索 / Provider",
    serperApiKey: "联网搜索 / Serper API Key",
    tavilyApiKey: "联网搜索 / Tavily API Key",
    semanticSearchEnabled: "搜索 / 语义工作区检索",
    semanticSearchModel: "搜索 / 嵌入模型",
    toolApprovalMode: "Agent / 工具审批",
    agentWorkDisplay: "Agent / 工作过程显示",
    lspEnabled: "Agent / 语言服务",
    multiModelCollaboration: "Agent / 多模型协同",
    minimalCoding: "Agent / 精简编码模式",
    pixpinPinAiEnabled: "插件 / PixPin 贴图 AI",
    snipastePinAiEnabled: "插件 / Snipaste 贴图 AI",
    appName: "About › Application Name",
    appVersion: "About › Version",
    appIdentifier: "About › Identifier",
  },
};

const settingsFieldKeywords: Record<
  "zh-CN" | "en-US",
  Partial<Record<SettingFieldId, string[]>>
> = {
  "en-US": {
    colorScheme: ["theme", "color", "scheme"],
    language: ["language", "locale"],
    zoom: ["zoom", "scale", "size", "font"],
    hardwareAccelerationEnabled: [
      "gpu",
      "hardware",
      "acceleration",
      "webview2",
      "rendering",
      "restart",
    ],
    opacity: ["opacity", "transparent", "glass", "blur", "acrylic", "appearance"],
    chromeFrostedGlass: [
      "frosted",
      "glass",
      "blur",
      "acrylic",
      "mica",
      "titlebar",
      "sidebar",
      "chrome",
      "restart",
    ],
    primaryHotkey: ["hotkey", "shortcut", "primary", "double", "alt", "ctrl", "shift", "meta"],
    secondaryHotkey: ["hotkey", "shortcut", "secondary", "ctrl", "alt", "space", "record"],
    deepseekApiKey: ["deepseek", "api", "key", "ai"],
    defaultModel: ["default", "model", "deepseek", "api"],
    multimodalModel: ["multimodal", "model", "vision", "image"],
    imageModel: ["image", "generate", "generation", "gpt-image-2", "dalle", "draw", "openai"],
    multimodalSplitAnalysis: [
      "multimodal",
      "split",
      "analysis",
      "vision",
      "image",
      "deepseek",
      "r1",
    ],
    largeContextEnabled: ["context", "window", "1m", "million", "token", "compact"],
    reasoningEffort: ["reasoning", "effort", "thinking", "deepseek", "grok", "responses"],
    reasoningLanguage: ["reasoning", "language", "response"],
    showReasoning: ["reasoning", "thinking", "display", "chat"],
    passToolReasoning: ["reasoning", "tool", "passthrough", "thinking"],
    continueThinkingAfterTools: ["reasoning", "thinking", "continue", "tools", "token"],
    memoryEnabled: ["memory", "enabled", "mem0"],
    mem0ApiKey: ["memory", "mem0", "api", "key"],
    mem0UserId: ["memory", "mem0", "user", "id"],
    mem0BaseUrl: ["memory", "mem0", "url", "endpoint"],
    webSearchEnabled: ["web", "search", "enabled", "online"],
    webSearchProvider: ["web", "search", "provider", "serper", "tavily"],
    serperApiKey: ["web", "search", "serper", "api", "key"],
    tavilyApiKey: ["web", "search", "tavily", "api", "key"],
    semanticSearchEnabled: ["semantic", "workspace", "search", "embedding", "rag"],
    semanticSearchModel: ["semantic", "model", "embedding", "bge", "e5"],
    toolApprovalMode: ["approval", "ask", "auto", "alwaysAllow"],
    agentWorkDisplay: ["display", "detailed", "compact", "timeline", "process", "agent", "work"],
    lspEnabled: ["lsp", "diagnostics", "definition"],
    multiModelCollaboration: ["agent", "model", "collaboration", "subagent"],
    minimalCoding: ["minimal", "yagni", "reuse", "stdlib", "diff", "agent", "coding"],
    pixpinPinAiEnabled: ["pixpin", "pin", "screenshot", "image", "ai", "button", "plugin"],
    snipastePinAiEnabled: ["snipaste", "pin", "screenshot", "image", "ai", "button", "plugin"],
    appName: ["app", "name"],
    appVersion: ["version"],
    appIdentifier: ["identifier", "bundle"],
  },
  "zh-CN": {
    colorScheme: ["配色", "主题", "颜色", "theme", "color"],
    language: ["语言", "language", "locale"],
    zoom: ["缩放", "放大", "大小", "字体", "zoom", "scale"],
    hardwareAccelerationEnabled: ["gpu", "硬件", "加速", "渲染", "webview2", "重启"],
    opacity: ["透明", "透明度", "毛玻璃", "blur", "opacity", "glass", "acrylic", "外观"],
    chromeFrostedGlass: ["毛玻璃", "顶栏", "侧栏", "透明", "blur", "acrylic", "mica", "glass"],
    primaryHotkey: ["快捷键", "主快捷键", "双击", "hotkey", "shortcut", "alt", "ctrl"],
    secondaryHotkey: ["快捷键", "副快捷键", "录制", "hotkey", "shortcut", "ctrl", "alt"],
    deepseekApiKey: ["deepseek", "api", "key", "密钥", "ai"],
    defaultModel: ["默认模型", "模型", "deepseek", "model", "api"],
    multimodalModel: ["多模态模型", "图片", "视觉", "multimodal", "vision", "image"],
    imageModel: ["生图", "画图", "配图", "gpt-image-2", "dalle", "image", "generate", "openai"],
    multimodalSplitAnalysis: ["多模态分步分析", "分步", "分析", "图片", "视觉", "deepseek", "r1"],
    largeContextEnabled: ["上下文", "1m", "百万", "token", "压缩", "context", "window"],
    reasoningEffort: ["reasoning", "effort", "思考", "推理", "deepseek", "grok", "responses"],
    reasoningLanguage: ["reasoning", "language", "推理语言", "回答语言"],
    showReasoning: ["思考过程", "推理", "显示", "reasoning", "thinking"],
    passToolReasoning: ["reasoning", "tool", "工具", "回传", "thinking"],
    continueThinkingAfterTools: ["续轮", "思考", "工具", "token", "thinking", "continue"],
    memoryEnabled: ["记忆", "启用", "memory", "mem0"],
    mem0ApiKey: ["记忆", "mem0", "api", "key", "密钥"],
    mem0UserId: ["记忆", "mem0", "user", "id", "用户"],
    mem0BaseUrl: ["记忆", "mem0", "url", "endpoint", "地址"],
    webSearchEnabled: ["联网", "搜索", "web", "search", "启用"],
    webSearchProvider: ["联网", "搜索", "provider", "serper", "tavily"],
    serperApiKey: ["联网", "搜索", "serper", "api", "key", "密钥"],
    tavilyApiKey: ["联网", "搜索", "tavily", "api", "key", "密钥"],
    semanticSearchEnabled: ["语义", "检索", "工作区", "embedding", "rag", "search"],
    semanticSearchModel: ["语义", "模型", "嵌入", "embedding", "bge", "e5"],
    toolApprovalMode: ["审批", "ask", "auto", "一律允许", "approval"],
    agentWorkDisplay: [
      "详细",
      "轻量",
      "显示",
      "过程详情",
      "时间线",
      "detailed",
      "compact",
      "display",
    ],
    lspEnabled: ["lsp", "语言服务", "diagnostics"],
    multiModelCollaboration: ["多模型", "协同", "子agent", "collaboration"],
    minimalCoding: ["精简", "编码", "yagni", "复用", "最小", "改动", "minimal"],
    pixpinPinAiEnabled: ["pixpin", "贴图", "截图", "图片", "ai", "按钮", "插件"],
    snipastePinAiEnabled: ["snipaste", "贴图", "截图", "图片", "ai", "按钮", "插件"],
    appName: ["应用", "名称"],
    appVersion: ["版本", "version"],
    appIdentifier: ["标识", "identifier"],
  },
};

export function getSettingFieldPath(
  language: AppLanguage,
  id: SettingFieldId,
  translatedTitle: string,
): string {
  if (language === "zh-CN" || language === "en-US") {
    return settingsFieldPaths[language][id] ?? settingsFieldPaths["en-US"][id] ?? translatedTitle;
  }
  return translatedTitle;
}

export function getSettingFieldKeywords(language: AppLanguage, id: SettingFieldId): string[] {
  if (language === "zh-CN") {
    return settingsFieldKeywords["zh-CN"][id] ?? settingsFieldKeywords["en-US"][id] ?? [];
  }
  return settingsFieldKeywords["en-US"][id] ?? [];
}
