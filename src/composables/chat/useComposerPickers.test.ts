// @vitest-environment happy-dom
import { nextTick, ref, watch } from "vue";
import { describe, expect, it, vi } from "vitest";
import { setOverlayPopupOpen } from "@/services/ipc";
import { useComposerPickers } from "./useComposerPickers";

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({ label: "overlay" }),
}));
vi.mock("@/services/ipc", () => ({ setOverlayPopupOpen: vi.fn() }));

describe("composer popup responsiveness", () => {
  it("switches menus in one Vue update without an intermediate collapsed layout", async () => {
    const pickers = useComposerPickers({
      emitLayoutChange: vi.fn(),
      endModelFilterSession: vi.fn(),
      onHistoryClose: vi.fn(),
      workspacePickerOpen: ref(false),
      workspaceQuickSelectOnly: ref(false),
      attachPanelOpen: ref(false),
      isInteractionRequestOpen: () => false,
    });
    pickers.modelPickerOpen.value = true;
    const layouts: boolean[] = [];
    const stop = watch(
      () => pickers.modelPickerOpen.value || pickers.thinkingTierPickerOpen.value,
      (open) => layouts.push(open),
    );
    pickers.prepareChipPicker();
    pickers.thinkingTierPickerOpen.value = true;
    await nextTick();
    expect(layouts).toEqual([]);
    expect(pickers.modelPickerOpen.value).toBe(false);
    expect(pickers.thinkingTierPickerOpen.value).toBe(true);
    stop();
  });
  it("does not hold up menu layout/focus when the native event queue is stalled", async () => {
    vi.mocked(setOverlayPopupOpen).mockReturnValue(new Promise(() => {}));
    const pickers = useComposerPickers({
      emitLayoutChange: vi.fn(),
      endModelFilterSession: vi.fn(),
      onHistoryClose: vi.fn(),
      workspacePickerOpen: ref(false),
      workspaceQuickSelectOnly: ref(false),
      attachPanelOpen: ref(false),
      isInteractionRequestOpen: () => false,
    });
    let ready = false;
    void pickers.syncPopupState(true).then(() => {
      ready = true;
    });
    await Promise.resolve();
    expect(ready).toBe(true);
    expect(setOverlayPopupOpen).toHaveBeenCalledWith("overlay", true);
  });
});
