import { tr } from "@/services/i18n";
import {
  settingsFieldIds,
  settingsFieldHelpIds,
  getSettingFieldPath,
  getSettingFieldKeywords,
  type SettingFieldId,
  type SettingHelpFieldId,
  type SettingsI18nKey,
} from "@/services/locales/settings";
import type { AppLanguage, CategoryId } from "@/types/setting";

export type { CategoryId };

export type SettingType =
  | "select-color"
  | "select-language"
  | "select-reasoning-effort"
  | "select-reasoning-language"
  | "select-web-search-provider"
  | "select-tool-approval-mode"
  | "select-agent-work-display"
  | "select-model"
  | "select-zoom"
  | "secret"
  | "memory-secret"
  | "search-secret"
  | "memory-text"
  | "toggle"
  | "readonly"
  | "slider"
  | "hotkey-record"
  | "collaboration-models";

export interface SettingDefinition {
  id: string;
  category: CategoryId;
  group: string;
  path: string;
  title: string;
  description: string;
  help: string;
  type: SettingType;
  keywords: string[];
  value?: string;
  min?: number;
  max?: number;
  step?: number;
}

export interface SettingsAppInfo {
  appName: string;
  appVersion: string;
  appIdentifier: string;
}

interface FieldCopy {
  title: string;
  description: string;
  help: string;
  path: string;
  keywords: string[];
}

const INLINE_DESCRIPTION_IDS: ReadonlySet<SettingFieldId> = new Set([
  "hardwareAccelerationEnabled",
  "chromeFrostedGlass",
  "primaryHotkey",
  "secondaryHotkey",
  "passToolReasoning",
  "continueThinkingAfterTools",
  "toolApprovalMode",
]);

const SILENT_COPY_IDS: ReadonlySet<SettingFieldId> = new Set([
  "language",
  "appName",
  "appVersion",
  "appIdentifier",
]);

const HELP_FIELD_ID_SET: ReadonlySet<string> = new Set(settingsFieldHelpIds);

function isHelpFieldId(id: SettingFieldId): id is SettingHelpFieldId {
  return HELP_FIELD_ID_SET.has(id);
}

function splitFieldCopy(
  language: AppLanguage,
  id: SettingFieldId,
  rawDescription: string,
): Pick<FieldCopy, "description" | "help"> {
  if (SILENT_COPY_IDS.has(id)) {
    return { description: "", help: "" };
  }
  const extraHelp = isHelpFieldId(id)
    ? tr(language, `settings.fields.${id}.help` as SettingsI18nKey)
    : "";
  if (INLINE_DESCRIPTION_IDS.has(id)) {
    return { description: rawDescription, help: extraHelp };
  }
  return { description: "", help: extraHelp || rawDescription };
}

function buildFieldCopy(language: AppLanguage): Record<SettingFieldId, FieldCopy> {
  return Object.fromEntries(
    settingsFieldIds.map((id) => {
      const title = tr(language, `settings.fields.${id}.title` as SettingsI18nKey);
      const rawDescription = tr(language, `settings.fields.${id}.description` as SettingsI18nKey);
      const { description, help } = splitFieldCopy(language, id, rawDescription);
      const entry: FieldCopy = {
        title,
        description,
        help,
        path: getSettingFieldPath(language, id, title),
        keywords: getSettingFieldKeywords(language, id),
      };
      return [id, entry];
    }),
  ) as Record<SettingFieldId, FieldCopy>;
}

export function buildSettingDefinitions(
  language: AppLanguage,
  info: SettingsAppInfo,
): SettingDefinition[] {
  const m = buildFieldCopy(language);
  const groups = {
    themeLanguage: tr(language, "settings.groups.themeLanguage"),
    window: tr(language, "settings.groups.window"),
    hotkeys: tr(language, "settings.groups.hotkeys"),
    performance: tr(language, "settings.groups.performance"),
    modelSelection: tr(language, "settings.groups.modelSelection"),
    imageGeneration: tr(language, "settings.groups.imageGeneration"),
    context: tr(language, "settings.groups.context"),
    reasoning: tr(language, "settings.groups.reasoning"),
    memory: tr(language, "settings.groups.memory"),
    mem0: tr(language, "settings.groups.mem0"),
    webSearch: tr(language, "settings.groups.webSearch"),
    searchKeys: tr(language, "settings.groups.searchKeys"),
    agentSafety: tr(language, "settings.groups.agentSafety"),
    agentDisplay: tr(language, "settings.groups.agentDisplay"),
    agentCapabilities: tr(language, "settings.groups.agentCapabilities"),
    plugins: tr(language, "settings.groups.plugins"),
    about: tr(language, "settings.groups.about"),
  };

  return [
    {
      id: "colorScheme",
      category: "appearance",
      group: groups.themeLanguage,
      path: m.colorScheme.path,
      title: m.colorScheme.title,
      description: m.colorScheme.description,
      help: m.colorScheme.help,
      type: "select-color",
      keywords: [...m.colorScheme.keywords],
    },
    {
      id: "language",
      category: "appearance",
      group: groups.themeLanguage,
      path: m.language.path,
      title: m.language.title,
      description: m.language.description,
      help: m.language.help,
      type: "select-language",
      keywords: [...m.language.keywords],
    },
    {
      id: "zoom",
      category: "appearance",
      group: groups.window,
      path: m.zoom.path,
      title: m.zoom.title,
      description: m.zoom.description,
      help: m.zoom.help,
      type: "select-zoom",
      keywords: [...m.zoom.keywords],
    },
    {
      id: "opacity",
      category: "appearance",
      group: groups.window,
      path: m.opacity.path,
      title: m.opacity.title,
      description: m.opacity.description,
      help: m.opacity.help,
      type: "slider",
      min: 10,
      max: 100,
      step: 5,
      keywords: [...m.opacity.keywords],
    },
    {
      id: "chromeFrostedGlass",
      category: "appearance",
      group: groups.window,
      path: m.chromeFrostedGlass.path,
      title: m.chromeFrostedGlass.title,
      description: m.chromeFrostedGlass.description,
      help: m.chromeFrostedGlass.help,
      type: "toggle",
      keywords: [...m.chromeFrostedGlass.keywords],
    },
    {
      id: "primaryHotkey",
      category: "appearance",
      group: groups.hotkeys,
      path: m.primaryHotkey.path,
      title: m.primaryHotkey.title,
      description: m.primaryHotkey.description,
      help: m.primaryHotkey.help,
      type: "hotkey-record",
      keywords: [...m.primaryHotkey.keywords],
    },
    {
      id: "secondaryHotkey",
      category: "appearance",
      group: groups.hotkeys,
      path: m.secondaryHotkey.path,
      title: m.secondaryHotkey.title,
      description: m.secondaryHotkey.description,
      help: m.secondaryHotkey.help,
      type: "hotkey-record",
      keywords: [...m.secondaryHotkey.keywords],
    },
    {
      id: "hardwareAccelerationEnabled",
      category: "appearance",
      group: groups.performance,
      path: m.hardwareAccelerationEnabled.path,
      title: m.hardwareAccelerationEnabled.title,
      description: m.hardwareAccelerationEnabled.description,
      help: m.hardwareAccelerationEnabled.help,
      type: "toggle",
      keywords: [...m.hardwareAccelerationEnabled.keywords],
    },
    {
      id: "defaultModel",
      category: "ai",
      group: groups.modelSelection,
      path: m.defaultModel.path,
      title: m.defaultModel.title,
      description: m.defaultModel.description,
      help: m.defaultModel.help,
      type: "select-model",
      keywords: [...m.defaultModel.keywords],
    },
    {
      id: "multimodalModel",
      category: "ai",
      group: groups.modelSelection,
      path: m.multimodalModel.path,
      title: m.multimodalModel.title,
      description: m.multimodalModel.description,
      help: m.multimodalModel.help,
      type: "select-model",
      keywords: [...m.multimodalModel.keywords],
    },
    {
      id: "imageModel",
      category: "image",
      group: groups.imageGeneration,
      path: m.imageModel.path,
      title: m.imageModel.title,
      description: m.imageModel.description,
      help: m.imageModel.help,
      type: "select-model",
      keywords: [...m.imageModel.keywords],
    },
    {
      id: "multimodalSplitAnalysis",
      category: "ai",
      group: groups.modelSelection,
      path: m.multimodalSplitAnalysis.path,
      title: m.multimodalSplitAnalysis.title,
      description: m.multimodalSplitAnalysis.description,
      help: m.multimodalSplitAnalysis.help,
      type: "toggle",
      keywords: [...m.multimodalSplitAnalysis.keywords],
    },
    {
      id: "largeContextEnabled",
      category: "ai",
      group: groups.context,
      path: m.largeContextEnabled.path,
      title: m.largeContextEnabled.title,
      description: m.largeContextEnabled.description,
      help: m.largeContextEnabled.help,
      type: "toggle",
      keywords: [...m.largeContextEnabled.keywords],
    },
    {
      id: "reasoningEffort",
      category: "ai",
      group: groups.reasoning,
      path: m.reasoningEffort.path,
      title: m.reasoningEffort.title,
      description: m.reasoningEffort.description,
      help: m.reasoningEffort.help,
      type: "select-reasoning-effort",
      keywords: [...m.reasoningEffort.keywords],
    },
    {
      id: "reasoningLanguage",
      category: "ai",
      group: groups.reasoning,
      path: m.reasoningLanguage.path,
      title: m.reasoningLanguage.title,
      description: m.reasoningLanguage.description,
      help: m.reasoningLanguage.help,
      type: "select-reasoning-language",
      keywords: [...m.reasoningLanguage.keywords],
    },
    {
      id: "showReasoning",
      category: "ai",
      group: groups.reasoning,
      path: m.showReasoning.path,
      title: m.showReasoning.title,
      description: m.showReasoning.description,
      help: m.showReasoning.help,
      type: "toggle",
      keywords: [...m.showReasoning.keywords],
    },
    {
      id: "passToolReasoning",
      category: "ai",
      group: groups.reasoning,
      path: m.passToolReasoning.path,
      title: m.passToolReasoning.title,
      description: m.passToolReasoning.description,
      help: m.passToolReasoning.help,
      type: "toggle",
      keywords: [...m.passToolReasoning.keywords],
    },
    {
      id: "continueThinkingAfterTools",
      category: "ai",
      group: groups.reasoning,
      path: m.continueThinkingAfterTools.path,
      title: m.continueThinkingAfterTools.title,
      description: m.continueThinkingAfterTools.description,
      help: m.continueThinkingAfterTools.help,
      type: "toggle",
      keywords: [...m.continueThinkingAfterTools.keywords],
    },
    {
      id: "memoryEnabled",
      category: "memory",
      group: groups.memory,
      path: m.memoryEnabled.path,
      title: m.memoryEnabled.title,
      description: m.memoryEnabled.description,
      help: m.memoryEnabled.help,
      type: "toggle",
      keywords: [...m.memoryEnabled.keywords],
    },
    {
      id: "mem0ApiKey",
      category: "memory",
      group: groups.mem0,
      path: m.mem0ApiKey.path,
      title: m.mem0ApiKey.title,
      description: m.mem0ApiKey.description,
      help: m.mem0ApiKey.help,
      type: "memory-secret",
      keywords: [...m.mem0ApiKey.keywords],
    },
    {
      id: "mem0UserId",
      category: "memory",
      group: groups.mem0,
      path: m.mem0UserId.path,
      title: m.mem0UserId.title,
      description: m.mem0UserId.description,
      help: m.mem0UserId.help,
      type: "memory-text",
      keywords: [...m.mem0UserId.keywords],
    },
    {
      id: "mem0BaseUrl",
      category: "memory",
      group: groups.mem0,
      path: m.mem0BaseUrl.path,
      title: m.mem0BaseUrl.title,
      description: m.mem0BaseUrl.description,
      help: m.mem0BaseUrl.help,
      type: "memory-text",
      keywords: [...m.mem0BaseUrl.keywords],
    },
    {
      id: "webSearchEnabled",
      category: "search",
      group: groups.webSearch,
      path: m.webSearchEnabled.path,
      title: m.webSearchEnabled.title,
      description: m.webSearchEnabled.description,
      help: m.webSearchEnabled.help,
      type: "toggle",
      keywords: [...m.webSearchEnabled.keywords],
    },
    {
      id: "webSearchProvider",
      category: "search",
      group: groups.webSearch,
      path: m.webSearchProvider.path,
      title: m.webSearchProvider.title,
      description: m.webSearchProvider.description,
      help: m.webSearchProvider.help,
      type: "select-web-search-provider",
      keywords: [...m.webSearchProvider.keywords],
    },
    {
      id: "serperApiKey",
      category: "search",
      group: groups.searchKeys,
      path: m.serperApiKey.path,
      title: m.serperApiKey.title,
      description: m.serperApiKey.description,
      help: m.serperApiKey.help,
      type: "search-secret",
      keywords: [...m.serperApiKey.keywords],
    },
    {
      id: "tavilyApiKey",
      category: "search",
      group: groups.searchKeys,
      path: m.tavilyApiKey.path,
      title: m.tavilyApiKey.title,
      description: m.tavilyApiKey.description,
      help: m.tavilyApiKey.help,
      type: "search-secret",
      keywords: [...m.tavilyApiKey.keywords],
    },
    {
      id: "toolApprovalMode",
      category: "agent",
      group: groups.agentSafety,
      path: m.toolApprovalMode.path,
      title: m.toolApprovalMode.title,
      description: m.toolApprovalMode.description,
      help: m.toolApprovalMode.help,
      type: "select-tool-approval-mode",
      keywords: [...m.toolApprovalMode.keywords],
    },
    {
      id: "agentWorkDisplay",
      category: "agent",
      group: groups.agentDisplay,
      path: m.agentWorkDisplay.path,
      title: m.agentWorkDisplay.title,
      description: m.agentWorkDisplay.description,
      help: m.agentWorkDisplay.help,
      type: "select-agent-work-display",
      keywords: [...m.agentWorkDisplay.keywords],
    },
    {
      id: "lspEnabled",
      category: "agent",
      group: groups.agentCapabilities,
      path: m.lspEnabled.path,
      title: m.lspEnabled.title,
      description: m.lspEnabled.description,
      help: m.lspEnabled.help,
      type: "toggle",
      keywords: [...m.lspEnabled.keywords],
    },
    {
      id: "multiModelCollaboration",
      category: "agent",
      group: groups.agentCapabilities,
      path: m.multiModelCollaboration.path,
      title: m.multiModelCollaboration.title,
      description: m.multiModelCollaboration.description,
      help: m.multiModelCollaboration.help,
      type: "collaboration-models",
      keywords: [...m.multiModelCollaboration.keywords],
    },
    {
      id: "minimalCoding",
      category: "agent",
      group: groups.agentCapabilities,
      path: m.minimalCoding.path,
      title: m.minimalCoding.title,
      description: m.minimalCoding.description,
      help: m.minimalCoding.help,
      type: "toggle",
      keywords: [...m.minimalCoding.keywords],
    },
    {
      id: "pixpinPinAiEnabled",
      category: "plugins",
      group: groups.plugins,
      path: m.pixpinPinAiEnabled.path,
      title: m.pixpinPinAiEnabled.title,
      description: m.pixpinPinAiEnabled.description,
      help: m.pixpinPinAiEnabled.help,
      type: "toggle",
      keywords: [...m.pixpinPinAiEnabled.keywords],
    },
    {
      id: "snipastePinAiEnabled",
      category: "plugins",
      group: groups.plugins,
      path: m.snipastePinAiEnabled.path,
      title: m.snipastePinAiEnabled.title,
      description: m.snipastePinAiEnabled.description,
      help: m.snipastePinAiEnabled.help,
      type: "toggle",
      keywords: [...m.snipastePinAiEnabled.keywords],
    },
    {
      id: "appName",
      category: "about",
      group: groups.about,
      path: m.appName.path,
      title: m.appName.title,
      description: m.appName.description,
      help: m.appName.help,
      type: "readonly",
      keywords: [...m.appName.keywords],
      value: info.appName,
    },
    {
      id: "appVersion",
      category: "about",
      group: groups.about,
      path: m.appVersion.path,
      title: m.appVersion.title,
      description: m.appVersion.description,
      help: m.appVersion.help,
      type: "readonly",
      keywords: [...m.appVersion.keywords],
      value: info.appVersion,
    },
    {
      id: "appIdentifier",
      category: "about",
      group: groups.about,
      path: m.appIdentifier.path,
      title: m.appIdentifier.title,
      description: m.appIdentifier.description,
      help: m.appIdentifier.help,
      type: "readonly",
      keywords: [...m.appIdentifier.keywords],
      value: info.appIdentifier,
    },
  ];
}
