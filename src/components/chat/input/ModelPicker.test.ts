// @vitest-environment happy-dom
import { mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import ModelPicker from "./ModelPicker.vue";

vi.mock("@/stores/setting", () => ({ useSettingStore: () => ({ customProviders: [] }) }));
vi.mock("@/services/providerFavicon", () => ({
  peekProviderFavicon: () => null,
  warmProviderFavicons: () => {},
}));

describe("flat grouped model picker", () => {
  it("shows all providers' models together and uses continuous selection indices", async () => {
    const deepseek = { id: "deepseek-chat", ownedBy: "deepseek", provider: "deepseek" };
    const gemini = { id: "gemini-flash", ownedBy: "google", provider: "gemini" };
    const secondDeepseek = { id: "deepseek-reasoner", ownedBy: "deepseek", provider: "deepseek" };
    const wrapper = mount(ModelPicker, {
      props: {
        models: [deepseek, gemini, secondDeepseek],
        flatGrouped: true,
        selectedModelId: deepseek.id,
        selectedProvider: deepseek.provider,
        selectedIndex: 0,
        activeProvider: null,
        loading: false,
        error: null,
        loadingText: "Loading",
        emptyText: "Empty",
        refreshText: "Refresh",
        backText: "Back",
        modelCountText: "{count} models",
        ariaLabel: "Models",
      },
    });
    expect(wrapper.findAll(".model-provider-heading")).toHaveLength(2);
    expect(wrapper.find(".model-group-item").exists()).toBe(false);
    expect(wrapper.find(".model-picker-back").exists()).toBe(false);
    const rows = wrapper.findAll(".model-picker-item");
    expect(rows).toHaveLength(3);
    await rows[1].trigger("mouseenter");
    await rows[1].trigger("mousedown");
    expect(wrapper.emitted("hover")).toEqual([[1]]);
    expect(wrapper.emitted("select")).toEqual([[secondDeepseek]]);
    await wrapper.get(".model-picker-refresh").trigger("mouseenter");
    expect(wrapper.emitted("hover")?.at(-1)).toEqual([3]);
    wrapper.unmount();
  });
});
