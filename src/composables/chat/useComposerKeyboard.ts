/**
 * Global keyboard routing and picker focus restoration for ChatInputBar.
 */

import { nextTick, type ComputedRef, type Ref } from "vue";
import type ComposerEditable from "@/components/chat/ComposerEditable.vue";

export function useComposerKeyboard(options: {
  interactivePickerOpen: ComputedRef<boolean>;
  showSuggestions: ComputedRef<boolean>;
  composerRef: Ref<InstanceType<typeof ComposerEditable> | null>;
  handleKeydown: (event: KeyboardEvent) => void;
  focusInput: () => Promise<void>;
}) {
  async function focusAfterDismiss() {
    // Parent-owned pickers close via emitted events; their props update on render.
    await nextTick();
    if (!options.interactivePickerOpen.value && !options.showSuggestions.value) {
      await options.focusInput();
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    const hadPicker = options.interactivePickerOpen.value || options.showSuggestions.value;
    options.handleKeydown(event);
    if (event.key === "Escape" && hadPicker && event.defaultPrevented) {
      event.stopPropagation();
      void focusAfterDismiss();
    }
  }

  /** Route picker keyboard navigation when focus left the composer (e.g. Alt-Tab). */
  function handleGlobalKeydown(event: KeyboardEvent) {
    if (!options.interactivePickerOpen.value && !options.showSuggestions.value) {
      return;
    }
    if (event.target instanceof Node && options.composerRef.value?.el?.contains(event.target)) {
      return;
    }
    handleKeydown(event);
  }

  function restorePickerFocus() {
    if (options.interactivePickerOpen.value) {
      void options.focusInput();
    }
  }

  return {
    handleKeydown,
    handleGlobalKeydown,
    restorePickerFocus,
  };
}
