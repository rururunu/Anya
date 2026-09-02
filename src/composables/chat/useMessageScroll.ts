import { nextTick, onMounted, onUnmounted, watch, type ComputedRef, type Ref } from "vue";
import type { ChatMessage } from "@/types/chat";
import { gsapScrollContainerTo } from "@/services/motion/gsapPresets";
import type { ActiveUserMessageMetrics } from "@/composables/chat/useMessagePreviewRail";

const SCROLL_NEAR_BOTTOM_THRESHOLD = 96;

export function useMessageScroll(options: {
  listRef: Ref<HTMLElement | null>;
  stickToBottom: Ref<boolean>;
  messages: ComputedRef<ChatMessage[]>;
  displayItems: ComputedRef<unknown[]>;
  activeUserMessageId: Ref<string>;
  railRef: Ref<HTMLElement | null>;
  updateActiveUserMessage: (metrics: ActiveUserMessageMetrics) => void;
}) {
  let cachedLastMessageEl: HTMLElement | null = null;
  let scrollRaf = 0;
  let bottomScrollRaf = 0;
  let resizeScrollRaf = 0;
  let resizeObserver: ResizeObserver | null = null;
  /* scrollTop we last set ourselves; scroll events landing there are not user intent. */
  let programmaticScrollTop: number | null = null;
  /* After the user deliberately scrolls up, ignore re-sticking for a moment so smooth
     wheel/trackpad scrolling that briefly passes "near bottom" doesn't snap them back. */
  let userScrollUpUntil = 0;
  const USER_SCROLL_GRACE_MS = 700;

  function setScrollTop(element: HTMLElement, top: number) {
    programmaticScrollTop = top;
    element.scrollTop = top;
  }

  function isNearBottom(element: HTMLElement) {
    const padBottom = Number.parseFloat(getComputedStyle(element).paddingBottom) || 0;
    const contentBottom = element.scrollHeight - padBottom;
    const viewportBottom = element.scrollTop + element.clientHeight;
    return contentBottom - viewportBottom <= SCROLL_NEAR_BOTTOM_THRESHOLD;
  }

  function refreshMessageDomCache() {
    const element = options.listRef.value;
    if (!element) {
      cachedLastMessageEl = null;
      return;
    }
    const items = element.querySelectorAll<HTMLElement>(".message-item");
    cachedLastMessageEl = items[items.length - 1] ?? null;
  }

  function isLastTurnOnScreen(element: HTMLElement) {
    const lastItem = cachedLastMessageEl;
    if (!lastItem || !element.contains(lastItem)) {
      refreshMessageDomCache();
    }
    const resolved = cachedLastMessageEl;
    if (!resolved) return false;
    const listRect = element.getBoundingClientRect();
    const lastRect = resolved.getBoundingClientRect();
    return lastRect.top < listRect.bottom && lastRect.bottom <= listRect.bottom + 48;
  }

  function activeUserMetrics(element: HTMLElement): ActiveUserMessageMetrics {
    return {
      isNearBottom: isNearBottom(element),
      isLastTurnOnScreen: isLastTurnOnScreen(element),
      cachedLastMessageEl,
    };
  }

  function handleScroll() {
    if (scrollRaf) return;
    scrollRaf = requestAnimationFrame(() => {
      scrollRaf = 0;
      const element = options.listRef.value;
      if (!element) return;
      // Our own scrollTo() also fires scroll events; those must not flip the sticky flag.
      const isProgrammatic =
        programmaticScrollTop !== null && Math.abs(element.scrollTop - programmaticScrollTop) < 2;
      programmaticScrollTop = null;
      if (!isProgrammatic) {
        const wantsBottom = isNearBottom(element) || isLastTurnOnScreen(element);
        if (!wantsBottom) {
          options.stickToBottom.value = false;
        } else if (performance.now() >= userScrollUpUntil) {
          options.stickToBottom.value = true;
        }
      }
      options.updateActiveUserMessage(activeUserMetrics(element));
    });
  }

  /** Wheel / trackpad: scrolling up is an explicit "let me read" and unsticks at once. */
  function handleWheel(event: WheelEvent) {
    if (event.deltaY < 0) {
      options.stickToBottom.value = false;
      userScrollUpUntil = performance.now() + USER_SCROLL_GRACE_MS;
    }
  }

  /** Keyboard scrolling inside the list: same intent as wheel-up. */
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowUp" || event.key === "PageUp" || event.key === "Home") {
      options.stickToBottom.value = false;
      userScrollUpUntil = performance.now() + USER_SCROLL_GRACE_MS;
    }
  }

  function scrollToMessage(messageId: string) {
    const container = options.listRef.value;
    const node = container?.querySelector<HTMLElement>(
      `[data-message-id="${CSS.escape(messageId)}"]`,
    );
    if (!container || !node) return;
    options.stickToBottom.value = false;
    options.activeUserMessageId.value = messageId;
    gsapScrollContainerTo(container, node, { offsetY: 42 });
    options.railRef.value?.focus();
  }

  function scrollToLatest() {
    const element = options.listRef.value;
    if (!element) return;
    options.stickToBottom.value = true;
    userScrollUpUntil = 0;
    element.scrollTo({ top: element.scrollHeight, behavior: "smooth" });
    options.updateActiveUserMessage(activeUserMetrics(element));
  }

  /** Pin the latest user turn on-screen while streaming when stick-to-bottom is active. */
  async function scrollToBottomIfNeeded() {
    await nextTick();
    refreshMessageDomCache();
    const element = options.listRef.value;
    if (!element) return;

    if (!options.stickToBottom.value) {
      options.updateActiveUserMessage(activeUserMetrics(element));
      return;
    }

    const padBottom = Number.parseFloat(getComputedStyle(element).paddingBottom) || 0;
    const maxScroll = element.scrollHeight - element.clientHeight;
    if (maxScroll <= 1) {
      setScrollTop(element, 0);
      options.updateActiveUserMessage(activeUserMetrics(element));
      return;
    }

    const users = element.querySelectorAll<HTMLElement>(".message-item.user");
    const lastUser = users[users.length - 1];
    if (lastUser) {
      const listTop = element.getBoundingClientRect().top;
      const userTop = lastUser.getBoundingClientRect().top - listTop + element.scrollTop;
      const contentBottom = element.scrollHeight - padBottom;
      const turnHeight = contentBottom - userTop;

      if (turnHeight <= element.clientHeight - 4) {
        setScrollTop(element, users.length <= 1 ? 0 : Math.max(0, userTop - 8));
        options.updateActiveUserMessage(activeUserMetrics(element));
        return;
      }
    }

    setScrollTop(element, maxScroll);
    options.updateActiveUserMessage(activeUserMetrics(element));
  }

  function scheduleScrollToBottomIfNeeded() {
    if (bottomScrollRaf) return;
    bottomScrollRaf = requestAnimationFrame(() => {
      bottomScrollRaf = 0;
      void scrollToBottomIfNeeded();
    });
  }

  watch(options.displayItems, () => {
    void nextTick(refreshMessageDomCache);
  });

  watch(
    () => options.messages.value.length,
    (length, previousLength) => {
      if (length > (previousLength ?? 0)) options.stickToBottom.value = true;
    },
  );

  watch(
    () => {
      const messages = options.messages.value;
      const last = messages[messages.length - 1];
      if (!last) return "0";
      const tools =
        last.toolActivities
          ?.map((activity) => `${activity.id}:${activity.status}:${activity.detail?.length ?? 0}`)
          .join(",") ?? "";
      const asks = last.askUserAnswer?.map((answer) => answer.selected.join(",")).join(";") ?? "";
      return `${messages.length}|${last.id}:${last.content.length}:${last.reasoning?.length ?? 0}:${tools}:${asks}:${last.status}:${last.activityStatus ?? ""}`;
    },
    () => void scheduleScrollToBottomIfNeeded(),
    { immediate: true },
  );

  onMounted(() => {
    const element = options.listRef.value;
    if (!element || typeof ResizeObserver === "undefined") return;
    resizeObserver = new ResizeObserver(() => {
      if (resizeScrollRaf) return;
      resizeScrollRaf = requestAnimationFrame(() => {
        resizeScrollRaf = 0;
        const el = options.listRef.value;
        if (!el || el.clientHeight < 8) return;
        void scrollToBottomIfNeeded();
      });
    });
    resizeObserver.observe(element);
  });

  onUnmounted(() => {
    resizeObserver?.disconnect();
    resizeObserver = null;
    if (scrollRaf) cancelAnimationFrame(scrollRaf);
    if (bottomScrollRaf) cancelAnimationFrame(bottomScrollRaf);
    if (resizeScrollRaf) cancelAnimationFrame(resizeScrollRaf);
  });

  return {
    handleScroll,
    handleWheel,
    handleKeydown,
    scrollToMessage,
    scrollToLatest,
  };
}
