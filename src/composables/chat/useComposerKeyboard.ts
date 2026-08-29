/**
 * Global keyboard routing and picker focus restoration for ChatInputBar.
 */

import type { ComputedRef, Ref } from "vue";
import type ComposerEditable from "@/components/chat/ComposerEditable.vue";

export function useComposerKeyboard(options: {
  interactivePickerOpen: ComputedRef<boolean>;
  composerRef: Ref<InstanceType<typeof ComposerEditable> | null>;
  handleKeydown: (event: KeyboardEvent) => void;
  focusInput: () => Promise<void>;
}) {
  /** Route picker keyboard navigation when focus left the composer (e.g. Alt-Tab). */
  function handleGlobalKeydown(event: KeyboardEvent) {
    if (!options.interactivePickerOpen.value) {
      return;
    }
    if (event.target instanceof Node && options.composerRef.value?.el?.contains(event.target)) {
      return;
    }
    options.handleKeydown(event);
  }

  function restorePickerFocus() {
    if (options.interactivePickerOpen.value) {
      void options.focusInput();
    }
  }

  return {
    handleGlobalKeydown,
    restorePickerFocus,
  };
}
