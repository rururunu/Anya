import { onUnmounted, ref, type Ref, watch } from "vue";

/** Matches the width the changed-files tree used before it was resizable. */
export const CHANGE_TREE_DEFAULT_WIDTH = 196;
export const CHANGE_TREE_MIN_WIDTH = 148;
export const CHANGE_TREE_MAX_WIDTH = 560;
/** The diff pane always keeps at least this much room beside the tree. */
export const DIFF_VIEW_MIN_WIDTH = 240;
const STORAGE_KEY = "anya.diffChangeTreeWidth.v1";

/**
 * Clamp a requested tree width: never narrower than the minimum, and never
 * wider than the pane can spare for the diff beside it.
 */
export function clampChangeTreeWidth(requested: number, paneWidth: number): number {
  const ceiling = Math.max(
    CHANGE_TREE_MIN_WIDTH,
    Math.min(CHANGE_TREE_MAX_WIDTH, paneWidth - DIFF_VIEW_MIN_WIDTH),
  );
  return Math.min(ceiling, Math.max(CHANGE_TREE_MIN_WIDTH, Math.round(requested)));
}

function readStoredWidth(): number {
  const raw = localStorage.getItem(STORAGE_KEY);
  if (raw === null || raw.trim() === "") return CHANGE_TREE_DEFAULT_WIDTH;
  const stored = Number(raw);
  if (!Number.isFinite(stored)) return CHANGE_TREE_DEFAULT_WIDTH;
  return Math.max(CHANGE_TREE_MIN_WIDTH, stored);
}

export function readStoredChangeTreeWidth() {
  return readStoredWidth();
}

/**
 * Resizable width for the changed-files tree, persisted across sessions.
 * The tree is docked to the right edge of the pane, so dragging left widens it.
 * `paneWidth` is watched so the ceiling follows the review sidebar's own width.
 */
export function useChangeTreeResize(options: { paneWidth: Ref<number> }) {
  const treeWidth = ref(readStoredWidth());
  const treeResizing = ref(false);
  let resizeStartX = 0;
  let resizeStartWidth = CHANGE_TREE_DEFAULT_WIDTH;

  function clampWidth(width: number) {
    return clampChangeTreeWidth(width, options.paneWidth.value);
  }

  function persistWidth() {
    localStorage.setItem(STORAGE_KEY, String(Math.round(treeWidth.value)));
  }

  /** Re-clamp after the pane itself is resized. */
  function updateWidth() {
    treeWidth.value = clampWidth(treeWidth.value);
  }

  function handlePointerMove(event: PointerEvent) {
    event.preventDefault();
    treeWidth.value = clampWidth(resizeStartWidth + (resizeStartX - event.clientX));
  }

  function stopResize() {
    window.removeEventListener("pointermove", handlePointerMove);
    window.removeEventListener("pointerup", stopResize);
    window.removeEventListener("pointercancel", stopResize);
    if (!treeResizing.value) return;
    treeResizing.value = false;
    persistWidth();
  }

  /** Begin a drag from the separator left of the tree. */
  function startResize(event: PointerEvent) {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    (event.currentTarget as HTMLElement | null)?.setPointerCapture?.(event.pointerId);
    resizeStartX = event.clientX;
    resizeStartWidth = treeWidth.value;
    treeResizing.value = true;
    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", stopResize, { once: true });
    window.addEventListener("pointercancel", stopResize, { once: true });
  }

  /** Arrow keys nudge the tree; Left grows it, Right shrinks it. */
  function handleResizeKey(event: KeyboardEvent) {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    treeWidth.value = clampWidth(treeWidth.value + (event.key === "ArrowLeft" ? 16 : -16));
    persistWidth();
  }

  /** Double-click the handle to restore the default width. */
  function resetWidth() {
    treeWidth.value = clampWidth(CHANGE_TREE_DEFAULT_WIDTH);
    persistWidth();
  }

  watch(options.paneWidth, updateWidth);

  onUnmounted(() => {
    stopResize();
  });

  return { treeWidth, treeResizing, startResize, handleResizeKey, resetWidth, updateWidth };
}
