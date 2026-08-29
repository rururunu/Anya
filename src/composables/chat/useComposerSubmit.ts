/**
 * Message delivery, reset, and workspace send options for ChatInputBar.
 */

import { nextTick, type Ref } from "vue";
import {
  parseComposerTextToSegments,
  serializeComposerSegments as serializeSegments,
} from "@/services/chat/composerSegments";
import { parseHashMentions } from "@/services/chat/hashMentions";
import { recordResourceUsages } from "@/services/usage/resourceUsage";
import type ComposerEditable from "@/components/chat/ComposerEditable.vue";
import type { Workspace } from "@/commands/workspace";
import type { CapturedContext } from "@/types/chat";
import type { ComposerSnapshot } from "@/services/chat/composerUndo";

export function useComposerSubmit(options: {
  message: Ref<string>;
  composerRef: Ref<InstanceType<typeof ComposerEditable> | null>;
  composerUndo: { push: (snapshot: ComposerSnapshot) => void };
  captureComposerSnapshot: () => ComposerSnapshot;
  clearComposerSegments: () => void;
  serializeComposerSegments: () => string;
  persistDraft: () => void;
  clearAttachedFiles: () => void;
  clearAttachedImages: () => void;
  clearMentionSuppression: () => void;
  attachedFilesMessagePrefix: () => string;
  formatAttachedImagesForMessage: () => string;
  selectedIndex: Ref<number>;
  overlayWorkspaceOverride: Ref<Workspace | null>;
  currentWorkspace: Ref<Workspace | null>;
  appearance: () => "overlay" | "workbench";
  capturedContext: () => CapturedContext | null | undefined;
  overlayContextWorkspaceRoot: () => string;
  matchKnownWorkspace: (root: string) => Workspace | null;
  emitSubmit: (text: string) => void;
  emitLayoutChange: () => void;
  resetLayoutTracking: () => void;
  resizeWorkbenchInput: () => void;
  resizeComposerInput: () => void;
  focusInput: () => Promise<void>;
  closeModelPicker: () => void;
  closeApprovalMenu: (immediate?: boolean) => void;
  closeChatModeMenu: (immediate?: boolean) => void;
  closeThinkingTierMenu: (immediate?: boolean) => void;
  closeImageGenPicker: () => void;
  workspacePickerOpen: Ref<boolean>;
  workspaceQuickSelectOnly: Ref<boolean>;
  attachPanelOpen: Ref<boolean>;
  resetWorkspaceFilesCache: () => void;
}) {
  function resolveSendWorkspaceOptions(): { workspaceId?: string; quickAsk?: boolean } {
    const active = options.overlayWorkspaceOverride.value ?? options.currentWorkspace.value;
    if (active) {
      return { workspaceId: active.id, quickAsk: false };
    }
    if (options.appearance() === "overlay") {
      const contextRoot = options.overlayContextWorkspaceRoot();
      const matched = contextRoot ? options.matchKnownWorkspace(contextRoot) : null;
      if (matched) {
        return { workspaceId: matched.id, quickAsk: false };
      }
      return { quickAsk: true };
    }
    const contextRoot = options.capturedContext()?.workspace?.root;
    if (contextRoot) {
      return { workspaceId: contextRoot, quickAsk: false };
    }
    return { quickAsk: true };
  }

  function reset() {
    options.clearComposerSegments();
    options.clearAttachedFiles();
    options.clearAttachedImages();
    options.clearMentionSuppression();
    options.selectedIndex.value = 0;
    options.resetLayoutTracking();
    if (options.composerRef.value) {
      options.composerRef.value.setText("");
    } else {
      options.message.value = "";
    }
    options.closeModelPicker();
    options.closeApprovalMenu();
    options.closeChatModeMenu();
    options.closeThinkingTierMenu();
    options.closeImageGenPicker();
    options.workspacePickerOpen.value = false;
    options.workspaceQuickSelectOnly.value = false;
    options.attachPanelOpen.value = false;
    if (options.appearance() === "overlay") {
      options.overlayWorkspaceOverride.value = null;
      options.currentWorkspace.value = null;
      options.resetWorkspaceFilesCache();
    }
    options.emitLayoutChange();
    void nextTick(options.resizeWorkbenchInput);
  }

  function setMessage(text: string) {
    options.composerUndo.push(options.captureComposerSnapshot());
    options.clearComposerSegments();
    options.clearAttachedFiles();
    options.clearAttachedImages();
    options.clearMentionSuppression();
    const parsed = parseComposerTextToSegments(text);
    const flat = serializeSegments(parsed.segments, parsed.liveMessage);
    if (options.composerRef.value) {
      options.composerRef.value.setText(flat);
    } else {
      options.message.value = flat;
    }
    options.emitLayoutChange();
    void nextTick(options.resizeComposerInput);
    void options.focusInput();
  }

  /** Emit the composed message with attachments and clear the composer. */
  function deliverMessage() {
    const text = options.serializeComposerSegments().trim();
    const attachedFileBlocks = options.attachedFilesMessagePrefix();
    const imageTags = options.formatAttachedImagesForMessage();
    const submittedText = [text, attachedFileBlocks, imageTags]
      .filter((part) => part.length > 0)
      .join("\n\n");
    recordResourceUsages(parseHashMentions(submittedText));
    options.emitSubmit(submittedText);
    options.clearComposerSegments();
    options.message.value = "";
    options.persistDraft();
    options.clearAttachedFiles();
    options.clearAttachedImages();
    options.emitLayoutChange();
  }

  return {
    resolveSendWorkspaceOptions,
    reset,
    setMessage,
    deliverMessage,
  };
}
