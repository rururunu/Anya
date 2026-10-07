// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { PhysicalPosition, PhysicalSize } from "@tauri-apps/api/dpi";
import { currentMonitor } from "@tauri-apps/api/window";
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
vi.mock("@tauri-apps/api/window", () => ({ currentMonitor: vi.fn(async () => null) }));
vi.mock("@/services/ipc", () => ({
  expandOverlayForChat: expand,
  resizeOverlayInput: async (height: number, zoom: number) => {
    await native.setSize({ width: 640 * zoom, height: height * zoom });
    await native.setPosition({ x: 200, y: 600 });
  },
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
    emits: ["layoutChange", "enterChat", "close", "contentHeight"],
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
    native.outerSize.mockResolvedValue(new PhysicalSize(640, 86));
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
      queueMicrotask(() => callback(0));
      return 1;
    });
    wrapper = mount(Overlay);
    await flushPromises();
    vi.clearAllMocks();
  });
  afterEach(() => {
    vi.mocked(currentMonitor).mockResolvedValue(null);
    vi.useRealTimers();
    wrapper.unmount();
    vi.unstubAllGlobals();
  });

  it("uses half the monitor height as the growth limit", async () => {
    vi.mocked(currentMonitor).mockResolvedValue({
      position: new PhysicalPosition(0, 0),
      size: new PhysicalSize(2560, 1440),
      scaleFactor: 1,
      name: "test",
    });
    native.outerPosition.mockResolvedValue(new PhysicalPosition(200, 100));
    const panel = wrapper.findComponent({ name: "PeekPanel" });
    panel.vm.$emit("enterChat", "half-screen");
    await flushPromises();
    vi.useFakeTimers();
    panel.vm.$emit("contentHeight", 1000);
    await vi.advanceTimersByTimeAsync(100);
    expect(native.setSize).toHaveBeenLastCalledWith(expect.objectContaining({ height: 720 }));
  });

  it("grows upward with content, coalesces updates and stops at the maximum", async () => {
    const panel = wrapper.findComponent({ name: "PeekPanel" });
    panel.vm.$emit("enterChat", "growing-thread");
    await flushPromises();
    vi.useFakeTimers();
    panel.vm.$emit("contentHeight", 280);
    panel.vm.$emit("contentHeight", 320);
    await vi.advanceTimersByTimeAsync(100);
    expect(native.setSize).toHaveBeenLastCalledWith(expect.objectContaining({ height: 320 }));
    expect(native.setPosition).toHaveBeenLastCalledWith(expect.objectContaining({ y: 366 }));
    panel.vm.$emit("contentHeight", 900);
    await vi.advanceTimersByTimeAsync(100);
    expect(native.setSize).toHaveBeenLastCalledWith(expect.objectContaining({ height: 520 }));
    native.setSize.mockClear();
    panel.vm.$emit("contentHeight", 1200);
    await vi.advanceTimersByTimeAsync(100);
    expect(native.setSize).not.toHaveBeenCalled();
  });

  it("coalesces rapid input layout changes into the final native size", async () => {
    const panel = wrapper.findComponent({ name: "PeekPanel" });
    for (const height of [86, 116, 146, 176]) panel.vm.$emit("layoutChange", layout(height));
    await flushPromises();
    expect(native.setSize).toHaveBeenCalledTimes(1);
    expect(native.setSize.mock.calls[0]?.[0]).toMatchObject({ width: 640, height: 176 });
    expect(native.setPosition.mock.calls[0]?.[0]).toMatchObject({ x: 200, y: 600 });
    expect(native.setMinSize).not.toHaveBeenCalled();
  });

  it("keeps the input popup top fixed when growing near the monitor bottom", async () => {
    vi.mocked(currentMonitor).mockResolvedValue({
      position: new PhysicalPosition(0, 0),
      size: new PhysicalSize(1920, 700),
      scaleFactor: 1,
      name: "test",
    });
    const panel = wrapper.findComponent({ name: "PeekPanel" });
    panel.vm.$emit("layoutChange", layout(180));
    await flushPromises();
    expect(native.setPosition).toHaveBeenLastCalledWith(expect.objectContaining({ y: 600 }));
  });

  it("retries the same input height after a native resize fails", async () => {
    const panel = wrapper.findComponent({ name: "PeekPanel" });
    native.setSize.mockRejectedValueOnce(new Error("resize failed"));
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    try {
      panel.vm.$emit("layoutChange", layout(140));
      await flushPromises();
      panel.vm.$emit("layoutChange", layout(140));
      await flushPromises();
      expect(native.setSize).toHaveBeenCalledTimes(2);
    } finally {
      consoleError.mockRestore();
    }
  });

  it("adds height when the rendered dock extends below the WebView viewport", async () => {
    const panel = wrapper.findComponent({ name: "PeekPanel" });
    panel.element.classList.add("composer-dock");
    vi.spyOn(panel.element, "getBoundingClientRect").mockReturnValue({ bottom: 120 } as DOMRect);
    const querySelector = document.querySelector.bind(document);
    const query = vi
      .spyOn(document, "querySelector")
      .mockImplementation((selector) =>
        selector === ".composer-dock" ? panel.element : querySelector(selector),
      );
    vi.stubGlobal("innerHeight", 100);
    try {
      panel.vm.$emit("layoutChange", layout(100));
      await flushPromises();
      expect(native.setSize.mock.calls.some(([size]) => size.height > 100)).toBe(true);
    } finally {
      query.mockRestore();
    }
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
