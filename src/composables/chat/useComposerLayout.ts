/**
 * Composer shell layout: picker height measurement, overlay chrome signals, chip positioning.
 */

import {
  computed,
  getCurrentScope,
  nextTick,
  onScopeDispose,
  ref,
  type ComputedRef,
  type Ref,
} from "vue";

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
  floatingPickerTarget?: Ref<HTMLElement | null>;
  pickerInFlow?: ComputedRef<boolean>;
  pickerDirection?: Ref<"up" | "down">;
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
  const chipPickerPosition = ref({
    left: 8,
    bottom: "42px",
    top: "auto",
    width: 280,
    maxHeight: 280,
  });
  const chipPickerStyle = computed(() => ({
    "--chip-picker-left": `${chipPickerPosition.value.left}px`,
    "--chip-picker-bottom": chipPickerPosition.value.bottom,
    "--chip-picker-top": chipPickerPosition.value.top,
    "--chip-picker-width": `${chipPickerPosition.value.width}px`,
    "--chip-picker-max-height": `${chipPickerPosition.value.maxHeight}px`,
  }));

  let layoutChangeFlushScheduled = false;
  let lastEmittedChromeHeight = 0;
  let lastEmittedPickerHeight = 0;
  let lastLayoutChromeSignature = "";
  let measuredPickerHeight = 0;
  let pickerMeasureScheduled = false;
  let observedPicker: HTMLElement | null = null;
  const pickerObserver =
    typeof ResizeObserver !== "undefined"
      ? new ResizeObserver(() => schedulePickerHeightMeasure())
      : null;
  if (getCurrentScope()) onScopeDispose(() => pickerObserver?.disconnect());

  /** Position a footer chip picker relative to its trigger button. */
  async function positionChipPicker(button: HTMLElement | null, preferredWidth: number) {
    if (options.appearance.value !== "workbench" && !options.floatingPickerTarget?.value) return;
    await nextTick();
    const shell = options.shellRef.value;
    if (!shell || !button) return;
    const host = options.floatingPickerTarget?.value ?? shell;
    const shellRect = host.getBoundingClientRect();
    const zoom = host.offsetWidth > 0 ? shellRect.width / host.offsetWidth : 1;
    const buttonRect = button.getBoundingClientRect();
    const edge = 8;
    const hostWidth = shellRect.width / zoom;
    const width = Math.max(120, Math.min(preferredWidth, hostWidth - edge * 2));
    const naturalLeft = (buttonRect.left - shellRect.left) / zoom;
    const down = options.floatingPickerTarget?.value && options.pickerDirection?.value === "down";
    chipPickerPosition.value = {
      left: Math.min(hostWidth - width - edge, Math.max(edge, naturalLeft)),
      bottom: down ? "auto" : `${(shellRect.bottom - buttonRect.top) / zoom + 6}px`,
      top: down ? `${(buttonRect.bottom - shellRect.top) / zoom + 6}px` : "auto",
      width,
      maxHeight: down
        ? 280
        : Math.max(0, Math.min(280, (buttonRect.top - shellRect.top) / zoom - 14)),
    };
    emitLayoutChange();
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
      await new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      );
      pickerMeasureScheduled = false;
      updateInteractionPickerMaxHeight();
      const pickerRows = options.activePickerRowCount();
      if (pickerRows <= 0) {
        measuredPickerHeight = 0;
        return;
      }
      const list = (
        options.floatingPickerTarget?.value ?? options.shellRef.value
      )?.querySelector<HTMLElement>(".command-list");
      if (list !== observedPicker) {
        pickerObserver?.disconnect();
        observedPicker = list ?? null;
        if (list) pickerObserver?.observe(list);
      }
      if (list && options.interactionRequestOpen.value) {
        list.scrollTop = 0;
      }
      // Measure content, even while the native input window is still small.
      // Viewport-capped offsetHeight otherwise locks the popup into that size.
      const height = list
        ? options.floatingPickerTarget?.value
          ? Math.min(
              280,
              Math.max(
                list.offsetHeight,
                list.scrollHeight + list.offsetHeight - list.clientHeight,
              ),
            )
          : list.offsetHeight
        : 0;
      if (height <= 0) return;
      if (Math.abs(height - measuredPickerHeight) < 1) return;
      measuredPickerHeight = height;
      flushLayoutChange();
    });
  }

  function flushLayoutChange(force = false) {
    const pickerRows = options.activePickerRowCount();
    const signature = options.layoutChromeSignature();
    const pickerStateChanged = signature !== lastLayoutChromeSignature;
    if (pickerRows <= 0 || pickerStateChanged) {
      measuredPickerHeight = 0;
    }

    const naturalPickerHeight =
      pickerRows > 0 ? Math.max(measuredPickerHeight, options.estimatePickerHeight(pickerRows)) : 0;
    const pickerHeight = options.pickerInFlow?.value
      ? 0
      : options.floatingPickerTarget?.value && pickerRows > 0
        ? Math.min(280, naturalPickerHeight) +
          Math.max(
            14,
            options.pickerDirection?.value === "down"
              ? (parseFloat(chipPickerPosition.value.top) || 0) -
                  (options.shellRef.value?.offsetHeight ?? 0) +
                  8
              : 14,
          )
        : naturalPickerHeight;

    const shell = options.shellRef.value;
    const inputBar = shell?.querySelector<HTMLElement>(".input-bar");
    const chromeHeight =
      options.appearance.value === "overlay" && shell
        ? shell.offsetHeight
        : (inputBar?.offsetHeight ?? 0);

    const chromeChanged = Math.abs(chromeHeight - lastEmittedChromeHeight) > 1;
    const pickerHeightChanged = Math.abs(pickerHeight - lastEmittedPickerHeight) > 1;

    if (!force && !pickerStateChanged && !chromeChanged && !pickerHeightChanged) {
      if (pickerRows > 0) schedulePickerHeightMeasure();
      return;
    }

    lastLayoutChromeSignature = signature;
    lastEmittedChromeHeight = chromeHeight;
    lastEmittedPickerHeight = pickerHeight;

    const layoutReason: ComposerLayoutReason =
      pickerStateChanged || pickerHeightChanged ? "picker" : chromeChanged ? "chrome" : "other";

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
    lastEmittedPickerHeight = 0;
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
