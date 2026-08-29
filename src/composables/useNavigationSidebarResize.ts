import { onUnmounted, ref, type Ref, watch } from "vue";

const DEFAULT_WIDTH = 258;
const MIN_WIDTH = 200;
const MAX_WIDTH = 480;
const RESIZE_HANDLE_WIDTH = 7;
const REVIEW_PANEL_RESIZE_HANDLE_WIDTH = 7;
const CHAT_PANE_MIN_WIDTH = 420;
const STORAGE_KEY = "anya.workbenchNavigationWidth.v1";

export const NAVIGATION_RESIZE_HANDLE_WIDTH = RESIZE_HANDLE_WIDTH;
export const NAVIGATION_SIDEBAR_MIN_WIDTH = MIN_WIDTH;
export const NAVIGATION_SIDEBAR_MAX_WIDTH = MAX_WIDTH;
export const NAVIGATION_SIDEBAR_DEFAULT_WIDTH = DEFAULT_WIDTH;

function readStoredWidth(): number {
  const stored = Number(localStorage.getItem(STORAGE_KEY));
  if (!Number.isFinite(stored)) return DEFAULT_WIDTH;
  return Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, stored));
}

export function readStoredNavigationSidebarWidth() {
  return readStoredWidth();
}

/**
 * Resizable left navigation sidebar width with persistence and keyboard nudging.
 */
export function useNavigationSidebarResize(options: {
  navigationOpen: Ref<boolean>;
  reviewOpen?: Ref<boolean>;
  reviewWidth?: Ref<number>;
  navigationWidth?: Ref<number>;
}) {
  const navigationWidth = options.navigationWidth ?? ref(readStoredWidth());
  const navigationResizing = ref(false);
  let resizeStartX = 0;
  let resizeStartWidth = DEFAULT_WIDTH;

  function reviewChromeWidth() {
    if (!options.reviewOpen?.value) return 0;
    return (options.reviewWidth?.value ?? 0) + REVIEW_PANEL_RESIZE_HANDLE_WIDTH;
  }

  function availableWidth() {
    const contentWidth = document.documentElement.clientWidth;
    return Math.min(
      MAX_WIDTH,
      Math.max(
        MIN_WIDTH,
        contentWidth - reviewChromeWidth() - CHAT_PANE_MIN_WIDTH - RESIZE_HANDLE_WIDTH,
      ),
    );
  }

  function clampWidth(width: number) {
    return Math.min(availableWidth(), Math.max(MIN_WIDTH, width));
  }

  function persistWidth() {
    localStorage.setItem(STORAGE_KEY, String(Math.round(navigationWidth.value)));
  }

  function updateWidth() {
    navigationWidth.value = clampWidth(navigationWidth.value);
  }

  function handlePointerMove(event: PointerEvent) {
    event.preventDefault();
    const requested = resizeStartWidth + event.clientX - resizeStartX;
    navigationWidth.value = clampWidth(requested);
  }

  function stopResize() {
    if (!navigationResizing.value) {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", stopResize);
      window.removeEventListener("pointercancel", stopResize);
      return;
    }
    navigationResizing.value = false;
    window.removeEventListener("pointermove", handlePointerMove);
    window.removeEventListener("pointerup", stopResize);
    window.removeEventListener("pointercancel", stopResize);
    persistWidth();
  }

  function startResize(event: PointerEvent) {
    if (!options.navigationOpen.value || event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    (event.currentTarget as HTMLElement | null)?.setPointerCapture?.(event.pointerId);
    resizeStartX = event.clientX;
    resizeStartWidth = navigationWidth.value;
    navigationResizing.value = true;
    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", stopResize, { once: true });
    window.addEventListener("pointercancel", stopResize, { once: true });
  }

  function handleResizeKey(event: KeyboardEvent) {
    if (!options.navigationOpen.value) return;
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const delta = event.key === "ArrowRight" ? 16 : -16;
    navigationWidth.value = clampWidth(navigationWidth.value + delta);
    persistWidth();
  }

  function resetWidth() {
    navigationWidth.value = clampWidth(DEFAULT_WIDTH);
    persistWidth();
  }

  watch(
    () => [options.navigationOpen.value, options.reviewOpen?.value, options.reviewWidth?.value],
    () => {
      if (options.navigationOpen.value) updateWidth();
    },
  );

  onUnmounted(() => {
    stopResize();
  });

  return {
    navigationWidth,
    navigationResizing,
    startResize,
    handleResizeKey,
    resetWidth,
    updateWidth,
  };
}
