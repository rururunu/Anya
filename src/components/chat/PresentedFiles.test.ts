// @vitest-environment jsdom
import { mount, flushPromises } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import PresentedFiles from "./PresentedFiles.vue";
import type { ChatMessage } from "@/types/chat";

const actions = vi.hoisted(() => ({ open: vi.fn(), reveal: vi.fn() }));
vi.mock("@/services/ipc", () => ({
  openInDefaultApp: actions.open,
  revealInExplorer: actions.reveal,
}));
vi.mock("@/stores/setting", () => ({ useSettingStore: () => ({ language: "zh-CN" }) }));
function message(names = ["报告.pptx"]): ChatMessage {
  return {
    id: "a",
    sessionId: "s",
    role: "assistant",
    status: "done",
    content: "",
    timestamp: 1,
    toolActivities: [
      {
        id: "p",
        toolName: "present",
        title: "交付",
        kind: "tool",
        success: true,
        status: "done",
        result: JSON.stringify({
          version: 1,
          files: names.map((name) => ({
            path: name,
            absolutePath: `C:\\work\\${name}`,
            name,
            size: 1234,
            description: "最终交付",
          })),
        }),
      },
    ],
  };
}
beforeEach(() => {
  actions.open.mockReset().mockResolvedValue(undefined);
  actions.reveal.mockReset().mockResolvedValue(undefined);
});
describe("PresentedFiles", () => {
  it("opens a document and reveals it using the verified absolute path", async () => {
    const wrapper = mount(PresentedFiles, { props: { message: message() } });
    await wrapper.get(".delivery-main").trigger("click");
    await flushPromises();
    expect(actions.open).toHaveBeenCalledWith("C:\\work\\报告.pptx");
    await wrapper.get(".delivery-reveal").trigger("click");
    await flushPromises();
    expect(actions.reveal).toHaveBeenCalledWith("C:\\work\\报告.pptx");
    wrapper.unmount();
  });
  it("previews images through Anya's existing sidebar event", async () => {
    const wrapper = mount(PresentedFiles, { props: { message: message(["效果图.png"]) } });
    await wrapper.get(".delivery-main").trigger("click");
    expect(wrapper.emitted("previewImage")).toEqual([["C:\\work\\效果图.png"]]);
    expect(actions.open).not.toHaveBeenCalled();
    wrapper.unmount();
  });
  it("expands additional files without creating cards from prose", async () => {
    const wrapper = mount(PresentedFiles, {
      props: { message: message(["1.pdf", "2.pdf", "3.pdf", "4.pdf", "5.pdf"]) },
    });
    expect(wrapper.findAll(".delivery-card")).toHaveLength(4);
    await wrapper.get(".delivery-toggle").trigger("click");
    expect(wrapper.findAll(".delivery-card")).toHaveLength(5);
    await wrapper.setProps({
      message: { ...message(), toolActivities: [], content: "[报告](report.pptx)" },
    });
    expect(wrapper.findAll(".delivery-card")).toHaveLength(0);
    wrapper.unmount();
  });
  it("shows a recoverable error when the file was moved or deleted", async () => {
    actions.open.mockRejectedValueOnce(new Error("missing"));
    const wrapper = mount(PresentedFiles, { props: { message: message() } });
    await wrapper.get(".delivery-main").trigger("click");
    await flushPromises();
    expect(wrapper.get('[role="alert"]').text()).toContain("无法打开文件");
    await wrapper.get(".delivery-main").trigger("click");
    await flushPromises();
    expect(wrapper.find('[role="alert"]').exists()).toBe(false);
    wrapper.unmount();
  });
});
