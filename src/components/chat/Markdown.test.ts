// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import Markdown from "./Markdown.vue";
import { readFile } from "@tauri-apps/plugin-fs";
import { computed, nextTick, ref } from "vue";
import { markdownWorkspaceRoot, resolveMarkdownImagePath } from "@/services/chat/localImageSrc";

vi.mock("@tauri-apps/plugin-fs", () => ({
  readFile: vi.fn(async () =>
    new TextEncoder().encode('<svg xmlns="http://www.w3.org/2000/svg"/>'),
  ),
}));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => `http://asset.localhost/${path}`,
}));
vi.mock("@tauri-apps/api/path", () => ({
  tempDir: async () => "C:/Temp",
  join: async (...parts: string[]) => parts.join("/"),
}));
vi.mock("./chartHydration", () => ({ hydrateChartBlocks: vi.fn(), disposeChartBlocks: vi.fn() }));
vi.mock("./mermaidHydration", () => ({
  hydrateMermaidBlocks: vi.fn(),
  disposeMermaidBlocks: vi.fn(),
}));

describe("local markdown images", () => {
  beforeEach(() => vi.clearAllMocks());
  it("resolves general-chat screenshots against the tools' public workspace", async () => {
    const wrapper = mount(Markdown, {
      props: { content: "![Theme](.anya/screenshots/latest.png)" },
    });
    await flushPromises();
    expect(readFile).toHaveBeenCalledWith("C:/Temp/peek-public/.anya/screenshots/latest.png");
    expect(wrapper.get("img").attributes("src")).toMatch(/^data:image\/png;base64,/);
    wrapper.unmount();
  });
  it("reloads relative images when the conversation workspace becomes available", async () => {
    const root = ref("");
    const wrapper = mount(Markdown, {
      props: { content: "![Theme](.anya/screenshots/latest.png)" },
      global: { provide: { [markdownWorkspaceRoot as symbol]: computed(() => root.value) } },
    });
    await flushPromises();
    root.value = "D:/project";
    await flushPromises();
    expect(readFile).toHaveBeenLastCalledWith("D:/project/.anya/screenshots/latest.png");
    await wrapper.get("img").trigger("click");
    expect(wrapper.emitted("previewImage")?.[0]).toEqual([
      "D:/project/.anya/screenshots/latest.png",
    ]);
    wrapper.unmount();
  });
  it("loads relative PDF preview images from the conversation workspace and previews their real path", async () => {
    const wrapper = mount(Markdown, {
      props: { content: "![封面](build/review/p-01.png)" },
      global: { provide: { [markdownWorkspaceRoot as symbol]: computed(() => "C:/workspace") } },
    });
    await flushPromises();
    expect(readFile).toHaveBeenCalledWith("C:/workspace/build/review/p-01.png");
    expect(wrapper.get("img").attributes("src")).toMatch(/^data:image\/png;base64,/);
    await wrapper.get("img").trigger("click");
    expect(wrapper.emitted("previewImage")?.[0]).toEqual(["C:/workspace/build/review/p-01.png"]);
    wrapper.unmount();
  });
  it("normalizes file URLs, escaped filenames and Windows relative paths without changing remote URLs", () => {
    expect(resolveMarkdownImagePath("file:///C:/workspace/my%20image.png")).toBe(
      "C:/workspace/my image.png",
    );
    expect(resolveMarkdownImagePath("path:build/p-01.png", "C:/workspace")).toBe(
      "C:/workspace/build/p-01.png",
    );
    expect(resolveMarkdownImagePath("build\\preview.png", "C:\\workspace")).toBe(
      "C:/workspace/build/preview.png",
    );
    expect(resolveMarkdownImagePath("https://example.com/image.png", "C:/workspace")).toBe(
      "https://example.com/image.png",
    );
  });
  it("batches live code rendering and renders the complete answer immediately when streaming ends", async () => {
    vi.useFakeTimers();
    const wrapper = mount(Markdown, {
      props: { content: "```js\nconst a = 1;\n```", streaming: true },
    });
    expect(wrapper.find(".hljs-keyword").exists()).toBe(false);
    await wrapper.setProps({ content: "```js\nconst b = 2;\n```" });
    expect(wrapper.get("code").text()).toContain("a = 1");
    await vi.advanceTimersByTimeAsync(120);
    expect(wrapper.get("code").text()).toContain("b = 2");
    await wrapper.setProps({ content: "```js\nconst final = 3;\n```", streaming: false });
    expect(wrapper.get("code").text()).toContain("final = 3");
    expect(wrapper.find(".hljs-keyword").exists()).toBe(true);
    wrapper.unmount();
    vi.useRealTimers();
  });
  it("defers growing Markdown work to idle time but commits the final answer immediately", async () => {
    vi.useFakeTimers();
    const idleCallbacks: IdleRequestCallback[] = [];
    vi.stubGlobal(
      "requestIdleCallback",
      vi.fn((callback: IdleRequestCallback) => {
        idleCallbacks.push(callback);
        return idleCallbacks.length;
      }),
    );
    vi.stubGlobal("cancelIdleCallback", vi.fn());
    const wrapper = mount(Markdown, { props: { content: "Start", streaming: true } });
    await wrapper.setProps({ content: "Start **later**" });
    await vi.advanceTimersByTimeAsync(120);
    expect(wrapper.text()).toBe("Start");
    idleCallbacks[0]?.({ didTimeout: false, timeRemaining: () => 20 });
    await nextTick();
    expect(wrapper.text()).toBe("Start later");
    await wrapper.setProps({ content: "Finished **now**", streaming: false });
    expect(wrapper.text()).toBe("Finished now");
    wrapper.unmount();
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });
  it("loads workspace SVG images through the same file reader as user thumbnails", async () => {
    const wrapper = mount(Markdown, { props: { content: "![Rabbit](C:/workspace/rabbit.svg)" } });
    await flushPromises();
    const image = wrapper.get("img");
    expect(readFile).toHaveBeenCalledWith("C:/workspace/rabbit.svg");
    expect(image.attributes("src")).toMatch(/^data:image\/svg\+xml;base64,/);
    expect(image.attributes("data-image-source")).toBe("C:/workspace/rabbit.svg");
    await wrapper.setProps({ content: "![Rabbit](C:/workspace/rabbit.svg)\n\nUpdated answer" });
    await flushPromises();
    expect(readFile).toHaveBeenCalledTimes(1);
    expect(wrapper.get("img").attributes("src")).toMatch(/^data:image\/svg\+xml;base64,/);
    wrapper.unmount();
  });
});
