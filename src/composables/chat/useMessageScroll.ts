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
  sessionId?: ComputedRef<string | undefined> | Ref<string | undefined>;
  isSending?: ComputedRef<boolean> | Ref<boolean>;
  updateActiveUserMessage: (metrics: ActiveUserMessageMetrics) => void;
}) {
  let cachedLastMessageEl: HTMLElement | null = null;
  let scrollRaf = 0;
  let bottomScrollRaf = 0;
  let resizeScrollRaf = 0;
  let resizeObserver: ResizeObserver | null = null;
  let programmaticScrollTop: number | null = null;
  let isSmoothScrollingToBottom = false;
  let sessionSwitchPending = true;
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

      if (sessionSwitchPending || isSmoothScrollingToBottom) {
        if (isNearBottom(element)) {
          sessionSwitchPending = false;
          isSmoothScrollingToBottom = false;
        }
        options.updateActiveUserMessage(activeUserMetrics(element));
        return;
      }

      const isProgrammatic =
        programmaticScrollTop !== null && Math.abs(element.scrollTop - programmaticScrollTop) <= 4;
      if (isProgrammatic) {
        programmaticScrollTop = null;
      } else {
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
      sessionSwitchPending = false;
      isSmoothScrollingToBottom = false;
      options.stickToBottom.value = false;
      userScrollUpUntil = performance.now() + USER_SCROLL_GRACE_MS;
    }
  }

  /** Keyboard scrolling inside the list: same intent as wheel-up. */
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowUp" || event.key === "PageUp" || event.key === "Home") {
      sessionSwitchPending = false;
      isSmoothScrollingToBottom = false;
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
    sessionSwitchPending = false;
    isSmoothScrollingToBottom = false;
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
    sessionSwitchPending = false;
    isSmoothScrollingToBottom = true;
    element.scrollTo({ top: element.scrollHeight, behavior: "smooth" });
    options.updateActiveUserMessage(activeUserMetrics(element));
  }

  function getTurnSpacer(element: HTMLElement): number {
    const spacer = element.querySelector<HTMLElement>(".turn-spacer");
    if (!spacer || spacer.style.display === "none") return 0;
    return Number.parseFloat(spacer.style.height) || 0;
  }

  function setTurnSpacer(element: HTMLElement, height: number) {
    const spacer = element.querySelector<HTMLElement>(".turn-spacer");
    if (!spacer) return;
    if (height <= 0) {
      spacer.style.display = "none";
      spacer.style.height = "0px";
    } else {
      spacer.style.display = "block";
      spacer.style.height = `${Math.round(height)}px`;
    }
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
    const currentSpacer = getTurnSpacer(element);
    const scrollHeightWithoutSpacer = element.scrollHeight - currentSpacer;
    const maxScrollWithoutSpacer = Math.max(0, scrollHeightWithoutSpacer - element.clientHeight);

    if (maxScrollWithoutSpacer <= 1) {
      setTurnSpacer(element, 0);
      setScrollTop(element, 0);
      sessionSwitchPending = false;
      options.updateActiveUserMessage(activeUserMetrics(element));
      return;
    }

    if (options.isSending?.value) {
      const users = element.querySelectorAll<HTMLElement>(".message-item.user");
      const lastUser = users[users.length - 1];
      if (lastUser) {
        const listTop = element.getBoundingClientRect().top;
        const userTop = lastUser.getBoundingClientRect().top - listTop + element.scrollTop;
        const targetViewportTop = Math.min(
          userTop,
          Math.max(64, Math.min(140, Math.round(element.clientHeight * 0.14))),
        );
        const targetScrollTop = Math.max(0, userTop - targetViewportTop);

        const contentBottom = scrollHeightWithoutSpacer - padBottom;
        const visibleHeight = element.clientHeight - padBottom;
        const viewportContentBottom = contentBottom - targetScrollTop;

        if (viewportContentBottom <= visibleHeight - 12) {
          const deficit = Math.max(0, targetScrollTop - maxScrollWithoutSpacer);
          setTurnSpacer(element, deficit);
          setScrollTop(element, targetScrollTop);
          options.updateActiveUserMessage(activeUserMetrics(element));
          return;
        }
      }
    }

    setTurnSpacer(element, 0);
    const targetScroll = Math.max(0, element.scrollHeight - element.clientHeight);
    setScrollTop(element, targetScroll);
    sessionSwitchPending = false;
    options.updateActiveUserMessage(activeUserMetrics(element));
  }

  function scheduleScrollToBottomIfNeeded() {
    if (bottomScrollRaf) return;
    bottomScrollRaf = requestAnimationFrame(() => {
      bottomScrollRaf = 0;
      void scrollToBottomIfNeeded();
    });
  }

  function resetForSessionSwitch() {
    options.stickToBottom.value = true;
    userScrollUpUntil = 0;
    cachedLastMessageEl = null;
    programmaticScrollTop = null;
    isSmoothScrollingToBottom = false;
    sessionSwitchPending = true;
    const element = options.listRef.value;
    if (element) {
      setTurnSpacer(element, 0);
    }
    void scheduleScrollToBottomIfNeeded();
  }

  if (options.sessionId) {
    watch(options.sessionId, () => {
      resetForSessionSwitch();
    });
  }

  if (options.isSending) {
    watch(options.isSending, (sending, wasSending) => {
      if (wasSending && !sending) {
        void scheduleScrollToBottomIfNeeded();
      }
    });
  }

  watch(options.displayItems, () => {
    void nextTick(() => {
      refreshMessageDomCache();
      if (options.stickToBottom.value) {
        void scrollToBottomIfNeeded();
      }
    });
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
      if (!last) return `${options.sessionId?.value ?? ""}:0`;
      const tools =
        last.toolActivities
          ?.map((activity) => `${activity.id}:${activity.status}:${activity.detail?.length ?? 0}`)
          .join(",") ?? "";
      const asks = last.askUserAnswer?.map((answer) => answer.selected.join(",")).join(";") ?? "";
      return `${options.sessionId?.value ?? ""}|${messages.length}|${last.id}:${last.content.length}:${last.reasoning?.length ?? 0}:${tools}:${asks}:${last.status}:${last.activityStatus ?? ""}`;
    },
    () => void scheduleScrollToBottomIfNeeded(),
    { immediate: true },
  );

  onMounted(() => {
    sessionSwitchPending = true;
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
    void scheduleScrollToBottomIfNeeded();
  });

  onUnmounted(() => {
    resizeObserver?.disconnect();
    resizeObserver = null;
    if (scrollRaf) cancelAnimationFrame(scrollRaf);
    if (bottomScrollRaf) cancelAnimationFrame(bottomScrollRaf);
    if (resizeScrollRaf) cancelAnimationFrame(resizeScrollRaf);
    const element = options.listRef.value;
    if (element) setTurnSpacer(element, 0);
  });

  return {
    handleScroll,
    handleWheel,
    handleKeydown,
    scrollToMessage,
    scrollToLatest,
  };
}
