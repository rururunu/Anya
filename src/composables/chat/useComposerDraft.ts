/**
 * Composer text, selection chips, undo stack, and per-session draft persistence.
 */

import { computed, nextTick, ref, watch, type Ref } from "vue";
import { useDebounceFn } from "@vueuse/core";
import {
  parseComposerTextToSegments,
  serializeComposerSegments as serializeSegments,
  type ComposerSegment,
} from "@/services/chat/composerSegments";
import { createComposerUndoStack, type ComposerSnapshot } from "@/services/chat/composerUndo";
import { useChatStore } from "@/stores/chat";

export function useComposerDraft(options: { sessionId: Ref<string>; onAfterUndo: () => void }) {
  const chatStore = useChatStore();
  const message = ref("");
  const composerSegments = ref<ComposerSegment[]>([]);
  const composerUndo = createComposerUndoStack();

  const hasComposerChips = computed(() =>
    composerSegments.value.some((seg) => seg.kind === "selection"),
  );

  /** Flatten selection chips and live textarea into the outbound message body. */
  function serializeComposerSegments() {
    return serializeSegments(composerSegments.value, message.value);
  }

  /** Capture current composer state for programmatic-edit undo. */
  function captureComposerSnapshot(): ComposerSnapshot {
    return {
      message: message.value,
      segments: composerSegments.value.map((seg) => ({ ...seg })),
    };
  }

  /** Restore the most recent snapshot; returns false when the stack is empty. */
  function undoComposerSnapshot(): boolean {
    const snapshot = composerUndo.pop();
    if (!snapshot) return false;
    message.value = snapshot.message;
    composerSegments.value = snapshot.segments;
    void nextTick(() => options.onAfterUndo());
    return true;
  }

  function clearComposerSegments() {
    composerSegments.value = [];
  }

  /** Persist draft for the given session; no-op when sessionId is empty. */
  function persistDraft(
    sessionId = options.sessionId.value,
    draft = serializeComposerSegments(),
    immediate = false,
  ) {
    if (!sessionId) return;
    chatStore.setComposeDraft(sessionId, draft, immediate ? { persistImmediate: true } : undefined);
  }

  /** Load draft from the session compose cache into the composer. */
  function loadDraft() {
    if (!options.sessionId.value) {
      clearComposerSegments();
      message.value = "";
      return;
    }
    const compose = chatStore.ensureCompose(options.sessionId.value);
    const parsed = parseComposerTextToSegments(compose.draft || "");
    message.value = serializeSegments(parsed.segments, parsed.liveMessage);
    composerSegments.value = [];
  }

  const persistDraftDebounced = useDebounceFn((sessionId: string) => {
    if (!sessionId || sessionId !== options.sessionId.value) return;
    persistDraft(sessionId);
  }, 1000);

  watch([message, composerSegments], () => {
    if (!options.sessionId.value) return;
    persistDraftDebounced(options.sessionId.value);
  });

  return {
    message,
    composerSegments,
    composerUndo,
    hasComposerChips,
    serializeComposerSegments,
    captureComposerSnapshot,
    undoComposerSnapshot,
    clearComposerSegments,
    persistDraft,
    loadDraft,
  };
}
