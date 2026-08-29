/**
 * Footer chip picker open state and dismiss helpers for ChatInputBar.
 */

import { ref, type Ref } from "vue";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { setOverlayPopupOpen } from "@/services/ipc";
import type { ImageGenFieldId } from "@/services/chat/imageGenMode";

export function useComposerPickers(options: {
  emitLayoutChange: () => void;
  endModelFilterSession: () => void;
  onHistoryClose: () => void;
  workspacePickerOpen: Ref<boolean>;
  workspaceQuickSelectOnly: Ref<boolean>;
  attachPanelOpen: Ref<boolean>;
  isInteractionRequestOpen: () => boolean;
}) {
  const modelPickerOpen = ref(false);
  const modelPickerProvider = ref<string | null>(null);
  const approvalPickerOpen = ref(false);
  const thinkingTierPickerOpen = ref(false);
  const thinkingPickerMode = ref<"slider" | "list">("slider");
  const chatModePickerOpen = ref(false);
  const imageGenPickerOpen = ref<ImageGenFieldId | null>(null);

  /** Sync overlay popup-open flag with the native shell. */
  async function syncPopupState(open: boolean) {
    const windowLabel = getCurrentWebviewWindow().label;
    try {
      await Promise.race([
        setOverlayPopupOpen(windowLabel, open),
        new Promise<void>((resolve) => {
          window.setTimeout(resolve, 800);
        }),
      ]);
    } catch (error) {
      console.error("set_overlay_popup_open failed:", error);
    }
  }

  function closeChipPickers() {
    if (modelPickerOpen.value) {
      options.endModelFilterSession();
    }
    modelPickerOpen.value = false;
    approvalPickerOpen.value = false;
    chatModePickerOpen.value = false;
    thinkingTierPickerOpen.value = false;
    imageGenPickerOpen.value = null;
  }

  function anyChipStillOpen() {
    return (
      modelPickerOpen.value ||
      approvalPickerOpen.value ||
      chatModePickerOpen.value ||
      thinkingTierPickerOpen.value ||
      Boolean(imageGenPickerOpen.value)
    );
  }

  function closeModelPicker() {
    if (!modelPickerOpen.value) return;
    options.endModelFilterSession();
    modelPickerOpen.value = false;
    modelPickerProvider.value = null;
    if (!anyChipStillOpen()) {
      void syncPopupState(false);
    }
    options.emitLayoutChange();
  }

  function closeApprovalPicker() {
    if (!approvalPickerOpen.value) return;
    approvalPickerOpen.value = false;
    if (!anyChipStillOpen()) {
      void syncPopupState(false);
    }
    options.emitLayoutChange();
  }

  function closeChatModePicker() {
    if (!chatModePickerOpen.value) return;
    chatModePickerOpen.value = false;
    if (!anyChipStillOpen()) {
      void syncPopupState(false);
    }
    options.emitLayoutChange();
  }

  function closeThinkingTierPicker() {
    if (!thinkingTierPickerOpen.value) return;
    thinkingTierPickerOpen.value = false;
    if (!anyChipStillOpen()) {
      void syncPopupState(false);
    }
    options.emitLayoutChange();
  }

  function closeImageGenPicker() {
    if (!imageGenPickerOpen.value) return;
    imageGenPickerOpen.value = null;
    if (!anyChipStillOpen()) {
      void syncPopupState(false);
    }
    options.emitLayoutChange();
  }

  function closeApprovalMenu(_immediate = false) {
    closeApprovalPicker();
  }

  function closeChatModeMenu(_immediate = false) {
    closeChatModePicker();
  }

  function closeThinkingTierMenu(_immediate = false) {
    closeThinkingTierPicker();
  }

  function dismissFloatingPickers() {
    const hadChip = anyChipStillOpen();
    const hadWorkspace = options.workspacePickerOpen.value;
    const hadAttach = options.attachPanelOpen.value;
    if (!hadChip && !hadWorkspace && !hadAttach) return;
    closeChipPickers();
    if (hadWorkspace) {
      options.workspacePickerOpen.value = false;
      options.workspaceQuickSelectOnly.value = false;
    }
    if (hadAttach) {
      options.attachPanelOpen.value = false;
    }
    void syncPopupState(false);
    options.emitLayoutChange();
  }

  function handleDocumentPointerDown(event: PointerEvent) {
    if (event.button !== 0) return;
    if (
      !anyChipStillOpen() &&
      !options.workspacePickerOpen.value &&
      !options.attachPanelOpen.value
    ) {
      return;
    }
    if (options.isInteractionRequestOpen()) return;
    const target = event.target;
    if (!(target instanceof Element)) return;
    if (target.closest(".command-list, .thinking-effort-panel")) return;
    if (target.closest("[data-picker-trigger]")) return;
    dismissFloatingPickers();
  }

  /** Close competing pickers before opening a footer chip menu. */
  async function prepareChipPicker() {
    options.onHistoryClose();
    options.workspacePickerOpen.value = false;
    options.workspaceQuickSelectOnly.value = false;
    options.attachPanelOpen.value = false;
    closeChipPickers();
  }

  return {
    modelPickerOpen,
    modelPickerProvider,
    approvalPickerOpen,
    thinkingTierPickerOpen,
    thinkingPickerMode,
    chatModePickerOpen,
    imageGenPickerOpen,
    syncPopupState,
    closeChipPickers,
    closeModelPicker,
    closeApprovalPicker,
    closeChatModePicker,
    closeThinkingTierPicker,
    closeImageGenPicker,
    closeApprovalMenu,
    closeChatModeMenu,
    closeThinkingTierMenu,
    dismissFloatingPickers,
    handleDocumentPointerDown,
    prepareChipPicker,
  };
}
