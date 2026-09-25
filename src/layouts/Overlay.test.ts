// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { PhysicalPosition, PhysicalSize } from "@tauri-apps/api/dpi";
import Overlay from "./Overlay.vue";

const expand = vi.hoisted(() => vi.fn<() => Promise<void>>(async () => {}));

const native = vi.hoisted(() => ({
  label: "overlay",
  isMaximized: vi.fn(async () => false),
  scaleFactor: vi.fn(async () => 1),
  outerPosition: vi.fn(),
  outerSize: vi.fn(),
  setSize: vi.fn<(size: { width: number; height: number }) => Promise<void>>(async () => {}),
  setPosition: vi.fn<(position: { x: number; y: number }) => Promise<void>>(async () => {}),
  setMinSize: vi.fn(async () => {}),
  setMaxSize: vi.fn(async () => {}),
  setResizable: vi.fn(async () => {}),
  setMaximizable: vi.fn(async () => {}),
  listen: vi.fn(async () => () => {}),
}));
vi.mock("@tauri-apps/api/webviewWindow", () => ({ getCurrentWebviewWindow: () => native }));
vi.mock("@tauri-apps/api/window", () => ({ currentMonitor: async () => null }));
vi.mock("@/services/ipc", () => ({
  expandOverlayForChat: expand,
  closeOverlay: vi.fn(),
  setOverlayChatMode: async () => {},
  takeOverlayContext: async () => null,
}));
vi.mock("@/stores/chat", () => ({ useChatStore: () => ({ setOverlayDraftSession: vi.fn() }) }));
vi.mock("@/stores/setting", () => ({ useSettingStore: () => ({ zoom: 100 }) }));
vi.mock("@/components/chat/PeekPanel.vue", () => ({
  default: {
    name: "PeekPanel",
    props: ["mode"],
    emits: ["layoutChange", "enterChat", "close"],
    template: "<div />",
  },
}));

const layout = (height: number, mode = "input") => ({
  mode,
  showSuggestions: false,
  suggestionCount: 0,
  showModelMenu: false,
  modelMenuHeight: 0,
  inputBarHeight: height,
});

describe("overlay native layout", () => {
  let wrapper: ReturnType<typeof mount>;
  beforeEach(async () => {
    vi.clearAllMocks();
    expand.mockResolvedValue(undefined);
    native.outerPosition.mockResolvedValue(new PhysicalPosition(200, 600));
    native.outerSize.mockResolvedValue(new PhysicalSize(640, 56));
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
      queueMicrotask(() => callback(0));
      return 1;
    });
    wrapper = mount(Overlay);
    await flushPromises();
    vi.clearAllMocks();
  });
  afterEach(() => {
    wrapper.unmount();
    vi.unstubAllGlobals();
  });

  it("coalesces rapid input layout changes into the final native size", async () => {
    const panel = wrapper.findComponent({ name: "PeekPanel" });
    for (const height of [86, 116, 146, 176]) panel.vm.$emit("layoutChange", layout(height));
    await flushPromises();
    expect(native.setSize).toHaveBeenCalledTimes(1);
    expect(native.setSize.mock.calls[0]?.[0]).toMatchObject({ width: 640, height: 176 });
    expect(native.setPosition.mock.calls[0]?.[0]).toMatchObject({ x: 200, y: 480 });
    expect(native.setMinSize).not.toHaveBeenCalled();
  });

  it("does not resize twice when chat mode emits its first layout", async () => {
    const panel = wrapper.findComponent({ name: "PeekPanel" });
    panel.vm.$emit("enterChat", "test-session");
    await nextTick();
    panel.vm.$emit("layoutChange", layout(56, "chat"));
    await flushPromises();
    expect(expand).toHaveBeenCalledTimes(1);
    expect(native.setSize).not.toHaveBeenCalled();
    expect(native.setPosition).not.toHaveBeenCalled();
    native.outerSize.mockClear();
    panel.vm.$emit("layoutChange", layout(56, "chat"));
    await flushPromises();
    expect(native.outerSize).not.toHaveBeenCalled();
  });

  it("drops pending input sizes when switching to chat", async () => {
    const panel = wrapper.findComponent({ name: "PeekPanel" });
    panel.vm.$emit("layoutChange", layout(200));
    panel.vm.$emit("enterChat", "test-session");
    panel.vm.$emit("layoutChange", layout(250));
    await flushPromises();
    expect(expand).toHaveBeenCalledTimes(1);
    expect(native.setSize).not.toHaveBeenCalled();
  });

  it("keeps chat content unmounted until native expansion completes", async () => {
    let finish!: () => void;
    expand.mockReturnValueOnce(
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
    );
    const panel = wrapper.findComponent({ name: "PeekPanel" });
    panel.vm.$emit("enterChat", "first-message");
    await flushPromises();
    expect(panel.props("mode")).toBe("input");
    panel.vm.$emit("layoutChange", layout(56));
    finish();
    await flushPromises();
    expect(panel.props("mode")).toBe("chat");
    expect(native.setSize).not.toHaveBeenCalled();
  });

  it("does not enter chat after closing during expansion", async () => {
    let finish!: () => void;
    expand.mockReturnValueOnce(
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
    );
    const panel = wrapper.findComponent({ name: "PeekPanel" });
    panel.vm.$emit("enterChat", "first-message");
    await flushPromises();
    panel.vm.$emit("close");
    finish();
    await flushPromises();
    expect(panel.props("mode")).toBe("input");
  });

  it("falls back to resizing before installing the larger minimum", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    expand.mockRejectedValueOnce(new Error("native resize failed"));
    const panel = wrapper.findComponent({ name: "PeekPanel" });
    panel.vm.$emit("enterChat", "first-message");
    await flushPromises();
    expect(native.setSize).toHaveBeenCalledTimes(1);
    expect(native.setSize.mock.invocationCallOrder[0]).toBeLessThan(
      native.setMinSize.mock.invocationCallOrder[0]!,
    );
    expect(panel.props("mode")).toBe("chat");
    warn.mockRestore();
  });
});
