/** @vitest-environment jsdom */
import { afterEach, describe, expect, it, vi } from "vitest";
import { createApp, defineComponent } from "vue";
import { useTurnHeadSticky } from "./useTurnHeadSticky";

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

function stubRect(el: Element, top: number, bottom = top + 40) {
  vi.spyOn(el, "getBoundingClientRect").mockReturnValue({
    top,
    bottom,
    left: 0,
    right: 100,
    width: 100,
    height: bottom - top,
    x: 0,
    y: top,
    toJSON() {
      return {};
    },
  });
}

describe("useTurnHeadSticky", () => {
  const observerInits: IntersectionObserverInit[] = [];

  afterEach(() => {
    document.body.innerHTML = "";
    observerInits.length = 0;
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("constructs IntersectionObserver with a pixel rootMargin and only sticks on the stick line", () => {
    vi.stubGlobal(
      "ResizeObserver",
      class {
        observe() {}
        disconnect() {}
        unobserve() {}
      },
    );
    vi.stubGlobal(
      "IntersectionObserver",
      class {
        constructor(_cb: IntersectionObserverCallback, options?: IntersectionObserverInit) {
          if (options) observerInits.push(options);
        }
        observe() {}
        disconnect() {}
        unobserve() {}
        takeRecords() {
          return [];
        }
      },
    );

    const list = document.createElement("div");
    list.style.overflowY = "auto";
    const turn = document.createElement("div");
    turn.className = "chat-turn";
    turn.innerHTML = `
      <div class="chat-turn-sticky-sentinel"></div>
      <div class="chat-turn-head"></div>
    `;
    list.append(turn);
    document.body.append(list);
    const sentinel = turn.querySelector(".chat-turn-sticky-sentinel")!;
    const head = turn.querySelector(".chat-turn-head") as HTMLElement;
    head.style.top = "0px";
    stubRect(list, 0, 400);
    stubRect(sentinel, 80);
    stubRect(head, 80, 120);

    const [{ bindTurnHead, isTurnHeadStuck }, app] = withSetup(() => useTurnHeadSticky());
    bindTurnHead("turn-1", turn);
    expect(observerInits[0]?.rootMargin).toBe("0px");
    expect(isTurnHeadStuck("turn-1")).toBe(false);

    stubRect(sentinel, -2);
    stubRect(head, 0, 40);
    list.dispatchEvent(new Event("scroll"));
    expect(isTurnHeadStuck("turn-1")).toBe(true);

    stubRect(sentinel, -80);
    stubRect(head, -50, -10);
    list.dispatchEvent(new Event("scroll"));
    expect(isTurnHeadStuck("turn-1")).toBe(false);
    app.unmount();
  });
});
