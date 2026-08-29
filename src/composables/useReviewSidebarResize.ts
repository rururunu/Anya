import { onUnmounted, ref, type Ref, watch } from "vue";

import {
  NAVIGATION_RESIZE_HANDLE_WIDTH,
  NAVIGATION_SIDEBAR_DEFAULT_WIDTH,
} from "@/composables/useNavigationSidebarResize";

const DEFAULT_WIDTH = 520;
const MIN_WIDTH = 320;
const MAX_WIDTH = 900;
const CHAT_PANE_MIN_WIDTH = 420;
const RESIZE_HANDLE_WIDTH = 7;
const STORAGE_KEY = "anya.workbenchReviewWidth.v1";

export const REVIEW_RESIZE_HANDLE_WIDTH = RESIZE_HANDLE_WIDTH;
export const REVIEW_SIDEBAR_MIN_WIDTH = MIN_WIDTH;
export const REVIEW_SIDEBAR_MAX_WIDTH = MAX_WIDTH;

function readStoredWidth(): number {
  const stored = Number(localStorage.getItem(STORAGE_KEY));
  if (!Number.isFinite(stored)) return DEFAULT_WIDTH;
  return Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, stored));
}

export function readStoredReviewSidebarWidth() {
  return readStoredWidth();
}

/**
 * Resizable review sidebar width with persistence and keyboard nudging.
 * `navigationOpen` is observed so the max width tracks the remaining layout.
 */
export function useReviewSidebarResize(options: {
  navigationOpen: Ref<boolean>;
  reviewOpen: Ref<boolean>;
  navigationWidth?: Ref<number>;
  reviewWidth?: Ref<number>;
}) {
  const reviewWidth = options.reviewWidth ?? ref(readStoredWidth());
  const reviewResizing = ref(false);
  let resizeStartX = 0;
  let resizeStartWidth = DEFAULT_WIDTH;

  /** Remaining width available for the review pane given current chrome. */
  function availableWidth() {
    const contentWidth = document.documentElement.clientWidth;
    const nav = options.navigationOpen.value
      ? (options.navigationWidth?.value ?? NAVIGATION_SIDEBAR_DEFAULT_WIDTH) +
        NAVIGATION_RESIZE_HANDLE_WIDTH
      : 0;
    return Math.min(
      MAX_WIDTH,
      Math.max(MIN_WIDTH, contentWidth - nav - CHAT_PANE_MIN_WIDTH - RESIZE_HANDLE_WIDTH),
    );
  }

  function clampWidth(width: number) {
    return Math.min(availableWidth(), Math.max(MIN_WIDTH, width));
  }

  function persistWidth() {
    localStorage.setItem(STORAGE_KEY, String(Math.round(reviewWidth.value)));
  }

  /** Re-clamp after layout changes (e.g. nav toggle or window resize). */
  function updateWidth() {
    reviewWidth.value = clampWidth(reviewWidth.value);
  }

  function handlePointerMove(event: PointerEvent) {
    event.preventDefault();
    const requested = resizeStartWidth + resizeStartX - event.clientX;
    reviewWidth.value = clampWidth(requested);
  }

  function stopResize() {
    if (!reviewResizing.value) {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", stopResize);
      window.removeEventListener("pointercancel", stopResize);
      return;
    }
    reviewResizing.value = false;
    window.removeEventListener("pointermove", handlePointerMove);
    window.removeEventListener("pointerup", stopResize);
    window.removeEventListener("pointercancel", stopResize);
    persistWidth();
  }

  /** Begin a drag resize from the vertical separator. */
  function startResize(event: PointerEvent) {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    (event.currentTarget as HTMLElement | null)?.setPointerCapture?.(event.pointerId);
    resizeStartX = event.clientX;
    resizeStartWidth = reviewWidth.value;
    reviewResizing.value = true;
    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", stopResize, { once: true });
    window.addEventListener("pointercancel", stopResize, { once: true });
  }

  /** Arrow keys nudge the pane; Left grows review, Right shrinks it. */
  function handleResizeKey(event: KeyboardEvent) {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const delta = event.key === "ArrowLeft" ? 16 : -16;
    reviewWidth.value = clampWidth(reviewWidth.value + delta);
    persistWidth();
  }

  /** Double-click the handle to restore the default width. */
  function resetWidth() {
    reviewWidth.value = clampWidth(DEFAULT_WIDTH);
    persistWidth();
  }

  watch(options.navigationOpen, () => {
    if (options.reviewOpen.value) updateWidth();
  });

  onUnmounted(() => {
    stopResize();
  });

  return {
    reviewWidth,
    reviewResizing,
    startResize,
    handleResizeKey,
    resetWidth,
    updateWidth,
  };
}
