// @vitest-environment happy-dom
import { computed, ref } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useComposerLayout } from "./useComposerLayout";

function fixture(
  direction: "up" | "down",
  zoom = 1,
  rows = 0,
  estimatedHeight = 0,
  inFlow = false,
) {
  const shell = document.createElement("div");
  const host = document.createElement("div");
  const button = document.createElement("button");
  Object.defineProperty(host, "offsetWidth", { value: 640 });
  host.getBoundingClientRect = () =>
    ({
      left: 0,
      top: 0,
      right: 640 * zoom,
      bottom: 520 * zoom,
      width: 640 * zoom,
      height: 520 * zoom,
    }) as DOMRect;
  button.getBoundingClientRect = () =>
    ({ left: 550 * zoom, top: 86 * zoom, bottom: 110 * zoom }) as DOMRect;
  const onLayoutChange = vi.fn();
  const layout = useComposerLayout({
    appearance: ref("overlay"),
    shellRef: ref(shell),
    floatingPickerTarget: ref(host),
    pickerInFlow: computed(() => inFlow),
    pickerDirection: ref(direction),
    interactionRequestOpen: computed(() => false),
    layoutChromeSignature: () => "model",
    activePickerRowCount: () => rows,
    estimatePickerHeight: () => estimatedHeight,
    buildLayoutPayload: ({ pickerHeight, chromeHeight, layoutReason }) => ({
      showSuggestions: false,
      suggestionCount: 0,
      showModelMenu: false,
      modelMenuHeight: 0,
      askUserRowCount: 0,
      pickerRowCount: 0,
      pickerHeight,
      inputBarHeight: chromeHeight,
      layoutReason,
    }),
    onLayoutChange,
  });
  return { layout, button, shell, host, onLayoutChange };
}

afterEach(() => vi.unstubAllGlobals());

describe("overlay floating picker placement", () => {
  it("does not reserve a second floating area for completion lists already inside the input", () => {
    const { layout, shell, onLayoutChange } = fixture("down", 1, 5, 180, true);
    Object.defineProperty(shell, "offsetHeight", { value: 270 });
    layout.flushLayoutChange();
    expect(onLayoutChange).toHaveBeenCalledWith(
      expect.objectContaining({ pickerHeight: 0, inputBarHeight: 270 }),
    );
  });
  it("opens below the trigger and clamps a right-hand model menu inside the window", async () => {
    const { layout, button } = fixture("down");
    await layout.positionChipPicker(button, 340);
    expect(layout.chipPickerStyle.value).toMatchObject({
      "--chip-picker-left": "292px",
      "--chip-picker-top": "116px",
      "--chip-picker-bottom": "auto",
    });
  });

  it("opens above the trigger during a conversation", async () => {
    const { layout, button } = fixture("up");
    await layout.positionChipPicker(button, 160);
    expect(layout.chipPickerStyle.value).toMatchObject({
      "--chip-picker-left": "472px",
      "--chip-picker-top": "auto",
      "--chip-picker-bottom": "440px",
    });
  });

  it("converts zoomed viewport coordinates back to design pixels", async () => {
    const { layout, button } = fixture("down", 1.5);
    await layout.positionChipPicker(button, 340);
    expect(layout.chipPickerStyle.value).toMatchObject({
      "--chip-picker-left": "292px",
      "--chip-picker-top": "116px",
      "--chip-picker-width": "340px",
    });
  });

  it("expands again when the rendered menu is taller than its row estimate", async () => {
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
      callback(0);
      return 1;
    });
    const { layout, host, onLayoutChange } = fixture("down", 1, 3, 118);
    const list = document.createElement("div");
    list.className = "command-list";
    // Initial viewport clips the list to 72px, but descriptions need 230px.
    Object.defineProperties(list, {
      offsetHeight: { value: 72 },
      clientHeight: { value: 70 },
      scrollHeight: { value: 228 },
    });
    host.append(list);
    layout.flushLayoutChange();
    await vi.waitFor(() => {
      expect(onLayoutChange).toHaveBeenLastCalledWith(
        expect.objectContaining({
          pickerHeight: 244,
          layoutReason: "picker",
        }),
      );
    });
    expect(onLayoutChange).toHaveBeenCalledTimes(2);
  });

  it("reserves the actual gap below the trigger and caps long menus for scrolling", async () => {
    const { layout, button, shell, onLayoutChange } = fixture("down", 1, 20, 700);
    Object.defineProperty(shell, "offsetHeight", { value: 100 });
    await layout.positionChipPicker(button, 340);
    layout.flushLayoutChange();
    expect(onLayoutChange).toHaveBeenLastCalledWith(expect.objectContaining({ pickerHeight: 304 }));
    expect(layout.chipPickerStyle.value["--chip-picker-max-height"]).toBe("280px");
  });
});
