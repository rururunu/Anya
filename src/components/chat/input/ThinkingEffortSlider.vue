<template>
  <div
    class="thinking-effort-panel"
    :class="inline ? 'inline' : 'command-list'"
    :style="sliderStyle"
    data-tauri-drag-region="false"
    role="slider"
    :aria-label="title"
    :aria-valuemin="0"
    :aria-valuemax="maxIndex"
    :aria-valuenow="index"
    :aria-valuetext="currentLabel"
    tabindex="0"
    @keydown="onKeydown"
    @wheel="onWheel"
  >
    <span v-if="inline" class="thinking-slider-title">{{ title }}</span>
    <div v-else class="thinking-slider-heading">{{ title }}</div>
    <div
      class="thinking-slider-hit"
      @pointerdown.stop.prevent="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
    >
      <div class="thinking-slider-track">
        <div class="thinking-slider-fill" />
        <div class="thinking-slider-positions">
          <span
            v-for="(_, tickIndex) in options"
            :key="tickIndex"
            class="thinking-slider-stop"
            :class="{ reached: tickIndex <= index }"
            :style="{ left: tickLeft(tickIndex) }"
          />
          <div class="thinking-slider-thumb" :class="{ dragging }" />
        </div>
      </div>
    </div>
    <div v-if="!inline" class="thinking-slider-labels">
      <span
        v-for="(option, optionIndex) in options"
        :key="option.id"
        :class="{ selected: optionIndex === index }"
        @mousedown.prevent
        @click="selectIndex(optionIndex)"
      >
        {{ option.label }}
      </span>
    </div>
    <span v-if="inline" class="thinking-slider-value">{{ currentLabel }}</span>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";

const props = defineProps<{
  options: Array<{ id: string; label: string }>;
  selectedId: string;
  title: string;
  /**
   * Embedded inside another list (e.g. under the current model in the model picker):
   * no panel chrome, title + value on one line, and the wheel scrolls the list instead.
   */
  inline?: boolean;
}>();

const emit = defineEmits<{
  select: [id: string];
}>();

const dragging = ref(false);
const dragIndex = ref(0);

const index = computed(() => {
  if (dragging.value) return dragIndex.value;
  const found = props.options.findIndex((option) => option.id === props.selectedId);
  return found >= 0 ? found : 0;
});

const maxIndex = computed(() => Math.max(props.options.length - 1, 0));

const currentLabel = computed(() => props.options[index.value]?.label ?? "");

const sliderStyle = computed(() => ({
  "--index": String(index.value),
  "--max": String(Math.max(maxIndex.value, 1)),
  "--count": String(Math.max(props.options.length, 1)),
}));

function tickLeft(tickIndex: number) {
  if (maxIndex.value <= 0) {
    return "0%";
  }
  return `${(tickIndex / maxIndex.value) * 100}%`;
}

function selectIndex(next: number) {
  const clamped = Math.max(0, Math.min(maxIndex.value, next));
  const option = props.options[clamped];
  if (option && option.id !== props.selectedId) {
    emit("select", option.id);
  }
}

function indexFromClientX(event: PointerEvent) {
  const hit = event.currentTarget as HTMLElement | null;
  if (!hit || props.options.length <= 1) {
    return 0;
  }
  const rect = (
    hit.querySelector<HTMLElement>(".thinking-slider-positions") ?? hit
  ).getBoundingClientRect();
  const ratio = rect.width <= 0 ? 0 : (event.clientX - rect.left) / rect.width;
  return Math.round(Math.min(1, Math.max(0, ratio)) * maxIndex.value);
}

function onPointerDown(event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }
  dragIndex.value = indexFromClientX(event);
  dragging.value = true;
  (event.currentTarget as HTMLElement)
    .closest<HTMLElement>(".thinking-effort-panel")
    ?.focus({ preventScroll: true });
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}

function onPointerMove(event: PointerEvent) {
  if (!dragging.value) {
    return;
  }
  dragIndex.value = indexFromClientX(event);
}

function onPointerUp(event: PointerEvent) {
  if (!dragging.value) return;
  const selected = dragIndex.value;
  dragging.value = false;
  if (event.type !== "pointercancel") selectIndex(selected);
  const hit = event.currentTarget as HTMLElement | null;
  if (hit?.hasPointerCapture(event.pointerId)) {
    hit.releasePointerCapture(event.pointerId);
  }
}

function onWheel(event: WheelEvent) {
  if (props.inline || maxIndex.value <= 0) {
    return;
  }
  event.preventDefault();
  const delta = event.deltaY === 0 ? event.deltaX : event.deltaY;
  if (delta === 0) {
    return;
  }
  selectIndex(index.value + (delta < 0 ? 1 : -1));
}

function onKeydown(event: KeyboardEvent) {
  const next =
    event.key === "Home"
      ? 0
      : event.key === "End"
        ? maxIndex.value
        : event.key === "ArrowRight" || event.key === "ArrowUp"
          ? index.value + 1
          : event.key === "ArrowLeft" || event.key === "ArrowDown"
            ? index.value - 1
            : null;
  if (next === null) return;
  event.preventDefault();
  event.stopPropagation();
  selectIndex(next);
}
</script>

<style scoped>
.thinking-effort-panel {
  --index: 0;
  --max: 1;
  --command-row-height: 44px;
  box-sizing: border-box;
  width: min(var(--chip-picker-width, 220px), 100%);
  max-width: 240px;
  padding: 10px 12px 12px;
  border-bottom: 1px solid var(--peek-border);
  background: var(--peek-list-bg);
  outline: none;
}

/* Inline: one compact row  [title] [========track========] [value]  matching list rows. */
.thinking-effort-panel.inline {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  max-width: none;
  height: 30px;
  padding: 0 14px 0 33px;
  border-bottom: 0;
  background: transparent;
}

.thinking-slider-value {
  margin-bottom: 8px;
  font-size: 13px;
  font-weight: 500;
  line-height: 16px;
  color: var(--peek-text);
}

.thinking-slider-title {
  flex: none;
  font-size: 11px;
  line-height: 14px;
  color: var(--peek-muted);
  white-space: nowrap;
}

.inline .thinking-slider-hit {
  flex: 1;
  min-width: 0;
}

.inline .thinking-slider-value {
  flex: none;
  min-width: 2.5em;
  margin: 0;
  font-size: 11px;
  line-height: 14px;
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.thinking-slider-hit {
  height: 18px;
  display: flex;
  align-items: center;
  cursor: pointer;
  touch-action: none;
  user-select: none;
}

.thinking-slider-track {
  position: relative;
  width: 100%;
  height: 3px;
  border-radius: 99px;
  background: color-mix(in srgb, var(--peek-text) 12%, transparent);
}
.thinking-slider-positions {
  position: absolute;
  inset: 0;
}

.thinking-slider-fill {
  position: absolute;
  top: 0;
  bottom: 0;
  left: 0;
  width: 100%;
  border-radius: inherit;
  background: color-mix(in srgb, var(--peek-text) 48%, transparent);
  transform: scaleX(calc(var(--index) / var(--max)));
  transform-origin: left center;
  transition: transform 140ms cubic-bezier(0.2, 0.8, 0.2, 1);
  pointer-events: none;
}

.thinking-slider-stop {
  position: absolute;
  top: 50%;
  width: 3px;
  height: 3px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--peek-text) 28%, transparent);
  transform: translate(-50%, -50%);
  pointer-events: none;
}

.thinking-slider-stop.reached {
  background: color-mix(in srgb, var(--peek-surface) 70%, var(--peek-text));
}

.thinking-slider-thumb {
  position: absolute;
  top: 50%;
  left: calc(var(--index) / var(--max) * 100%);
  z-index: 1;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--peek-text);
  transform: translate(-50%, -50%);
  transition: left 140ms cubic-bezier(0.2, 0.8, 0.2, 1);
  pointer-events: none;
}

.thinking-slider-hit:hover .thinking-slider-thumb,
.thinking-slider-thumb.dragging {
  transform: translate(-50%, -50%) scale(1.08);
}

.thinking-slider-thumb.dragging,
.thinking-slider-hit:has(.dragging) .thinking-slider-fill {
  transition: none;
}

.thinking-effort-panel:not(.inline) {
  max-width: none;
}
.thinking-slider-heading {
  margin-bottom: 18px;
  font-size: 13px;
  font-weight: 600;
  color: var(--peek-text);
}
.thinking-effort-panel:not(.inline) .thinking-slider-hit {
  height: 36px;
  padding: 0 max(0px, calc(100% / var(--count) / 2 - 14px));
}
.thinking-effort-panel:not(.inline) .thinking-slider-track {
  height: 24px;
  background: color-mix(in srgb, var(--peek-accent) 12%, var(--peek-surface));
}
.thinking-effort-panel:not(.inline) .thinking-slider-positions {
  inset: 0 14px;
}
.thinking-effort-panel:not(.inline) .thinking-slider-fill {
  background: linear-gradient(
    to right,
    color-mix(in srgb, var(--peek-accent) 28%, var(--peek-surface)),
    var(--peek-accent)
  );
  transform: none;
  /* Extend the fill beyond the thumb's 9px radius, leaving 4px of color around it. */
  clip-path: inset(
    0 max(0px, calc(1px + (1 - var(--index) / var(--max)) * (100% - 28px))) 0 0 round 99px
  );
  transition: clip-path 140ms cubic-bezier(0.2, 0.8, 0.2, 1);
}
.thinking-effort-panel:not(.inline) .thinking-slider-stop {
  width: 6px;
  height: 6px;
  background: color-mix(in srgb, var(--peek-accent) 36%, var(--peek-surface));
}
.thinking-effort-panel:not(.inline) .thinking-slider-stop.reached {
  background: var(--peek-surface);
}
.thinking-effort-panel:not(.inline) .thinking-slider-thumb {
  box-sizing: border-box;
  width: 18px;
  height: 18px;
  border: 2px solid color-mix(in srgb, var(--peek-accent) 40%, var(--peek-surface));
  background: #fff;
  box-shadow: 0 2px 6px color-mix(in srgb, var(--peek-accent) 24%, transparent);
}
.thinking-effort-panel:not(.inline) .thinking-slider-hit:hover .thinking-slider-thumb,
.thinking-effort-panel:not(.inline) .thinking-slider-thumb.dragging {
  transform: translate(-50%, -50%);
}
.thinking-effort-panel:focus-visible .thinking-slider-thumb {
  outline: 2px solid var(--peek-accent);
  outline-offset: 2px;
}
.thinking-slider-labels {
  display: grid;
  grid-template-columns: repeat(var(--count), minmax(0, 1fr));
  width: 100%;
  min-height: 22px;
  margin: 6px 0 0;
  font-size: 11px;
  line-height: 16px;
  color: var(--peek-muted);
}
.thinking-slider-labels span {
  min-width: 0;
  padding-block: 3px;
  border-radius: 6px;
  white-space: nowrap;
  cursor: pointer;
  text-align: center;
}
.thinking-slider-labels .selected {
  background: color-mix(in srgb, var(--peek-accent) 10%, transparent);
  color: var(--peek-accent);
  font-weight: 600;
}
</style>
