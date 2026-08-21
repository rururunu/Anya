<template>
  <div
    class="thinking-effort-panel command-list"
    :style="sliderStyle"
    data-tauri-drag-region="false"
    role="slider"
    :aria-label="title"
    :aria-valuemin="0"
    :aria-valuemax="maxIndex"
    :aria-valuenow="index"
    :aria-valuetext="currentLabel"
    @wheel.prevent="onWheel"
  >
    <div class="thinking-slider-value">{{ currentLabel }}</div>
    <div
      class="thinking-slider-hit"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
    >
      <div class="thinking-slider-track">
        <div class="thinking-slider-fill" />
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
</template>

<script setup lang="ts">
import { computed, ref } from "vue";

const props = defineProps<{
  options: Array<{ id: string; label: string }>;
  selectedId: string;
  title: string;
}>();

const emit = defineEmits<{
  select: [id: string];
}>();

const dragging = ref(false);

const index = computed(() => {
  const found = props.options.findIndex((option) => option.id === props.selectedId);
  return found >= 0 ? found : 0;
});

const maxIndex = computed(() => Math.max(props.options.length - 1, 0));

const currentLabel = computed(() => props.options[index.value]?.label ?? "");

const sliderStyle = computed(() => ({
  "--index": String(index.value),
  "--max": String(Math.max(maxIndex.value, 1)),
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
  const rect = hit.getBoundingClientRect();
  const ratio = rect.width <= 0 ? 0 : (event.clientX - rect.left) / rect.width;
  return Math.round(Math.min(1, Math.max(0, ratio)) * maxIndex.value);
}

function onPointerDown(event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }
  dragging.value = true;
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  selectIndex(indexFromClientX(event));
}

function onPointerMove(event: PointerEvent) {
  if (!dragging.value) {
    return;
  }
  selectIndex(indexFromClientX(event));
}

function onPointerUp(event: PointerEvent) {
  dragging.value = false;
  const hit = event.currentTarget as HTMLElement | null;
  if (hit?.hasPointerCapture(event.pointerId)) {
    hit.releasePointerCapture(event.pointerId);
  }
}

function onWheel(event: WheelEvent) {
  if (maxIndex.value <= 0) {
    return;
  }
  const delta = event.deltaY === 0 ? event.deltaX : event.deltaY;
  if (delta === 0) {
    return;
  }
  selectIndex(index.value + (delta < 0 ? 1 : -1));
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
}

.thinking-slider-value {
  margin-bottom: 8px;
  font-size: 13px;
  font-weight: 500;
  line-height: 16px;
  color: var(--peek-text);
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
</style>
