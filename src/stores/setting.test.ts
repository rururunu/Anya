/** @vitest-environment jsdom */
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { applyZoom, useSettingStore } from "./setting";
import type { AppSettings } from "@/types/setting";

let mockWindowLabel = "desktop-pet";

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({
    label: mockWindowLabel,
  }),
}));

vi.mock("@/services/ipc", () => ({
  getAppSettings: vi.fn(),
  setAppSettings: vi.fn(),
}));

vi.mock("@/services/overlay/appearance", () => ({
  applyOpacity: vi.fn(),
  applyChromeFrostedGlass: vi.fn(),
}));

vi.mock("@/services/theme", () => ({
  applyThemeAppearance: vi.fn(),
}));

describe("applyZoom", () => {
  beforeEach(() => {
    document.documentElement.style.removeProperty("zoom");
    document.documentElement.style.removeProperty("--ui-zoom");
    delete document.documentElement.dataset.uiZoomed;
    delete document.documentElement.dataset.zoomShell;
  });

  it("exempts desktop-pet window from UI zoom", () => {
    mockWindowLabel = "desktop-pet";
    applyZoom(150);

    expect(document.documentElement.style.zoom).toBe("");
    expect(document.documentElement.style.getPropertyValue("--ui-zoom")).toBe("1");
    expect(document.documentElement.dataset.uiZoomed).toBe("false");
    expect(document.documentElement.dataset.zoomShell).toBe("pet");
  });

  it("applies scale handling to workbench window", () => {
    mockWindowLabel = "workbench";
    applyZoom(120);

    expect(document.documentElement.style.zoom).toBe("");
    expect(document.documentElement.style.getPropertyValue("--ui-zoom")).toBe("1.2");
    expect(document.documentElement.dataset.uiZoomed).toBe("true");
    expect(document.documentElement.dataset.zoomShell).toBe("workbench");
  });

  it("applies document zoom to regular auxiliary windows", () => {
    mockWindowLabel = "overlay";
    applyZoom(125);

    expect(document.documentElement.style.zoom).toBe("1.25");
    expect(document.documentElement.style.getPropertyValue("--ui-zoom")).toBe("1.25");
    expect(document.documentElement.dataset.uiZoomed).toBe("true");
    expect(document.documentElement.dataset.zoomShell).toBe("window");
  });
});

/** Every field set to a non-default value that survives normalization unchanged. */
const FULL_SETTINGS: AppSettings = {
  colorScheme: "dark",
  language: "en-US",
  deepseekApiKey: "sk-test",
  deepseekModels: [
    { id: "m1", disabled: false, custom: true },
    { id: "m2", disabled: true, custom: false },
  ],
  memoryEnabled: false,
  mem0ApiKey: "mem0-key",
  mem0UserId: "u-test",
  mem0BaseUrl: "https://mem0.example.com",
  webSearchEnabled: true,
  webSearchProvider: "tavily",
  serperApiKey: "serper-key",
  tavilyApiKey: "tavily-key",
  toolApprovalMode: "auto",
  chatMode: "plan",
  lspEnabled: true,
  lspServers: [
    { id: "ts", languages: ["typescript"], command: "tsserver", args: ["--stdio"], enabled: true },
  ],
  mcpServers: [{ id: "fs", title: "FS", command: "mcp-fs", args: [], enabled: true }],
  smitheryApiKey: "smithery-key",
  enabledBuiltinSkills: ["skill-a"],
  opacity: 90,
  chromeFrostedGlass: true,
  chatModel: "my-model",
  chatModelProvider: "my-provider",
  multimodalModel: "mm-model",
  multimodalModelProvider: "mm-provider",
  imageModel: "img-model",
  imageModelProvider: "img-provider",
  imageProviders: [
    { id: "p1", name: "P1", baseUrl: "https://p1.example.com", apiKey: "k", models: "m" },
  ],
  imageStyleTemplates: [{ id: "t1", name: "T1", prompt: "draw" }],
  multimodalSplitAnalysis: false,
  largeContextEnabled: false,
  reasoningEffort: "high",
  reasoningLanguage: "zh",
  passToolReasoning: false,
  continueThinkingAfterTools: false,
  showReasoning: false,
  agentWorkDisplay: "compact",
  multiModelCollaboration: true,
  collaborationModels: ["a", "b"],
  minimalCoding: true,
  zoom: 110,
  hardwareAccelerationEnabled: true,
  primaryHotkey: "Ctrl",
  primaryHotkeyEnabled: false,
  secondaryHotkey: "Ctrl+Space",
  secondaryHotkeyEnabled: false,
  customProviders: [
    {
      id: "c1",
      name: "C1",
      baseUrl: "https://c1.example.com",
      apiKey: "k",
      models: "m1\nm2",
      disabledModels: "m2",
    },
  ],
  pixpinPinAiEnabled: false,
  snipastePinAiEnabled: false,
  semanticSearchEnabled: true,
  semanticSearchBackend: "local",
  semanticSearchModel: "bge-m3",
  semanticSearchApiBaseUrl: "https://emb.example.com",
  semanticSearchApiKey: "emb-key",
  semanticSearchApiModel: "emb-model",
  onboardingCompleted: true,
  customThemes: [{ id: "theme1", name: "Theme", mode: "dark", tokens: { a: "b" }, updatedAt: 1 }],
  customBackground: { image: "path:C:/bg.png", opacity: 0.3, blur: 2, fit: "cover" },
};

describe("applySettings", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("copies every settings field onto the store (guards against dropped fields)", () => {
    const store = useSettingStore();
    const stateKeys = (Object.keys(store.$state) as (keyof AppSettings)[]).sort();

    // 1. Guard that every key in defaultSettings / store is present in FULL_SETTINGS
    expect(Object.keys(FULL_SETTINGS).sort()).toEqual(stateKeys);

    // 2. Guard that every field in FULL_SETTINGS is actually non-default
    const initialDefaults = { ...store.$state };
    for (const key of stateKeys) {
      expect(
        FULL_SETTINGS[key],
        `FULL_SETTINGS.${key} must differ from the default value`,
      ).not.toEqual(initialDefaults[key]);
    }

    // 3. Apply settings and assert every field (including secrets) was correctly copied
    store.applySettings(FULL_SETTINGS);

    for (const key of stateKeys) {
      expect(store.$state[key], `field "${key}" was not applied`).toEqual(FULL_SETTINGS[key]);
    }
  });

  it("applyPublicSettings copies non-secret fields and preserves secrets", () => {
    const store = useSettingStore();
    const initialSecrets: Record<string, unknown> = {
      deepseekApiKey: store.deepseekApiKey,
      mem0ApiKey: store.mem0ApiKey,
      serperApiKey: store.serperApiKey,
      tavilyApiKey: store.tavilyApiKey,
      smitheryApiKey: store.smitheryApiKey,
    };

    store.applyPublicSettings(FULL_SETTINGS);

    // Secrets must NOT be overwritten by applyPublicSettings
    expect(store.deepseekApiKey).toBe(initialSecrets.deepseekApiKey);
    expect(store.mem0ApiKey).toBe(initialSecrets.mem0ApiKey);
    expect(store.serperApiKey).toBe(initialSecrets.serperApiKey);
    expect(store.tavilyApiKey).toBe(initialSecrets.tavilyApiKey);
    expect(store.smitheryApiKey).toBe(initialSecrets.smitheryApiKey);

    // Non-secret fields must be updated
    expect(store.colorScheme).toBe(FULL_SETTINGS.colorScheme);
    expect(store.language).toBe(FULL_SETTINGS.language);
    expect(store.deepseekModels).toEqual(FULL_SETTINGS.deepseekModels);
    expect(store.memoryEnabled).toBe(FULL_SETTINGS.memoryEnabled);
    expect(store.chatMode).toBe(FULL_SETTINGS.chatMode);
  });
});
