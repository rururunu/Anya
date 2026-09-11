/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { nextTick } from "vue";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import PluginHomeView from "./PluginHomeView.vue";
import { usePluginsStore } from "@/stores/plugins";
import type { UserPluginSummary } from "@/services/plugins/ipc";

function samplePlugin(overrides: Partial<UserPluginSummary> = {}): UserPluginSummary {
  return {
    id: "video-bg",
    name: "Video backdrop",
    version: "0.1.0",
    apiVersion: "1",
    apiSupported: true,
    description: "backdrop",
    path: "/tmp/video-bg",
    enabled: true,
    permissions: ["ui.workbench"],
    capabilities: [],
    granted: ["ui.workbench"],
    hasUi: true,
    hasHost: false,
    role: "ui",
    contributes: {},
    official: false,
    ...overrides,
  };
}

async function flushMount() {
  await nextTick();
  await nextTick();
}

describe("PluginHomeView", () => {
  it("mounts the home settings view into a content-sized pane", async () => {
    setActivePinia(createPinia());
    const store = usePluginsStore();
    store.plugins = [samplePlugin()];
    store.setHomeSettingsView("video-bg", (el) => {
      const label = document.createElement("label");
      label.textContent = "视频来源";
      el.append(label);
    });

    const wrapper = mount(PluginHomeView, { props: { pluginId: "video-bg" } });
    await flushMount();

    const tabs = wrapper.findAll(".home-tab");
    await tabs[1].trigger("click");
    await flushMount();
    await flushMount();

    const pane = wrapper.get(".plugin-host-pane");
    expect(pane.classes()).toContain("is-fit-content");
    expect(pane.text()).toContain("视频来源");
  });

  it("hides uninstall for official plugins", async () => {
    setActivePinia(createPinia());
    usePluginsStore().plugins = [
      samplePlugin({ id: "terminal", name: "Terminal", official: true }),
    ];
    const wrapper = mount(PluginHomeView, { props: { pluginId: "terminal" } });
    await flushMount();
    expect(wrapper.find(".home-icon-btn.is-danger").exists()).toBe(false);
  });

  it("shows uninstall for user plugins", async () => {
    setActivePinia(createPinia());
    usePluginsStore().plugins = [samplePlugin()];
    const wrapper = mount(PluginHomeView, { props: { pluginId: "video-bg" } });
    await flushMount();
    expect(wrapper.find(".home-icon-btn.is-danger").exists()).toBe(true);
  });
});
