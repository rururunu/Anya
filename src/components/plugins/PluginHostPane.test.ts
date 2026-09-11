/** @vitest-environment jsdom */
import { describe, expect, it, vi } from "vitest";
import { nextTick } from "vue";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import PluginHostPane from "./PluginHostPane.vue";

async function flushMount() {
  await nextTick();
  await nextTick();
}

describe("PluginHostPane", () => {
  setActivePinia(createPinia());

  it("does not remount when hidden and shown again, even if mount returns no teardown", async () => {
    let mounts = 0;
    const mountFn = (el: HTMLElement) => {
      mounts += 1;
      el.textContent = `session-${mounts}`;
    };

    const wrapper = mount(PluginHostPane, {
      props: { mountFn, active: false, pluginId: "terminal" },
    });
    await flushMount();
    expect(mounts).toBe(0);

    await wrapper.setProps({ active: true });
    await flushMount();
    expect(mounts).toBe(1);
    expect(wrapper.text()).toBe("session-1");

    await wrapper.setProps({ active: false });
    await flushMount();
    expect(mounts).toBe(1);
    expect(wrapper.text()).toBe("session-1");
    expect(wrapper.get(".plugin-host-pane").classes()).toContain("is-inactive");

    await wrapper.setProps({ active: true });
    await flushMount();
    expect(mounts).toBe(1);
    expect(wrapper.text()).toBe("session-1");
  });

  it("does not call mount teardown when toggling active", async () => {
    let mounts = 0;
    let teardowns = 0;
    const mountFn = (el: HTMLElement) => {
      mounts += 1;
      el.textContent = "alive";
      return () => {
        teardowns += 1;
      };
    };

    const wrapper = mount(PluginHostPane, {
      props: { mountFn, active: true, pluginId: "terminal" },
    });
    await flushMount();
    expect(mounts).toBe(1);

    await wrapper.setProps({ active: false });
    await flushMount();
    await wrapper.setProps({ active: true });
    await flushMount();
    expect(mounts).toBe(1);
    expect(teardowns).toBe(0);

    wrapper.unmount();
    expect(teardowns).toBe(1);
  });

  it("flags a mount() that renders nothing, and clears the flag once content appears", async () => {
    vi.useFakeTimers();
    let el!: HTMLElement;
    const mountFn = (root: HTMLElement) => {
      el = root;
    };

    const wrapper = mount(PluginHostPane, {
      props: { mountFn, active: true, pluginId: "video-bg" },
    });
    await flushMount();
    await vi.advanceTimersByTimeAsync(600);
    expect(wrapper.get(".plugin-host-pane").find(".plugin-host-empty").exists()).toBe(true);

    el.appendChild(document.createElement("video"));
    await flushMount();
    expect(wrapper.get(".plugin-host-pane").find(".plugin-host-empty").exists()).toBe(false);

    wrapper.unmount();
    vi.useRealTimers();
  });

  it("mounts when active is omitted (home / workbench main / composer)", async () => {
    const mountFn = (el: HTMLElement) => {
      el.textContent = "shown";
    };
    const wrapper = mount(PluginHostPane, {
      props: { mountFn, pluginId: "video-bg" },
    });
    await flushMount();
    expect(wrapper.text()).toBe("shown");
  });

  it("sizes to content instead of filling a 0-height parent when fit=content", async () => {
    const mountFn = (el: HTMLElement) => {
      el.textContent = "settings";
    };
    const wrapper = mount(PluginHostPane, {
      props: { mountFn, pluginId: "video-bg", fit: "content" },
    });
    await flushMount();
    expect(wrapper.get(".plugin-host-pane").classes()).toContain("is-fit-content");
    expect(wrapper.text()).toBe("settings");
  });
});
