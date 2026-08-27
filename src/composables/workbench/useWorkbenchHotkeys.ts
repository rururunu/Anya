import { nextTick, ref, type Ref } from "vue";

import type ChatInputBar from "@/components/chat/ChatInputBar.vue";

export interface UseWorkbenchHotkeysOptions {
  settingsOpen: Ref<boolean>;
  initializing: Ref<boolean>;
  navigationOpen: Ref<boolean>;
  inputRef: Ref<InstanceType<typeof ChatInputBar> | null>;
  toggleReviewSidebar: () => void;
  openSettings: () => void;
  createQuickConversation: () => Promise<void>;
}

/**
 * Global keyboard shortcuts for the workbench (search, new chat, toggle
 * panes, settings, focus composer, shortcut help) plus the search palette
 * open flag they share with the titlebar search button.
 */
export function useWorkbenchHotkeys(options: UseWorkbenchHotkeysOptions) {
  const {
    settingsOpen,
    initializing,
    navigationOpen,
    inputRef,
    toggleReviewSidebar,
    openSettings,
    createQuickConversation,
  } = options;

  const searchPaletteOpen = ref(false);
  const shortcutHelpOpen = ref(false);

  function openSearchPalette() {
    shortcutHelpOpen.value = false;
    searchPaletteOpen.value = true;
  }

  function focusComposer() {
    shortcutHelpOpen.value = false;
    void nextTick(() => inputRef.value?.focusInput());
  }

  function handleWorkbenchHotkey(event: KeyboardEvent) {
    if (event.key === "Escape" && shortcutHelpOpen.value) {
      event.preventDefault();
      shortcutHelpOpen.value = false;
      return;
    }

    const mod = event.ctrlKey || event.metaKey;
    if (!mod || event.altKey) return;
    if (settingsOpen.value || initializing.value) return;

    const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
    const code = event.code;

    if (key === "k" || (key === "f" && event.shiftKey)) {
      event.preventDefault();
      searchPaletteOpen.value = !searchPaletteOpen.value;
      return;
    }
    if (key === "f") {
      event.preventDefault();
      searchPaletteOpen.value = false;
      return;
    }
    if (key === "n") {
      event.preventDefault();
      void createQuickConversation();
      return;
    }
    if (key === "b") {
      event.preventDefault();
      navigationOpen.value = !navigationOpen.value;
      return;
    }
    if (key === "r") {
      event.preventDefault();
      toggleReviewSidebar();
      return;
    }
    if (key === "," || code === "Comma") {
      event.preventDefault();
      openSettings();
      return;
    }
    if (key === "l") {
      event.preventDefault();
      focusComposer();
      return;
    }
    if (key === "/" || code === "Slash") {
      event.preventDefault();
      shortcutHelpOpen.value = !shortcutHelpOpen.value;
    }
  }

  return {
    searchPaletteOpen,
    shortcutHelpOpen,
    openSearchPalette,
    focusComposer,
    handleWorkbenchHotkey,
  };
}
