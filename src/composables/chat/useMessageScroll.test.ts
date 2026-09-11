/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { computed, createApp, defineComponent, nextTick, ref } from "vue";
import { useMessageScroll } from "./useMessageScroll";
import type { ChatMessage } from "@/types/chat";

function withSetup<T>(composable: () => T): [T, ReturnType<typeof createApp>] {
  let result!: T;
  const app = createApp(
    defineComponent({
      setup() {
        result = composable();
        return () => {};
      },
    }),
  );
  app.mount(document.createElement("div"));
  return [result, app];
}

function makeMockElement(options?: {
  scrollHeight?: number;
  clientHeight?: number;
  scrollTop?: number;
}): HTMLElement {
  const el = document.createElement("div");
  let st = options?.scrollTop ?? 0;
  Object.defineProperty(el, "scrollHeight", {
    configurable: true,
    get: () => options?.scrollHeight ?? 1000,
  });
  Object.defineProperty(el, "clientHeight", {
    configurable: true,
    get: () => options?.clientHeight ?? 400,
  });
  Object.defineProperty(el, "scrollTop", {
    configurable: true,
    get: () => st,
    set: (val: number) => {
      st = val;
    },
  });
  el.scrollTo = vi.fn((opts?: ScrollToOptions | number) => {
    if (typeof opts === "object" && opts?.top !== undefined) {
      st = opts.top;
    }
  }) as any;
  return el;
}

describe("useMessageScroll", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("scrolls to the very bottom when entering a conversation", async () => {
    const listEl = makeMockElement({ scrollHeight: 1200, clientHeight: 400, scrollTop: 0 });
    const listRef = ref<HTMLElement | null>(listEl);
    const stickToBottom = ref(true);
    const sessionId = ref("session-1");
    const isSending = ref(false);
    const messages = ref<ChatMessage[]>([
      { id: "m1", role: "user", content: "hello" } as ChatMessage,
      { id: "m2", role: "assistant", content: "world" } as ChatMessage,
    ]);
    const displayItems = computed(() => messages.value);
    const activeUserMessageId = ref("");
    const railRef = ref<HTMLElement | null>(null);
    const updateMetrics = vi.fn();

    const [scrollHook, app] = withSetup(() =>
      useMessageScroll({
        listRef,
        stickToBottom,
        messages: computed(() => messages.value),
        displayItems,
        activeUserMessageId,
        railRef,
        sessionId,
        isSending,
        updateActiveUserMessage: updateMetrics,
      }),
    );

    await vi.advanceTimersByTimeAsync(100);
    expect(listEl.scrollTop).toBe(800);
    expect(stickToBottom.value).toBe(true);

    app.unmount();
  });

  it("resets stickToBottom and scrolls to bottom when switching sessions", async () => {
    const listEl = makeMockElement({ scrollHeight: 2000, clientHeight: 500, scrollTop: 100 });
    const listRef = ref<HTMLElement | null>(listEl);
    const stickToBottom = ref(false);
    const sessionId = ref("session-1");
    const isSending = ref(false);
    const messages = ref<ChatMessage[]>([
      { id: "m1", role: "user", content: "msg1" } as ChatMessage,
    ]);
    const displayItems = computed(() => messages.value);
    const activeUserMessageId = ref("");
    const railRef = ref<HTMLElement | null>(null);

    const [scrollHook, app] = withSetup(() =>
      useMessageScroll({
        listRef,
        stickToBottom,
        messages: computed(() => messages.value),
        displayItems,
        activeUserMessageId,
        railRef,
        sessionId,
        isSending,
        updateActiveUserMessage: vi.fn(),
      }),
    );

    sessionId.value = "session-2";
    messages.value = [
      { id: "m2", role: "user", content: "new session user" } as ChatMessage,
      { id: "m3", role: "assistant", content: "new session assistant" } as ChatMessage,
    ];

    await vi.advanceTimersByTimeAsync(100);

    expect(stickToBottom.value).toBe(true);
    expect(listEl.scrollTop).toBe(1500);

    app.unmount();
  });

  it("unsticks when user scrolls up with mouse wheel", async () => {
    const listEl = makeMockElement({ scrollHeight: 2000, clientHeight: 500, scrollTop: 1500 });
    const listRef = ref<HTMLElement | null>(listEl);
    const stickToBottom = ref(true);
    const sessionId = ref("session-1");
    const isSending = ref(false);
    const messages = ref<ChatMessage[]>([]);
    const displayItems = computed(() => messages.value);

    const [scrollHook, app] = withSetup(() =>
      useMessageScroll({
        listRef,
        stickToBottom,
        messages: computed(() => messages.value),
        displayItems,
        activeUserMessageId: ref(""),
        railRef: ref<HTMLElement | null>(null),
        sessionId,
        isSending,
        updateActiveUserMessage: vi.fn(),
      }),
    );

    scrollHook.handleWheel(new WheelEvent("wheel", { deltaY: -50 }));
    expect(stickToBottom.value).toBe(false);

    app.unmount();
  });

  it("re-sticks and scrolls to latest when scrollToLatest is called", async () => {
    const listEl = makeMockElement({ scrollHeight: 2000, clientHeight: 500, scrollTop: 500 });
    const listRef = ref<HTMLElement | null>(listEl);
    const stickToBottom = ref(false);
    const sessionId = ref("session-1");
    const isSending = ref(false);
    const messages = ref<ChatMessage[]>([]);
    const displayItems = computed(() => messages.value);

    const [scrollHook, app] = withSetup(() =>
      useMessageScroll({
        listRef,
        stickToBottom,
        messages: computed(() => messages.value),
        displayItems,
        activeUserMessageId: ref(""),
        railRef: ref<HTMLElement | null>(null),
        sessionId,
        isSending,
        updateActiveUserMessage: vi.fn(),
      }),
    );

    scrollHook.scrollToLatest();
    expect(stickToBottom.value).toBe(true);
    expect(listEl.scrollTo).toHaveBeenCalledWith({ top: 2000, behavior: "smooth" });

    app.unmount();
  });
});
