import { ref, type ComputedRef, type Ref } from "vue";
import type { ChatMessage } from "@/types/chat";

export type MessagePreviewUserContent = ReturnType<
  typeof import("@/services/chat/selectionAttachment").parseSelectionAttachment
>;

export type ActiveUserMessageMetrics = {
  isNearBottom: boolean;
  isLastTurnOnScreen: boolean;
  cachedLastMessageEl: HTMLElement | null;
};

export function useMessagePreviewRail(options: {
  listRef: Ref<HTMLElement | null>;
  userMessages: ComputedRef<ChatMessage[]>;
  stickToBottom: Ref<boolean>;
  findOpen: Ref<boolean>;
  userContent: (message: ChatMessage) => MessagePreviewUserContent;
}) {
  const railRef = ref<HTMLElement | null>(null);
  const activeUserMessageId = ref("");

  /** Highlight the user turn that intersects the message list viewport. */
  function updateActiveUserMessage(metrics: ActiveUserMessageMetrics) {
    const element = options.listRef.value;
    const users = options.userMessages.value;
    if (!element || !users.length) {
      activeUserMessageId.value = "";
      return;
    }
    if (options.stickToBottom.value || metrics.isNearBottom || metrics.isLastTurnOnScreen) {
      activeUserMessageId.value = users[users.length - 1]?.id ?? "";
      return;
    }

    const listRect = element.getBoundingClientRect();
    const inset = 8;
    let active = users[0]?.id ?? "";
    for (let index = 0; index < users.length; index += 1) {
      const message = users[index];
      const node = element.querySelector<HTMLElement>(
        `[data-message-id="${CSS.escape(message.id)}"]`,
      );
      if (!node) continue;
      const next = users[index + 1]
        ? element.querySelector<HTMLElement>(
            `[data-message-id="${CSS.escape(users[index + 1].id)}"]`,
          )
        : null;
      const turnTop = node.getBoundingClientRect().top;
      let turnBottom = listRect.bottom;
      if (next) {
        turnBottom = next.getBoundingClientRect().top;
      } else {
        const lastItem = metrics.cachedLastMessageEl;
        if (lastItem && element.contains(lastItem)) {
          turnBottom = lastItem.getBoundingClientRect().bottom;
        }
      }
      const intersects = turnBottom > listRect.top + inset && turnTop < listRect.bottom - inset;
      if (intersects) active = message.id;
    }
    activeUserMessageId.value = active;
  }

  function messagePreview(message: ChatMessage) {
    const parsed = options.userContent(message);
    const compact = parsed.message.replace(/\s+/g, " ").trim();
    if (compact) {
      return compact.length > 72 ? `${compact.slice(0, 72)}...` : compact;
    }
    if (parsed.attachedFiles?.length) {
      return parsed.attachedFiles.map((file) => file.name).join(", ");
    }
    if (parsed.images?.length) {
      return parsed.images.length === 1 ? "image" : `${parsed.images.length} images`;
    }
    return "";
  }

  function jumpRail(delta: number, scrollToMessage: (messageId: string) => void) {
    const users = options.userMessages.value;
    if (!users.length) return;
    const index = users.findIndex((message) => message.id === activeUserMessageId.value);
    const from = index < 0 ? (delta > 0 ? -1 : users.length) : index;
    const next = users[Math.min(users.length - 1, Math.max(0, from + delta))];
    if (next) scrollToMessage(next.id);
  }

  function onRailKeydown(event: KeyboardEvent, scrollToMessage: (messageId: string) => void) {
    if (options.findOpen.value) return;
    if (event.key === "ArrowUp") {
      event.preventDefault();
      jumpRail(-1, scrollToMessage);
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      jumpRail(1, scrollToMessage);
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      const first = options.userMessages.value[0];
      if (first) scrollToMessage(first.id);
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      const last = options.userMessages.value[options.userMessages.value.length - 1];
      if (last) scrollToMessage(last.id);
    }
  }

  return {
    railRef,
    activeUserMessageId,
    updateActiveUserMessage,
    messagePreview,
    onRailKeydown,
  };
}
