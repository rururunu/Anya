/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import PluginSidebarIcon from "./PluginSidebarIcon.vue";

describe("PluginSidebarIcon", () => {
  it("renders a plugin image URL", () => {
    const wrapper = mount(PluginSidebarIcon, {
      props: { name: "anya-plugin://localhost/demo/ui/icon.svg", size: 18 },
    });
    expect(wrapper.get("img").attributes("src")).toContain("anya-plugin.localhost");
  });

  it("maps a known glyph when no image URL is given", () => {
    const wrapper = mount(PluginSidebarIcon, {
      props: { name: "terminal" },
    });
    expect(wrapper.find("img").exists()).toBe(false);
    expect(wrapper.find("svg").exists()).toBe(true);
  });
});
