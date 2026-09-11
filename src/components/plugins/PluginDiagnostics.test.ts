/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import PluginDiagnostics from "./PluginDiagnostics.vue";
import { registerAsset, unregisterPluginAssets } from "@/composables/plugins/assetRegistry";
import { mountSlot, unmountPluginFromAllAnchors } from "@/composables/plugins/slotRegistry";

describe("PluginDiagnostics", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    unmountPluginFromAllAnchors("plugin-a");
    unmountPluginFromAllAnchors("plugin-b");
    unregisterPluginAssets("plugin-a");
    unregisterPluginAssets("plugin-b");
  });

  it("offers take-over for exclusive slot and asset conflicts", () => {
    mountSlot("workbench.main", { id: "a:main", pluginId: "plugin-a", mount: () => {} });
    mountSlot("workbench.main", { id: "b:main", pluginId: "plugin-b", mount: () => {} });
    registerAsset("workbench.backdrop", "plugin-a", { kind: "image", source: "a.png" });
    registerAsset("workbench.backdrop", "plugin-b", { kind: "image", source: "b.png" });

    const wrapper = mount(PluginDiagnostics);
    const text = wrapper.text();
    expect(text).toContain("workbench.main");
    expect(text).toContain("workbench.backdrop");
    expect(text).toContain("plugin-a");
    expect(text).toContain("plugin-b");
    const buttons = wrapper.findAll(".diagnostics-takeover");
    expect(buttons).toHaveLength(2);
    expect(buttons[0].text()).toContain("停用占用者并重载被拒绝插件");
  });
});
