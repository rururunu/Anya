import { onUnmounted, shallowRef } from "vue";

type Binding = {
  target: HTMLElement;
  resize: ResizeObserver;
  sticky: IntersectionObserver | null;
  root: Element | null;
  onScroll: (() => void) | null;
};

function scrollRootOf(el: HTMLElement): Element | null {
  let current: HTMLElement | null = el.parentElement;
  while (current) {
    const overflowY = getComputedStyle(current).overflowY;
    if (overflowY === "auto" || overflowY === "scroll") return current;
    current = current.parentElement;
  }
  return null;
}

function stickyTopPx(head: HTMLElement): number {
  const style = getComputedStyle(head);
  const fromTop = Number.parseFloat(style.top);
  if (Number.isFinite(fromTop)) return fromTop;
  const fromVar = Number.parseFloat(style.getPropertyValue("--chat-sticky-top"));
  return Number.isFinite(fromVar) ? fromVar : 0;
}

function isHeadStuck(head: HTMLElement, sentinel: HTMLElement, root: Element): boolean {
  const stickLine = root.getBoundingClientRect().top + stickyTopPx(head);
  return (
    sentinel.getBoundingClientRect().top <= stickLine + 1 &&
    head.getBoundingClientRect().bottom > stickLine
  );
}

/** Measure the turn header and only paint its cover while it is actually stuck. */
export function useTurnHeadSticky() {
  const bindings = new Map<string, Binding>();
  const stuckKeys = shallowRef(new Set<string>());

  function setStuck(key: string, value: boolean) {
    if (stuckKeys.value.has(key) === value) return;
    const next = new Set(stuckKeys.value);
    if (value) next.add(key);
    else next.delete(key);
    stuckKeys.value = next;
  }

  function isTurnHeadStuck(key: string) {
    return stuckKeys.value.has(key);
  }

  function disconnect(key: string) {
    const current = bindings.get(key);
    if (current) {
      current.resize.disconnect();
      current.sticky?.disconnect();
      if (current.root && current.onScroll) {
        current.root.removeEventListener("scroll", current.onScroll);
      }
      bindings.delete(key);
    }
    setStuck(key, false);
  }

  function bindTurnHead(key: string, el: unknown) {
    const turn = el instanceof HTMLElement ? el : null;
    const current = bindings.get(key);
    if (current && current.target !== turn) disconnect(key);
    if (!turn) {
      disconnect(key);
      return;
    }
    if (current?.target === turn) return;

    const apply = () => {
      const head = turn.querySelector<HTMLElement>(":scope > .chat-turn-head");
      const card = head?.querySelector<HTMLElement>(".user-composer");
      const height = card?.offsetHeight ?? head?.offsetHeight ?? 0;
      if (height > 0) {
        turn.style.setProperty("--turn-head-height", `${height}px`);
        turn.style.setProperty(
          "--code-block-sticky-top",
          `calc(var(--chat-sticky-top, 0px) + ${height}px + var(--chat-sticky-fade, 12px))`,
        );
      }
      const sentinel = turn.querySelector<HTMLElement>(":scope > .chat-turn-sticky-sentinel");
      const root = bindings.get(key)?.root ?? scrollRootOf(turn);
      if (head && sentinel && root) setStuck(key, isHeadStuck(head, sentinel, root));
    };

    const resize = new ResizeObserver(apply);
    resize.observe(turn);
    const head = turn.querySelector(":scope > .chat-turn-head");
    if (head instanceof HTMLElement) resize.observe(head);
    const card = turn.querySelector(".user-composer");
    if (card instanceof HTMLElement) resize.observe(card);

    const sentinel = turn.querySelector(":scope > .chat-turn-sticky-sentinel");
    const root =
      head instanceof HTMLElement && sentinel instanceof HTMLElement ? scrollRootOf(turn) : null;
    let sticky: IntersectionObserver | null = null;
    let onScroll: (() => void) | null = null;
    if (root && head instanceof HTMLElement && sentinel instanceof HTMLElement) {
      onScroll = apply;
      root.addEventListener("scroll", onScroll, { passive: true });
      if (typeof IntersectionObserver !== "undefined") {
        try {
          sticky = new IntersectionObserver(apply, {
            root,
            threshold: 0,
            rootMargin: "0px",
          });
          sticky.observe(sentinel);
        } catch {
          sticky = null;
        }
      }
    }

    bindings.set(key, { target: turn, resize, sticky, root, onScroll });
    apply();
  }

  onUnmounted(() => {
    for (const key of [...bindings.keys()]) disconnect(key);
  });

  return { bindTurnHead, isTurnHeadStuck };
}
