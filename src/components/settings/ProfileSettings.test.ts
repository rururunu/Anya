// @vitest-environment happy-dom
import { mount, flushPromises } from "@vue/test-utils";
import { ref } from "vue";
import { setUserAvatar, setSeedUserAvatar } from "@/services/chat/userAvatar";

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ProfileSettings from "./ProfileSettings.vue";
const getReport = vi.hoisted(() => vi.fn());
const getSettings = vi.hoisted(() => vi.fn());
const setSettings = vi.hoisted(() => vi.fn());
const applySettings = vi.hoisted(() => vi.fn());
const dialogSave = vi.hoisted(() => vi.fn());
const writeBackup = vi.hoisted(() => vi.fn());
const confirmReset = vi.hoisted(() => vi.fn());
const resetInvoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({
  invoke: resetInvoke,
  convertFileSrc: (path: string) => path,
}));
vi.mock("@/components/ui/confirm-dialog", () => ({
  AppConfirmDialog: {
    template: "<div />",
    methods: { ask: (options: unknown) => confirmReset(options) },
  },
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: dialogSave }));
vi.mock("@tauri-apps/plugin-fs", () => ({ writeFile: writeBackup }));
vi.mock("@/stores/chatModel", () => ({
  useChatModelStore: () => ({ refresh: vi.fn().mockResolvedValue(undefined) }),
}));
vi.mock("@/services/ipc", () => ({
  getTokenUsageReport: getReport,
  getAppSettings: getSettings,
  setAppSettings: setSettings,
}));
vi.mock("@/stores/setting", () => ({
  useSettingStore: () => ({
    language: "zh-CN",
    reasoningEffort: "high",
    chatModel: "default-model",
    chatModelProvider: "default-provider",
    toolApprovalMode: "ask",
    applySettings,
  }),
}));
vi.mock("@/services/chat/userAvatar", () => ({
  useUserAvatar: () => ref("data:image/svg+xml,test"),
  setUserAvatar: vi.fn(),
  randomizeUserAvatar: vi.fn(),
  setSeedUserAvatar: vi.fn(),
  getUserAvatarSeed: () => "saved-seed",
  getUserAvatarOptions: () => ({}),
  avatarStyleControls: () => ({}),
  sanitizeAvatarOptions: (_style: string, options: unknown) => options ?? {},
  getUserAvatarStyle: () => "notionists",
  userAvatarStyles: [
    "notionists",
    "adventurer",
    "pixelArt",
    "bottts",
    "emoji",
    "identicon",
    "lorelei",
    "micah",
    "openPeeps",
    "thumbs",
    "shapes",
    "rings",
  ],
  avatarStylePreview: () => "data:image/svg+xml,test",
}));
const fixture = {
  modelCalls: 3,
  total: { totalTokens: 80, accuracy: "exact" },
  byModel: [{ model: "test-model", calls: 3 }],
  timeline: [{ bucket: new Date().toISOString(), totalTokens: 80 }],
};
const wrappers: ReturnType<typeof mount>[] = [];
function page() {
  const wrapper = mount(ProfileSettings);
  wrappers.push(wrapper);
  return wrapper;
}
beforeEach(() => {
  localStorage.clear();
  getReport.mockReset().mockResolvedValue(fixture);
  getSettings
    .mockReset()
    .mockResolvedValue({ chatModel: "old-model", customProviders: [], colorScheme: "light" });
  setSettings.mockReset().mockImplementation(async (patch) => ({
    chatModel: "old-model",
    customProviders: [],
    colorScheme: "light",
    ...patch,
  }));
  applySettings.mockClear();
  dialogSave.mockReset().mockResolvedValue(null);
  writeBackup.mockReset().mockResolvedValue(undefined);
  confirmReset.mockReset().mockResolvedValue(false);
  resetInvoke.mockReset().mockResolvedValue(undefined);
  vi.mocked(setUserAvatar).mockClear();
  vi.mocked(setSeedUserAvatar).mockClear();
});
afterEach(() => {
  wrappers.splice(0).forEach((wrapper) => wrapper.unmount());
});
describe("personal profile", () => {
  it("never resets when the dangerous action is cancelled", async () => {
    const wrapper = page();
    await flushPromises();
    await wrapper.get(".profile-reset-button").trigger("click");
    await flushPromises();
    (document.querySelectorAll(".deletion-option")[1] as HTMLButtonElement).click();
    await flushPromises();
    expect(confirmReset).toHaveBeenCalledWith(
      expect.objectContaining({ tone: "danger", confirmOnEnter: false }),
    );
    expect(confirmReset.mock.calls[0][0].description).toContain("不可恢复");
    expect(resetInvoke).not.toHaveBeenCalled();
  });
  it("starts native reset only after explicit confirmation", async () => {
    confirmReset.mockResolvedValueOnce(true);
    const wrapper = page();
    await flushPromises();
    await wrapper.get(".profile-reset-button").trigger("click");
    await flushPromises();
    (document.querySelectorAll(".deletion-option")[1] as HTMLButtonElement).click();
    await flushPromises();
    expect(resetInvoke).toHaveBeenCalledWith("delete_all_user_information", {
      confirmation: "DELETE_ALL_USER_INFORMATION",
      preserveSettings: false,
    });
    expect(wrapper.get(".profile-reset-button").attributes("disabled")).toBeDefined();
  });
  it("offers information-only deletion and explicitly keeps provider configuration", async () => {
    confirmReset.mockResolvedValueOnce(true);
    const wrapper = page();
    await flushPromises();
    expect(wrapper.get(".profile-reset-button").text()).toBe("删除");
    await wrapper.get(".profile-reset-button").trigger("click");
    await flushPromises();
    expect(resetInvoke).not.toHaveBeenCalled();
    (document.querySelectorAll(".deletion-option")[0] as HTMLButtonElement).click();
    await flushPromises();
    expect(confirmReset.mock.calls[0][0].description).toContain(
      "API Key、供应商、模型及其他应用设置会保留",
    );
    expect(resetInvoke).toHaveBeenCalledWith("delete_all_user_information", {
      confirmation: "DELETE_ALL_USER_INFORMATION",
      preserveSettings: true,
    });
  });
  it("exports native settings with API keys to the selected JSON file", async () => {
    getSettings.mockResolvedValueOnce({
      colorScheme: "dark",
      chatModel: "export-model",
      customProviders: [],
      deepseekApiKey: "private-test-key",
    });
    dialogSave.mockResolvedValueOnce("C:/tmp/user-information.json");
    const wrapper = page();
    await flushPromises();
    await wrapper.findAll(".profile-transfer-actions button")[1].trigger("click");
    await flushPromises();
    expect(writeBackup).toHaveBeenCalledOnce();
    const data = JSON.parse(new TextDecoder().decode(writeBackup.mock.calls[0][1]));
    expect(data.settings.deepseekApiKey).toBe("private-test-key");
    expect(data.settings.chatModel).toBe("export-model");
    expect(data.format).toBe("anya-user-information");
  });
  it("imports configuration and profile through the settings persistence API", async () => {
    const wrapper = page();
    await flushPromises();
    const data = {
      format: "anya-user-information",
      version: 1,
      settings: { chatModel: "imported-model", customProviders: [], colorScheme: "dark" },
      profile: {
        displayName: "Imported user",
        handle: "imported",
        avatar: { style: "pixelArt", seed: "imported-seed", options: {}, image: null },
      },
      tokenUsagePreferences: { range: "7d", granularity: "week" },
    };
    const input = wrapper.get(".profile-transfer-actions input");
    Object.defineProperty(input.element, "files", {
      configurable: true,
      value: [new File([JSON.stringify(data)], "profile.json", { type: "application/json" })],
    });
    await input.trigger("change");
    await flushPromises();
    expect(setSettings).toHaveBeenCalledWith(data.settings);
    expect(applySettings).toHaveBeenCalledWith(
      expect.objectContaining({ chatModel: "imported-model" }),
    );
    expect(wrapper.get("h1").text()).toBe("Imported user");
    expect(localStorage.getItem("anya.user-avatar-seed.v1")).toBe("imported-seed");
  });
  it("lets the user enter a seed without calling the model", async () => {
    const wrapper = page();
    await flushPromises();
    await wrapper.get(".avatar-button").trigger("click");
    await wrapper.get("#avatar-seed").setValue("my-own-avatar");
    await wrapper.get(".avatar-seed-preview button").trigger("click");
    expect(setSeedUserAvatar).toHaveBeenCalledWith("notionists", "my-own-avatar", {});
    expect(wrapper.find("#avatar-description").exists()).toBe(false);
  });
  it("opens avatar editing from the avatar and labels insights as habits", async () => {
    const wrapper = page();
    await flushPromises();
    await wrapper.get(".avatar-button").trigger("click");
    expect(wrapper.get(".avatar-editor").text()).toContain("上传图片");
    expect(wrapper.get(".avatar-editor").text()).toContain("换一个随机头像");
    expect(wrapper.findAll(".avatar-style-option")).toHaveLength(12);
    await wrapper.findAll(".avatar-style-option")[3].trigger("click");
    expect(wrapper.findAll(".avatar-style-option")[3].attributes("aria-pressed")).toBe("true");
    expect(wrapper.get(".profile-insights h2").text()).toBe("习惯");
    await wrapper.get(".avatar-button").trigger("click");
    expect(wrapper.find(".avatar-editor").exists()).toBe(false);
  });
  it("shows recorded usage and switches activity views without featured content", async () => {
    const wrapper = page();
    await flushPromises();
    expect(wrapper.get(".profile-metrics").text()).toContain("80");
    expect(wrapper.get(".profile-insights").text()).toContain("test-model · 100%");
    expect(wrapper.findAll(".activity-calendar .activity-cell").length).toBeGreaterThan(360);
    const tabs = wrapper.findAll(".activity-tabs button");
    await tabs[1].trigger("click");
    expect(wrapper.find(".activity-calendar").exists()).toBe(false);
    expect(wrapper.findAll(".activity-bars span").length).toBeGreaterThan(50);
    await tabs[2].trigger("click");
    expect(tabs[2].attributes("aria-pressed")).toBe("true");
    expect(wrapper.text()).not.toContain("精选作品");
    expect(wrapper.text()).not.toContain("热门插件");
  });
  it("persists the edited local profile and reloads it on the next visit", async () => {
    const wrapper = page();
    await flushPromises();
    await wrapper.get(".profile-edit").trigger("click");
    const inputs = wrapper.findAll(".profile-form input");
    await inputs[0].setValue("测试用户");
    await inputs[1].setValue("@my profile");
    await wrapper.get("form").trigger("submit");
    const next = page();
    await flushPromises();
    expect(next.get("h1").text()).toBe("测试用户");
    expect(next.get(".profile-handle").text()).toBe("@my-profile");
  });
  it("shows a retry state instead of fabricating zero totals on loading failure", async () => {
    getReport.mockRejectedValueOnce(new Error("offline"));
    const wrapper = page();
    await flushPromises();
    expect(wrapper.get("[role='alert']").text()).toContain("offline");
    expect(wrapper.get(".profile-metrics").text()).toContain("—");
    await wrapper.get("[role='alert'] button").trigger("click");
    await flushPromises();
    expect(wrapper.find("[role='alert']").exists()).toBe(false);
    expect(wrapper.get(".profile-metrics").text()).toContain("80");
  });
});
