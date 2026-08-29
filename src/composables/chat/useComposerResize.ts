/**
 * Composer textarea auto-grow, caret tracking, and resize scheduling.
 */

import { nextTick, ref, watch, type Ref } from "vue";

type ComposerEditorRef = Ref<{
  el: HTMLElement | null;
  getSelection(): { start: number; end: number };
} | null>;

export function useComposerResize(options: {
  composerRef: ComposerEditorRef;
  message: Ref<string>;
  appearance: Ref<"overlay" | "workbench">;
  emitLayoutChange: () => void;
}) {
  const composerInputMultiline = ref(false);
  const composerCaret = ref(0);

  function syncComposerCaret() {
    const next = options.composerRef.value?.getSelection().start ?? options.message.value.length;
    if (composerCaret.value !== next) {
      composerCaret.value = next;
    }
  }

  function onComposerInput() {
    syncComposerCaret();
    scheduleResizeComposerInput();
  }

  watch(options.message, (value) => {
    if (composerCaret.value > value.length) {
      composerCaret.value = value.length;
    }
  });

  let composerResizeRaf = 0;
  let composerResizePendingForce = false;

  /** Coalesce auto-grow measurements to one layout pass per frame while typing. */
  function scheduleResizeComposerInput(force = false) {
    composerResizePendingForce = composerResizePendingForce || force;
    if (composerResizeRaf) return;
    composerResizeRaf = requestAnimationFrame(() => {
      composerResizeRaf = 0;
      const forceMeasure = composerResizePendingForce;
      composerResizePendingForce = false;
      const heightChanged = resizeComposerInput(forceMeasure);
      if (options.appearance.value === "overlay" && heightChanged) {
        void nextTick(() => options.emitLayoutChange());
      }
    });
  }

  function onComposerCaretChange(caret: number) {
    if (composerCaret.value !== caret) {
      composerCaret.value = caret;
    }
  }

  function resizeComposerInput(force = false): boolean {
    const editor = options.composerRef.value;
    if (!editor) return false;
    const el = editor.el;
    if (!el) return false;
    const lineHeight = 24;
    const maxLines = options.appearance.value === "overlay" ? 4 : 8;
    const minHeight = lineHeight;
    const maxHeight = lineHeight * maxLines;
    const prevHeight = el.offsetHeight;

    if (!force && prevHeight <= minHeight + 4) {
      const probe = el.scrollHeight;
      if (probe <= minHeight + 4) {
        el.style.overflowY = "hidden";
        composerInputMultiline.value = false;
        return false;
      }
    }

    el.style.height = "auto";
    const contentHeight = el.scrollHeight;
    const nextHeight = Math.max(minHeight, Math.min(contentHeight, maxHeight));
    el.style.height = `${nextHeight}px`;
    el.style.overflowY = contentHeight > maxHeight ? "auto" : "hidden";

    composerInputMultiline.value = nextHeight > lineHeight + 4;
    return Math.abs(nextHeight - prevHeight) > 1;
  }

  function resizeWorkbenchInput() {
    resizeComposerInput();
  }

  function disposeComposerResize() {
    if (composerResizeRaf) {
      cancelAnimationFrame(composerResizeRaf);
      composerResizeRaf = 0;
    }
  }

  return {
    composerInputMultiline,
    composerCaret,
    syncComposerCaret,
    onComposerInput,
    onComposerCaretChange,
    scheduleResizeComposerInput,
    resizeComposerInput,
    resizeWorkbenchInput,
    disposeComposerResize,
  };
}
