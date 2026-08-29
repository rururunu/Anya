/**
 * Composer shell layout: picker height measurement, overlay chrome signals, chip positioning.
 */

import { computed, nextTick, ref, type ComputedRef, type Ref } from "vue";

export type ComposerLayoutReason = "picker" | "chrome" | "other";

export interface ComposerLayoutPayload {
  showSuggestions: boolean;
  suggestionCount: number;
  showModelMenu: boolean;
  modelMenuHeight: number;
  askUserRowCount: number;
  pickerRowCount: number;
  pickerHeight?: number;
  hasImages?: boolean;
  hasFiles?: boolean;
  isPreviewOpen?: boolean;
  inputBarHeight?: number;
  layoutReason?: ComposerLayoutReason;
}

export function useComposerLayout(options: {
  appearance: Ref<"overlay" | "workbench">;
  shellRef: Ref<HTMLElement | null>;
  interactionRequestOpen: ComputedRef<boolean>;
  layoutChromeSignature: () => string;
  activePickerRowCount: () => number;
  estimatePickerHeight: (pickerRows: number) => number;
  buildLayoutPayload: (args: {
    pickerRows: number;
    pickerHeight: number;
    chromeHeight: number;
    layoutReason: ComposerLayoutReason;
  }) => ComposerLayoutPayload;
  onLayoutChange: (payload: ComposerLayoutPayload) => void;
}) {
  const chipPickerPosition = ref({ left: 8, bottom: 42, width: 280 });
  const chipPickerStyle = computed(() => ({
    "--chip-picker-left": `${chipPickerPosition.value.left}px`,
    "--chip-picker-bottom": `${chipPickerPosition.value.bottom}px`,
    "--chip-picker-width": `${chipPickerPosition.value.width}px`,
  }));

  let layoutChangeFlushScheduled = false;
  let lastEmittedChromeHeight = 0;
  let lastLayoutChromeSignature = "";
  let measuredPickerHeight = 0;
  let pickerMeasureScheduled = false;

  /** Position a footer chip picker relative to its trigger button. */
  async function positionChipPicker(button: HTMLElement | null, preferredWidth: number) {
    if (options.appearance.value !== "workbench") return;
    await nextTick();
    const shell = options.shellRef.value;
    if (!shell || !button) return;
    const shellRect = shell.getBoundingClientRect();
    const buttonRect = button.getBoundingClientRect();
    const edge = 8;
    const width = Math.max(120, Math.min(preferredWidth, shellRect.width - edge * 2));
    const naturalLeft = buttonRect.left - shellRect.left;
    chipPickerPosition.value = {
      left: Math.min(shellRect.width - width - edge, Math.max(edge, naturalLeft)),
      bottom: shellRect.bottom - buttonRect.top + 4,
      width,
    };
  }

  /** Cap ask/approval panels so sticky headers are not clipped. */
  function updateInteractionPickerMaxHeight() {
    const shell = options.shellRef.value;
    if (!shell) return;
    if (!options.interactionRequestOpen.value) {
      shell.style.removeProperty("--interaction-picker-max-height");
      return;
    }

    const pane =
      shell.closest<HTMLElement>(".conversation-pane") ||
      shell.closest<HTMLElement>(".peek-panel") ||
      shell.closest<HTMLElement>(".composer-dock");
    const inputBar = shell.querySelector<HTMLElement>(".input-bar");
    const inputHeight = inputBar?.getBoundingClientRect().height ?? 96;
    const topReserve = 20;
    const bottomReserve = 12;

    let available = Math.floor(window.innerHeight * 0.48);
    if (pane) {
      const paneHeight = pane.getBoundingClientRect().height;
      available = Math.floor(paneHeight - inputHeight - topReserve - bottomReserve);
    }

    const capped = Math.max(180, Math.min(available, Math.floor(window.innerHeight * 0.62)));
    shell.style.setProperty("--interaction-picker-max-height", `${capped}px`);
  }

  function schedulePickerHeightMeasure() {
    if (pickerMeasureScheduled) return;
    pickerMeasureScheduled = true;
    void nextTick(async () => {
      pickerMeasureScheduled = false;
      await new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      );
      updateInteractionPickerMaxHeight();
      const pickerRows = options.activePickerRowCount();
      if (pickerRows <= 0) {
        measuredPickerHeight = 0;
        return;
      }
      const list = document.querySelector(".chat-input-shell .command-list") as HTMLElement | null;
      if (list && options.interactionRequestOpen.value) {
        list.scrollTop = 0;
      }
      const height = list?.offsetHeight ?? 0;
      if (height <= 0) return;
      if (Math.abs(height - measuredPickerHeight) < 1) return;
      measuredPickerHeight = height;
      flushLayoutChange();
    });
  }

  function flushLayoutChange(force = false) {
    const pickerRows = options.activePickerRowCount();
    if (pickerRows <= 0) {
      measuredPickerHeight = 0;
    }

    const pickerHeight =
      pickerRows > 0 ? Math.max(measuredPickerHeight, options.estimatePickerHeight(pickerRows)) : 0;

    const shell = options.shellRef.value;
    const inputBar = shell?.querySelector<HTMLElement>(".input-bar");
    const chromeHeight =
      options.appearance.value === "overlay" && shell
        ? shell.offsetHeight
        : (inputBar?.offsetHeight ?? 0);

    const signature = options.layoutChromeSignature();
    const pickerStateChanged = signature !== lastLayoutChromeSignature;
    const chromeChanged = Math.abs(chromeHeight - lastEmittedChromeHeight) > 1;

    if (!force && !pickerStateChanged && !chromeChanged) {
      if (pickerRows > 0) schedulePickerHeightMeasure();
      return;
    }

    lastLayoutChromeSignature = signature;
    lastEmittedChromeHeight = chromeHeight;

    const layoutReason: ComposerLayoutReason = pickerStateChanged
      ? "picker"
      : chromeChanged
        ? "chrome"
        : "other";

    options.onLayoutChange(
      options.buildLayoutPayload({ pickerRows, pickerHeight, chromeHeight, layoutReason }),
    );

    if (pickerRows > 0) {
      schedulePickerHeightMeasure();
    }
  }

  /** Schedule a layout emit on the next tick; coalesces rapid calls. */
  function emitLayoutChange(force = false) {
    if (layoutChangeFlushScheduled) return;
    layoutChangeFlushScheduled = true;
    void nextTick(() => {
      layoutChangeFlushScheduled = false;
      flushLayoutChange(force);
    });
  }

  function resetLayoutTracking() {
    lastEmittedChromeHeight = 0;
    lastLayoutChromeSignature = "";
    measuredPickerHeight = 0;
  }

  return {
    chipPickerPosition,
    chipPickerStyle,
    positionChipPicker,
    emitLayoutChange,
    flushLayoutChange,
    updateInteractionPickerMaxHeight,
    resetLayoutTracking,
  };
}
