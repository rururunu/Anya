// @vitest-environment happy-dom
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  applyInformationStorage,
  exportUserInformation,
  informationStorage,
  parseUserInformation,
  readUsagePreferences,
} from "./userInformation";
import type { AppSettings } from "@/types/setting";
vi.mock("@/services/chat/imageEditReference", () => ({
  loadImageSourceAsDataUrl: vi.fn().mockResolvedValue("data:image/png;base64,Ymc="),
}));
const settings = {
  colorScheme: "dark",
  chatModel: "my-model",
  chatModelProvider: "my-provider",
  deepseekApiKey: "test-deepseek-key",
  deepseekModels: [{ id: "custom-model", disabled: false, custom: true }],
  customProviders: [
    {
      id: "my-provider",
      name: "Provider",
      baseUrl: "https://example.com",
      apiKey: "test-provider-key",
      models: "my-model",
      modelProtocols: { "my-model": "responses" },
    },
  ],
  imageModel: "image-model",
  imageProviders: [
    {
      id: "image",
      name: "Images",
      baseUrl: "https://example.com",
      apiKey: "test-image-key",
      models: "image-model",
    },
  ],
  memoryEnabled: true,
  mem0ApiKey: "test-memory-key",
  webSearchEnabled: true,
  serperApiKey: "test-search-key",
  customThemes: [],
  customBackground: { image: "C:/pictures/background.png" },
} as unknown as AppSettings;
beforeEach(() => {
  localStorage.clear();
  localStorage.setItem(
    "anya.local-profile.v1",
    JSON.stringify({ displayName: "测试用户", handle: "my-profile" }),
  );
  localStorage.setItem("anya.user-avatar-style.v1", "adventurer");
  localStorage.setItem("anya.user-avatar-seed.v1", "my-seed");
  localStorage.setItem("anya.user-avatar-options.v1", JSON.stringify({ hairColor: "#ab2a18" }));
  localStorage.setItem("anya.user-avatar-image.v1", "data:image/png;base64,YQ==");
  localStorage.setItem(
    "anya.token-usage-preferences.v1",
    JSON.stringify({ range: "7d", granularity: "week", customFrom: "", customTo: "" }),
  );
});
describe("user information export and import", () => {
  it("round-trips model lists, provider protocols, API keys, appearance and avatar data", async () => {
    const exported = await exportUserInformation(settings);
    expect(exported.settings.customBackground?.image).toBe("data:image/png;base64,Ymc=");
    expect(settings.customBackground?.image).toBe("C:/pictures/background.png");
    const imported = parseUserInformation(JSON.stringify(exported), settings);
    expect(imported.settings.deepseekApiKey).toBe(settings.deepseekApiKey);
    expect(imported.settings.customProviders).toEqual(settings.customProviders);
    expect(imported.settings.imageProviders).toEqual(settings.imageProviders);
    expect(imported.settings.deepseekModels).toEqual(settings.deepseekModels);
    expect(imported.settings.mem0ApiKey).toBe("test-memory-key");
    expect(imported.settings.serperApiKey).toBe("test-search-key");
    localStorage.clear();
    applyInformationStorage(informationStorage(imported));
    expect(JSON.parse(localStorage.getItem("anya.local-profile.v1")!)).toEqual({
      displayName: "测试用户",
      handle: "my-profile",
    });
    expect(localStorage.getItem("anya.user-avatar-image.v1")).toBe("data:image/png;base64,YQ==");
    expect(readUsagePreferences().granularity).toBe("week");
    expect(imported).not.toHaveProperty("tokenUsageHistory");
  });
  it("rejects wrong formats and invalid settings before applying anything", async () => {
    const exported = await exportUserInformation(settings);
    expect(() =>
      parseUserInformation(JSON.stringify({ ...exported, version: 2 }), settings),
    ).toThrow();
    expect(() =>
      parseUserInformation(
        JSON.stringify({ ...exported, settings: { colorScheme: 12 } }),
        settings,
      ),
    ).toThrow();
    expect(() =>
      parseUserInformation(
        JSON.stringify({
          ...exported,
          profile: {
            ...exported.profile,
            avatar: { ...exported.profile.avatar, image: "https://example.com/avatar.png" },
          },
        }),
        settings,
      ),
    ).toThrow();
  });
  it("ignores unknown setting keys while keeping the supported configuration", async () => {
    const exported = await exportUserInformation(settings);
    const imported = parseUserInformation(
      JSON.stringify({ ...exported, settings: { ...exported.settings, unexpected: "value" } }),
      settings,
    );
    expect(imported.settings).not.toHaveProperty("unexpected");
    expect(imported.settings.chatModel).toBe("my-model");
  });
});
