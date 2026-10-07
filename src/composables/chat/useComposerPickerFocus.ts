import { nextTick, watch, type Ref } from "vue";
import { useEventListener } from "@vueuse/core";

/** One focus session spans switches between footer menus. */
export function useComposerPickerFocus(options: {
  open: Ref<boolean>;
  editor: () => HTMLElement | null | undefined;
  focusEditor: () => Promise<void>;
}) {
  let restoreEditor = false;
  let capturedTrigger = false;
  let revision = 0;

  useEventListener(
    document,
    "pointerdown",
    (event) => {
      if (options.open.value || !(event.target instanceof Element)) return;
      if (!event.target.closest("[data-picker-trigger]")) return;
      capturedTrigger = true;
      restoreEditor = document.activeElement === options.editor();
    },
    { capture: true },
  );

  watch(options.open, async (open) => {
    const current = ++revision;
    if (open) {
      if (!capturedTrigger) restoreEditor = document.activeElement === options.editor();
      capturedTrigger = false;
      return;
    }
    const shouldRestore = restoreEditor;
    restoreEditor = false;
    await nextTick();
    if (!shouldRestore || current !== revision || options.open.value) return;
    const active = document.activeElement;
    // An outside click may intentionally focus another control.
    if (
      active &&
      active !== document.body &&
      active !== options.editor() &&
      !active.closest(".picker-content, [data-picker-trigger]")
    )
      return;
    await options.focusEditor();
  });
}
