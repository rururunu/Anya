// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { nextTick } from "vue";
import ThemeEditor from "./ThemeEditor.vue";
import type { CustomThemeConfig } from "@/types/setting";

const { saveCustomTheme } = vi.hoisted(() => ({ saveCustomTheme: vi.fn() }));
vi.mock("@/stores/setting", () => ({
  useSettingStore: () => ({ language: "zh-CN", saveCustomTheme }),
}));
const source: CustomThemeConfig = {
  id: "original",
  name: "Original",
  mode: "light",
  tokens: { "--peek-user-bubble-bg": "#112233" },
  updatedAt: 1,
};
let wrapper: ReturnType<typeof mount>;
const button = (text: string) =>
  Array.from(document.querySelectorAll("button")).find(
    (entry) => entry.textContent?.trim() === text,
  )!;
async function click(text: string) {
  button(text).click();
  await nextTick();
}
async function input(token: string, value: string) {
  const node = document.getElementById(token) as HTMLInputElement;
  node.value = value;
  node.dispatchEvent(new Event("change", { bubbles: true }));
  await nextTick();
}
beforeEach(() => {
  saveCustomTheme.mockReset().mockResolvedValue(undefined);
  wrapper = mount(ThemeEditor, { props: { theme: source }, attachTo: document.body });
});
afterEach(() => {
  wrapper.unmount();
  document.body.innerHTML = "";
});

describe("theme editor drafts", () => {
  it("blocks saving while a visual field contains an invalid declaration", async () => {
    await click("用户消息");
    await input("--peek-user-bubble-bg", "red;display:none");
    await click("保存并应用");
    expect(saveCustomTheme).not.toHaveBeenCalled();
    await input("--peek-user-bubble-bg", "#aabbcc");
    await click("保存并应用");
    await flushPromises();
    expect(saveCustomTheme).toHaveBeenCalledTimes(1);
  });
  it("edits a region without changing settings and supports undo/reset", async () => {
    await click("用户消息");
    await input("--peek-user-bubble-bg", "#aabbcc");
    expect(
      document
        .querySelector<HTMLElement>(".theme-preview")
        ?.style.getPropertyValue("--peek-user-bubble-bg"),
    ).toBe("#aabbcc");
    expect(source.tokens["--peek-user-bubble-bg"]).toBe("#112233");
    expect(saveCustomTheme).not.toHaveBeenCalled();
    await click("撤销");
    expect((document.getElementById("--peek-user-bubble-bg") as HTMLInputElement).value).toBe(
      "#112233",
    );
    await click("重做");
    await click("重置此区域");
    expect((document.getElementById("--peek-user-bubble-bg") as HTMLInputElement).value).toBe("");
    await click("保存并应用");
    await flushPromises();
    expect(saveCustomTheme.mock.calls[0][0].tokens).not.toHaveProperty("--peek-user-bubble-bg");
  });

  it("removes live preview on cancel without persisting drafts", async () => {
    const toggle = document.querySelector<HTMLInputElement>(".live-toggle input")!;
    toggle.click();
    await nextTick();
    expect(document.head.querySelector("[data-theme-editor-preview]")?.textContent).toContain(
      "--peek-user-bubble-bg:#112233",
    );
    await click("取消");
    expect(wrapper.emitted("close")).toHaveLength(1);
    wrapper.unmount();
    expect(document.head.querySelector("[data-theme-editor-preview]")).toBeNull();
    expect(saveCustomTheme).not.toHaveBeenCalled();
  });

  it("keeps the editor open after a failed save and can retry as a copy", async () => {
    saveCustomTheme.mockRejectedValueOnce(new Error("disk unavailable"));
    await click("保存并应用");
    await flushPromises();
    expect(document.querySelector('[role="alert"]')?.textContent).toContain("disk unavailable");
    expect(wrapper.emitted("close")).toBeUndefined();
    await click("另存为新主题");
    await flushPromises();
    expect(saveCustomTheme.mock.calls[1][0].id).not.toBe(source.id);
    expect(source.name).toBe("Original");
    expect(wrapper.emitted("close")).toHaveLength(1);
  });

  it("rejects malformed advanced JSON and preserves explicit low contrast colors", async () => {
    await click("JSON");
    const textarea = document.getElementById("theme-json") as HTMLTextAreaElement;
    textarea.value = "{broken";
    textarea.dispatchEvent(new Event("input", { bubbles: true }));
    await click("保存并应用");
    expect(saveCustomTheme).not.toHaveBeenCalled();
    textarea.value = JSON.stringify({ "--peek-text": "#ffffff", "--peek-list-bg": "#ffffff" });
    textarea.dispatchEvent(new Event("input", { bubbles: true }));
    await click("应用到预览");
    expect(document.querySelector(".contrast-warning")?.textContent).toContain("1.0:1");
    await click("保存并应用");
    await flushPromises();
    expect(saveCustomTheme.mock.calls[0][0].tokens["--peek-text"]).toBe("#ffffff");
  });
});
