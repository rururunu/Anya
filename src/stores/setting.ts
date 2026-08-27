import { defineStore } from "pinia";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

import { DEFAULT_CHAT_MODEL } from "@/constants/chat";
import { getAppSettings, setAppSettings } from "@/services/ipc";
import { applyOpacity, applyChromeFrostedGlass } from "@/services/overlay/appearance";
import {
  normalizeColorScheme,
  normalizeChatMode,
  normalizeReasoningEffort,
  type AppLanguage,
  type AppSettings,
  type AppSettingsPatch,
  type ColorScheme,
  COLOR_SCHEME_CACHE_KEY,
  defaultGeminiOAuthSettings,
  readCachedColorScheme,
} from "@/types/setting";

const LEGACY_STORAGE_KEY = "peek.settings";
let settingsUpdateSequence = 0;

const defaultSettings: AppSettings = {
  colorScheme: "light",
  language: "zh-CN",
  deepseekApiKey: "",
  geminiOauth: defaultGeminiOAuthSettings(),
  memoryEnabled: true,
  mem0ApiKey: "",
  mem0UserId: "peek-user",
  mem0BaseUrl: "https://api.mem0.ai/v1",
  webSearchEnabled: false,
  webSearchProvider: "serper",
  serperApiKey: "",
  tavilyApiKey: "",
  toolApprovalMode: "ask",
  chatMode: "agent",
  lspEnabled: false,
  lspServers: [],
  mcpServers: [],
  smitheryApiKey: "",
  enabledBuiltinSkills: [],
  opacity: 100,
  chromeFrostedGlass: false,
  chatModel: DEFAULT_CHAT_MODEL,
  chatModelProvider: "",
  multimodalModel: "gpt-4o",
  multimodalModelProvider: "",
  imageModel: "gpt-image-2",
  imageModelProvider: "",
  imageProviders: [],
  imageStyleTemplates: [],
  multimodalSplitAnalysis: true,
  largeContextEnabled: true,
  reasoningEffort: "disabled",
  reasoningLanguage: "auto",
  passToolReasoning: true,
  continueThinkingAfterTools: true,
  showReasoning: true,
  agentWorkDisplay: "detailed",
  multiModelCollaboration: false,
  collaborationModels: [],
  minimalCoding: false,
  zoom: 100,
  hardwareAccelerationEnabled: false,
  primaryHotkey: "Alt",
  primaryHotkeyEnabled: true,
  secondaryHotkey: "Ctrl+Alt+Space",
  secondaryHotkeyEnabled: true,
  customProviders: [],
  pixpinPinAiEnabled: true,
  snipastePinAiEnabled: true,
  semanticSearchEnabled: false,
  semanticSearchBackend: "api",
  semanticSearchModel: "multilingual-e5-small",
  semanticSearchApiBaseUrl: "",
  semanticSearchApiKey: "",
  semanticSearchApiModel: "",
  onboardingCompleted: false,
};

function applyColorSchemeToDocument(colorScheme: ColorScheme, glass: boolean) {
  const root = document.documentElement;
  const body = document.body;
  if (glass) {
    // Chromium paints an opaque canvas for `color-scheme` on <html>, which
    // hides DWM Acrylic behind the WebView. Keep the scheme on <body> instead.
    root.style.colorScheme = "normal";
    root.style.background = "transparent";
    if (body) {
      body.style.colorScheme = colorScheme;
      body.style.background = "transparent";
    }
    return;
  }
  root.style.colorScheme = colorScheme;
  root.style.removeProperty("background");
  if (body) {
    body.style.removeProperty("color-scheme");
    body.style.removeProperty("background");
  }
}

export function applyTheme(settings: Pick<AppSettings, "colorScheme" | "language">) {
  const colorScheme = normalizeColorScheme(settings.colorScheme);
  try {
    localStorage.setItem(COLOR_SCHEME_CACHE_KEY, colorScheme);
  } catch {
    // ignore quota / private mode
  }
  const root = document.documentElement;
  const nextDark = colorScheme === "dark";
  const glass = root.classList.contains("chrome-frosted-glass");
  const sameTheme =
    root.dataset.theme === colorScheme &&
    root.classList.contains("dark") === nextDark &&
    root.lang === settings.language &&
    (glass ? root.style.colorScheme === "normal" : root.style.colorScheme === colorScheme);
  if (sameTheme) {
    return;
  }
  root.lang = settings.language;
  root.dataset.theme = colorScheme;
  root.classList.toggle("dark", nextDark);
  applyColorSchemeToDocument(colorScheme, glass);
}

/** Theme to paint before settings finish loading (matches boot splash). */
export function bootstrapThemeHint(language: AppLanguage = "zh-CN") {
  return { colorScheme: readCachedColorScheme(), language };
}

export function applyZoom(zoom: number) {
  const normalized = Math.max(zoom, 1) / 100;
  const root = document.documentElement;
  root.style.setProperty("--ui-zoom", String(normalized));
  root.dataset.uiZoomed = normalized === 1 ? "false" : "true";

  // Workbench uses transform:scale on `.workbench` (see Main.vue).
  // Overlay/Settings keep document zoom paired with window resize.
  let isWorkbench = false;
  try {
    isWorkbench = getCurrentWebviewWindow().label === "workbench";
  } catch {
    isWorkbench = false;
  }

  if (isWorkbench) {
    root.style.removeProperty("zoom");
    root.dataset.zoomShell = "workbench";
  } else {
    root.style.zoom = String(normalized);
    root.dataset.zoomShell = "window";
  }
}

function normalizeOpacityValue(settings: AppSettings): number {
  let opacityVal = settings.opacity;
  if (opacityVal === undefined) {
    const legacy = settings as AppSettings & { frostedGlass?: boolean };
    if (legacy.frostedGlass !== undefined) {
      opacityVal = legacy.frostedGlass ? 80 : 100;
    }
  }
  return opacityVal ?? 100;
}

function normalizeZoomValue(settings: AppSettings): number {
  let zoomVal = settings.zoom;
  if (zoomVal !== undefined && zoomVal <= 2.0) {
    zoomVal = Math.round(zoomVal * 100);
  }
  return zoomVal ?? 100;
}

function applyCommonSettings(target: AppSettings, settings: AppSettings) {
  target.colorScheme = normalizeColorScheme(settings.colorScheme);
  target.language = settings.language;
  target.opacity = normalizeOpacityValue(settings);
  target.chromeFrostedGlass = settings.chromeFrostedGlass ?? false;
  target.chatModel = settings.chatModel ?? DEFAULT_CHAT_MODEL;
  target.chatModelProvider = settings.chatModelProvider ?? "";
  target.multimodalModel = settings.multimodalModel ?? "gpt-4o";
  target.multimodalModelProvider = settings.multimodalModelProvider ?? "";
  target.imageModel = settings.imageModel ?? "gpt-image-2";
  target.imageModelProvider = settings.imageModelProvider ?? "";
  target.imageProviders = settings.imageProviders ?? [];
  target.imageStyleTemplates = settings.imageStyleTemplates ?? [];
  target.multimodalSplitAnalysis = settings.multimodalSplitAnalysis ?? true;
  target.largeContextEnabled = settings.largeContextEnabled ?? true;
  target.reasoningEffort = normalizeReasoningEffort(settings.reasoningEffort);
  target.reasoningLanguage = settings.reasoningLanguage ?? "auto";
  target.passToolReasoning = settings.passToolReasoning ?? true;
  target.continueThinkingAfterTools = settings.continueThinkingAfterTools ?? true;
  target.showReasoning = settings.showReasoning ?? true;
  target.agentWorkDisplay = settings.agentWorkDisplay === "compact" ? "compact" : "detailed";
  target.multiModelCollaboration = settings.multiModelCollaboration ?? false;
  target.collaborationModels = settings.collaborationModels ?? [];
  target.minimalCoding = settings.minimalCoding ?? false;
  target.memoryEnabled = settings.memoryEnabled ?? true;
  target.mem0UserId = settings.mem0UserId ?? "peek-user";
  target.mem0BaseUrl = settings.mem0BaseUrl ?? "https://api.mem0.ai/v1";
  target.webSearchEnabled = settings.webSearchEnabled ?? false;
  target.webSearchProvider = settings.webSearchProvider ?? "serper";
  target.toolApprovalMode = settings.toolApprovalMode ?? "ask";
  target.chatMode = normalizeChatMode(settings.chatMode);
  target.lspEnabled = settings.lspEnabled ?? false;
  target.lspServers = settings.lspServers ?? [];
  target.mcpServers = settings.mcpServers ?? [];
  target.enabledBuiltinSkills = settings.enabledBuiltinSkills ?? [];
  target.zoom = normalizeZoomValue(settings);
  target.hardwareAccelerationEnabled = settings.hardwareAccelerationEnabled ?? false;
  target.primaryHotkey = settings.primaryHotkey ?? "Alt";
  target.primaryHotkeyEnabled = settings.primaryHotkeyEnabled ?? true;
  target.secondaryHotkey = settings.secondaryHotkey ?? "Ctrl+Alt+Space";
  target.secondaryHotkeyEnabled = settings.secondaryHotkeyEnabled ?? true;
  target.customProviders = settings.customProviders ?? [];
  target.pixpinPinAiEnabled = settings.pixpinPinAiEnabled ?? true;
  target.snipastePinAiEnabled = settings.snipastePinAiEnabled ?? true;
  target.semanticSearchEnabled = settings.semanticSearchEnabled ?? false;
  target.semanticSearchBackend = settings.semanticSearchBackend ?? "api";
  target.semanticSearchModel = settings.semanticSearchModel ?? "multilingual-e5-small";
  target.semanticSearchApiBaseUrl = settings.semanticSearchApiBaseUrl ?? "";
  target.semanticSearchApiKey = settings.semanticSearchApiKey ?? "";
  target.semanticSearchApiModel = settings.semanticSearchApiModel ?? "";
  target.onboardingCompleted = settings.onboardingCompleted ?? true;
}

function applySecretSettings(target: AppSettings, settings: AppSettings) {
  target.deepseekApiKey = settings.deepseekApiKey ?? "";
  target.geminiOauth = settings.geminiOauth ?? defaultGeminiOAuthSettings();
  target.mem0ApiKey = settings.mem0ApiKey ?? "";
  target.serperApiKey = settings.serperApiKey ?? "";
  target.tavilyApiKey = settings.tavilyApiKey ?? "";
  target.smitheryApiKey = settings.smitheryApiKey ?? "";
}

export const useSettingStore = defineStore("setting", {
  state: (): AppSettings => ({ ...defaultSettings }),
  actions: {
    applyPublicSettings(settings: AppSettings) {
      applyCommonSettings(this, settings);
      applyTheme(settings);
      applyZoom(this.zoom);
      void applyOpacity(this.opacity);
      void applyChromeFrostedGlass(this.chromeFrostedGlass);
    },
    applySettings(settings: AppSettings) {
      applyCommonSettings(this, settings);
      applySecretSettings(this, settings);
      applyTheme(settings);
      applyZoom(this.zoom);
      void applyOpacity(this.opacity);
      void applyChromeFrostedGlass(this.chromeFrostedGlass);
    },
    async load() {
      try {
        const settings = await getAppSettings();
        this.applySettings(settings);
      } catch (error) {
        console.error("get_app_settings failed:", error);
        this.applySettings(defaultSettings);
      }

      const legacy = localStorage.getItem(LEGACY_STORAGE_KEY);
      if (!legacy) {
        return;
      }

      try {
        const parsed = JSON.parse(legacy) as AppSettingsPatch;
        const settings = await setAppSettings(parsed);
        this.applySettings(settings);
      } catch (error) {
        console.error("legacy settings migration failed:", error);
      } finally {
        localStorage.removeItem(LEGACY_STORAGE_KEY);
      }
    },
    async update(partial: AppSettingsPatch) {
      const sequence = ++settingsUpdateSequence;
      const previous = { ...this.$state } as AppSettings;
      const optimistic = { ...previous, ...partial } as AppSettings;
      this.applySettings(optimistic);

      try {
        const settings = await setAppSettings(partial);
        if (sequence === settingsUpdateSequence) {
          this.applySettings(settings);
        }
      } catch (error) {
        if (sequence === settingsUpdateSequence) {
          this.applySettings(previous);
        }
        throw error;
      }
    },
  },
});

export type { AppLanguage, AppSettings, AppSettingsPatch, ColorScheme };
