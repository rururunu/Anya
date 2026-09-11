/** @vitest-environment jsdom */
import { afterEach, describe, expect, it } from "vitest";
import { nextTick } from "vue";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import PluginSlotOutlet from "./PluginSlotOutlet.vue";
import { mountSlot, unmountPluginFromAllAnchors } from "@/composables/plugins/slotRegistry";

async function flush() {
  await nextTick();
  await nextTick();
}

describe("PluginSlotOutlet", () => {
  setActivePinia(createPinia());

  afterEach(() => {
    unmountPluginFromAllAnchors("plugin-a");
    unmountPluginFromAllAnchors("plugin-b");
  });

  it("renders stacked composer accessories from SlotRegistry", async () => {
    mountSlot("composer.accessory", {
      id: "a:badge",
      pluginId: "plugin-a",
      mount: (el) => {
        el.textContent = "badge";
      },
    });
    const wrapper = mount(PluginSlotOutlet, {
      props: { anchorId: "composer.accessory", fit: "content", layout: "wrap" },
    });
    await flush();
    expect(wrapper.text()).toContain("badge");
    expect(wrapper.get(".plugin-slot-outlet").classes()).toContain("is-wrap");
    wrapper.unmount();
  });

  it("filters by pluginId and chrome", async () => {
    mountSlot("sidebar.tabs", {
      id: "a:nav",
      pluginId: "plugin-a",
      mount: (el) => {
        el.textContent = "nav-page";
      },
      chrome: ["nav"],
    });
    mountSlot("sidebar.tabs", {
      id: "b:term",
      pluginId: "plugin-b",
      mount: (el) => {
        el.textContent = "term";
      },
      chrome: ["views"],
    });
    const views = mount(PluginSlotOutlet, {
      props: { anchorId: "sidebar.tabs", chrome: "views" },
    });
    await flush();
    expect(views.text()).toContain("term");
    expect(views.text()).not.toContain("nav-page");

    const owned = mount(PluginSlotOutlet, {
      props: { anchorId: "sidebar.tabs", pluginId: "plugin-a" },
    });
    await flush();
    expect(owned.text()).toContain("nav-page");
    expect(owned.text()).not.toContain("term");
    views.unmount();
    owned.unmount();
  });

  it("skips nav-without-views tabs when excludeMainPane is set", async () => {
    mountSlot("sidebar.tabs", {
      id: "a:main",
      pluginId: "plugin-a",
      mount: (el) => {
        el.textContent = "main";
      },
      chrome: ["nav"],
    });
    mountSlot("sidebar.tabs", {
      id: "b:review",
      pluginId: "plugin-b",
      mount: (el) => {
        el.textContent = "review";
      },
      chrome: ["views"],
    });
    const wrapper = mount(PluginSlotOutlet, {
      props: { anchorId: "sidebar.tabs", excludeMainPane: true },
    });
    await flush();
    expect(wrapper.text()).toContain("review");
    expect(wrapper.text()).not.toContain("main");
    wrapper.unmount();
  });

  it("skips header-only action tabs when excludeMainPane is set", async () => {
    mountSlot("sidebar.tabs", {
      id: "a:open",
      pluginId: "plugin-a",
      mount: (el) => {
        el.textContent = "action";
      },
      chrome: ["header"],
    });
    mountSlot("sidebar.tabs", {
      id: "b:review",
      pluginId: "plugin-b",
      mount: (el) => {
        el.textContent = "review";
      },
      chrome: ["views"],
    });
    const wrapper = mount(PluginSlotOutlet, {
      props: { anchorId: "sidebar.tabs", excludeMainPane: true },
    });
    await flush();
    expect(wrapper.text()).toContain("review");
    expect(wrapper.text()).not.toContain("action");
    wrapper.unmount();
  });

  it("keeps inactive panes mounted but hidden when activeId is set", async () => {
    let mounts = 0;
    mountSlot("sidebar.tabs", {
      id: "a:one",
      pluginId: "plugin-a",
      mount: (el) => {
        mounts += 1;
        el.textContent = "one";
      },
      chrome: ["views"],
    });
    mountSlot("sidebar.tabs", {
      id: "a:two",
      pluginId: "plugin-a",
      mount: (el) => {
        mounts += 1;
        el.textContent = "two";
      },
      chrome: ["views"],
    });
    const wrapper = mount(PluginSlotOutlet, {
      props: { anchorId: "sidebar.tabs", chrome: "views", activeId: "a:one", hostActive: true },
    });
    await flush();
    expect(mounts).toBe(1);
    await wrapper.setProps({ activeId: "a:two" });
    await flush();
    expect(mounts).toBe(2);
    await wrapper.setProps({ activeId: "a:one" });
    await flush();
    expect(mounts).toBe(2);
    wrapper.unmount();
  });
});
