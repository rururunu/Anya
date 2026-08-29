<template>
  <Teleport to="body">
    <Transition name="app-tooltip">
      <div
        v-if="visible && text"
        ref="bubbleRef"
        class="app-tooltip-bubble"
        :class="side"
        :style="bubbleStyle"
        role="tooltip"
      >
        {{ text }}
        <span class="app-tooltip-arrow" />
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from "vue";

const bubbleRef = ref<HTMLElement | null>(null);
const visible = ref(false);
const text = ref("");
const side = ref<"top" | "bottom">("top");
const bubbleStyle = ref<Record<string, string>>({});
let activeTarget: HTMLElement | null = null;
let showTimer: number | null = null;
let observer: MutationObserver | null = null;

function adoptTitle(element: HTMLElement) {
  const title = element.getAttribute("title")?.trim();
  if (!title) return;
  element.dataset.uiTooltip = title;
  element.removeAttribute("title");
  if (!element.getAttribute("aria-label") && element.matches("button, [role='button']")) {
    element.setAttribute("aria-label", title);
  }
}

function adoptTitles(root: ParentNode) {
  if (root instanceof HTMLElement && root.hasAttribute("title")) adoptTitle(root);
  root.querySelectorAll<HTMLElement>("[title]").forEach(adoptTitle);
}

function resolveTarget(target: EventTarget | null) {
  if (!(target instanceof Element)) return null;
  const element = target.closest<HTMLElement>("[data-ui-tooltip], [title]");
  if (element?.hasAttribute("title")) adoptTitle(element);
  return element?.dataset.uiTooltip ? element : null;
}

async function positionBubble(target: HTMLElement) {
  await nextTick();
  if (!bubbleRef.value || target !== activeTarget) return;
  const targetRect = target.getBoundingClientRect();
  const bubbleRect = bubbleRef.value.getBoundingClientRect();
  const gap = 8;
  const edge = 8;
  const placeBelow = targetRect.top < bubbleRect.height + gap + edge;
  side.value = placeBelow ? "bottom" : "top";
  const left = Math.min(
    window.innerWidth - bubbleRect.width / 2 - edge,
    Math.max(bubbleRect.width / 2 + edge, targetRect.left + targetRect.width / 2),
  );
  const top = placeBelow ? targetRect.bottom + gap : targetRect.top - gap;
  bubbleStyle.value = {
    left: `${left}px`,
    top: `${top}px`,
    transform: placeBelow ? "translate(-50%, 0)" : "translate(-50%, -100%)",
  };
}

function show(target: HTMLElement, immediate = false) {
  if (showTimer !== null) window.clearTimeout(showTimer);
  activeTarget = target;
  const reveal = () => {
    if (activeTarget !== target) return;
    text.value = target.dataset.uiTooltip ?? "";
    visible.value = Boolean(text.value);
    void positionBubble(target);
  };
  if (immediate) reveal();
  else showTimer = window.setTimeout(reveal, 220);
}

function hide(target?: HTMLElement | null) {
  if (target && target !== activeTarget) return;
  if (showTimer !== null) window.clearTimeout(showTimer);
  showTimer = null;
  activeTarget = null;
  visible.value = false;
}

function onPointerOver(event: PointerEvent) {
  const target = resolveTarget(event.target);
  if (target && target !== activeTarget) show(target);
}

function onPointerOut(event: PointerEvent) {
  const target = resolveTarget(event.target);
  if (target && !target.contains(event.relatedTarget as Node | null)) hide(target);
}

function onFocusIn(event: FocusEvent) {
  const target = resolveTarget(event.target);
  if (target) show(target, true);
}

function onFocusOut(event: FocusEvent) {
  hide(resolveTarget(event.target));
}

function onViewportChange() {
  hide();
}

onMounted(() => {
  adoptTitles(document.body);
  observer = new MutationObserver((records) => {
    for (const record of records) {
      if (record.type === "attributes" && record.target instanceof HTMLElement) {
        adoptTitle(record.target);
      }
      record.addedNodes.forEach((node) => {
        if (node instanceof HTMLElement) adoptTitles(node);
      });
      record.removedNodes.forEach((node) => {
        if (
          activeTarget &&
          node instanceof Node &&
          (node === activeTarget || node.contains(activeTarget))
        ) {
          hide();
        }
      });
    }
  });
  observer.observe(document.body, {
    subtree: true,
    childList: true,
    attributes: true,
    attributeFilter: ["title"],
  });
  document.addEventListener("pointerover", onPointerOver, true);
  document.addEventListener("pointerout", onPointerOut, true);
  document.addEventListener("focusin", onFocusIn, true);
  document.addEventListener("focusout", onFocusOut, true);
  window.addEventListener("resize", onViewportChange);
});

onUnmounted(() => {
  observer?.disconnect();
  document.removeEventListener("pointerover", onPointerOver, true);
  document.removeEventListener("pointerout", onPointerOut, true);
  document.removeEventListener("focusin", onFocusIn, true);
  document.removeEventListener("focusout", onFocusOut, true);
  window.removeEventListener("resize", onViewportChange);
  hide();
});
</script>

<style scoped>
.app-tooltip-bubble {
  position: fixed;
  z-index: 10000;
  width: max-content;
  max-width: min(360px, calc(100vw - 16px));
  padding: 5px 8px;
  border: 1px solid var(--peek-border);
  border-radius: 6px;
  background: var(--peek-surface);
  color: var(--peek-text);
  box-shadow: 0 6px 18px color-mix(in srgb, #000 18%, transparent);
  font-size: 11px;
  line-height: 16px;
  pointer-events: none;
}

.app-tooltip-arrow {
  position: absolute;
  left: 50%;
  width: 7px;
  height: 7px;
  border: inherit;
  background: inherit;
  transform: translateX(-50%) rotate(45deg);
}

.app-tooltip-bubble.top .app-tooltip-arrow {
  bottom: -4px;
  border-top: 0;
  border-left: 0;
}

.app-tooltip-bubble.bottom .app-tooltip-arrow {
  top: -4px;
  border-right: 0;
  border-bottom: 0;
}

.app-tooltip-enter-active,
.app-tooltip-leave-active {
  transition:
    opacity 100ms ease,
    transform 100ms ease;
}
.app-tooltip-enter-from,
.app-tooltip-leave-to {
  opacity: 0;
}
</style>
