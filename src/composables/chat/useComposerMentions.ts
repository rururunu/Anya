/**
 * @/# mention suggestion suppression after Esc dismisses the picker.
 */

import { ref } from "vue";

export function useComposerMentions(options: {
  resetSelectedIndex: () => void;
  emitLayoutChange: () => void;
}) {
  const mentionSuggestSuppressed = ref<{ trigger: "@" | "#"; start: number } | null>(null);

  function isMentionSuggestSuppressed(trigger: "@" | "#", start: number) {
    const suppressed = mentionSuggestSuppressed.value;
    return Boolean(suppressed && suppressed.trigger === trigger && suppressed.start === start);
  }

  /** Record a suppressed mention token so Esc closes the picker without deleting text. */
  function suppressMentionSuggestions(trigger: "@" | "#", start: number) {
    mentionSuggestSuppressed.value = { trigger, start };
    options.resetSelectedIndex();
    options.emitLayoutChange();
  }

  function clearMentionSuppression() {
    mentionSuggestSuppressed.value = null;
  }

  return {
    mentionSuggestSuppressed,
    isMentionSuggestSuppressed,
    suppressMentionSuggestions,
    clearMentionSuppression,
  };
}
