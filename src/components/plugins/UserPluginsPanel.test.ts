/** @vitest-environment jsdom */
import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { usePluginsStore } from "@/stores/plugins";

vi.mock("@/services/plugins/ipc", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/services/plugins/ipc")>();
  return {
    ...actual,
    listUserPlugins: vi.fn(async () => []),
    pluginSafeModeStatus: vi.fn(async () => ({ active: false, reason: "" })),
  };
});

import UserPluginsPanel from "./UserPluginsPanel.vue";
import { pluginRole } from "@/composables/plugins/usePluginActions";
import type { UserPluginSummary } from "@/services/plugins/ipc";

describe("UserPluginsPanel toolbar", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    const store = usePluginsStore();
    store.plugins = [];
    store.setSelectedPluginId(null);
  });

  it("mixes icon-only, ghost, and filled actions", () => {
    const wrapper = mount(UserPluginsPanel);
    const iconButtons = wrapper.findAll(".toolbar-icons button");
    expect(iconButtons).toHaveLength(3);
    expect(iconButtons[0].attributes("aria-label")).toBe("刷新");
    expect(iconButtons[1].attributes("aria-label")).toBe("打开插件目录");
    expect(iconButtons[2].attributes("aria-label")).toBe("关闭全部插件窗口");
    for (const button of iconButtons) {
      expect(button.attributes("data-size")).toBe("icon");
      expect(button.attributes("data-variant")).toBe("ghost");
    }

    const labeled = wrapper.findAll(".user-plugins-toolbar > button");
    expect(labeled).toHaveLength(2);
    expect(labeled[0].text()).toContain("导入文件夹");
    expect(labeled[0].attributes("data-variant")).toBe("ghost");
    expect(labeled[1].text()).toContain("导入");
    expect(labeled[1].attributes("data-variant")).toBeUndefined();
  });
});

describe("pluginRole", () => {
  function plugin(overrides: Partial<UserPluginSummary> = {}): UserPluginSummary {
    return {
      id: "x",
      name: "X",
      version: "0.1.0",
      apiVersion: "1.0",
      apiSupported: true,
      description: "",
      path: "",
      enabled: false,
      permissions: [],
      capabilities: [],
      granted: [],
      hasUi: false,
      hasHost: true,
      role: "",
      contributes: {},
      official: false,
      ...overrides,
    };
  }

  it("prefers explicit role over chrome", () => {
    expect(pluginRole(plugin({ role: "agent", contributes: { sidebar: true } }))).toBe("agent");
    expect(pluginRole(plugin({ role: "service", contributes: { sidebar: true } }))).toBe("service");
  });

  it("infers agent when tools exist without chrome", () => {
    expect(pluginRole(plugin({ contributes: { agent: { tools: true } } }))).toBe("agent");
  });

  it("infers ui from chrome", () => {
    expect(pluginRole(plugin({ contributes: { sidebar: true } }))).toBe("ui");
  });
});
