/** @vitest-environment jsdom */
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { nextTick } from "vue";
import { mount } from "@vue/test-utils";
import WorkbenchPluginBackdrop from "./WorkbenchPluginBackdrop.vue";
import { registerAsset, unregisterPluginAssets } from "@/composables/plugins/assetRegistry";
import { pluginAssetUrl } from "@/services/plugins/ipc";

beforeAll(() => {
  Object.defineProperty(HTMLMediaElement.prototype, "play", {
    configurable: true,
    value: () => Promise.resolve(),
  });
  Object.defineProperty(HTMLMediaElement.prototype, "pause", {
    configurable: true,
    value: () => undefined,
  });
});

describe("pluginAssetUrl", () => {
  it("rewrites anya-plugin:// so WebView media elements can load the file", () => {
    expect(pluginAssetUrl("anya-plugin://localhost/video-bg/data/picked/clip.webm")).toBe(
      "http://anya-plugin.localhost/video-bg/data/picked/clip.webm",
    );
    expect(pluginAssetUrl("https://example.com/a.webm")).toBe("https://example.com/a.webm");
  });
});

describe("WorkbenchPluginBackdrop", () => {
  afterEach(() => unregisterPluginAssets("video-bg"));

  it("renders nothing until a workbench.backdrop override is registered", () => {
    const wrapper = mount(WorkbenchPluginBackdrop);
    expect(wrapper.find(".workbench-plugin-backdrop").exists()).toBe(false);
  });

  it("plays a registered video backdrop from the plugin protocol URL", async () => {
    registerAsset("workbench.backdrop", "video-bg", {
      kind: "video",
      source: "anya-plugin://localhost/video-bg/data/picked/clip.webm",
    });
    const wrapper = mount(WorkbenchPluginBackdrop, { props: { windowFocused: true } });
    await nextTick();
    const video = wrapper.get("video");
    expect(video.attributes("src")).toBe(
      "http://anya-plugin.localhost/video-bg/data/picked/clip.webm",
    );
    expect((video.element as HTMLVideoElement).muted).toBe(true);
    expect(video.attributes("loop")).toBeDefined();
  });
});
