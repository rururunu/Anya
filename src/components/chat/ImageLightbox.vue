<template>
  <Teleport to="body">
    <Transition name="image-lightbox">
      <div
        v-if="open"
        class="image-lightbox lightbox"
        data-tauri-drag-region="false"
        role="dialog"
        aria-modal="true"
        :aria-label="previewLabel"
        @mousedown.self="onBackdropClick"
      >
        <div class="image-lightbox-actions">
          <button
            type="button"
            class="image-lightbox-action"
            :class="{ active: paintMode }"
            :title="selectRegionTitle"
            :disabled="editBusy || !resolvedSource"
            @click="togglePaintMode"
          >
            <Brush :size="18" />
          </button>
          <button
            type="button"
            class="image-lightbox-action"
            :title="editFromTitle"
            :disabled="editBusy || paintMode"
            @click="editFromActive"
          >
            <Pencil :size="18" />
          </button>
          <button
            type="button"
            class="image-lightbox-action"
            :title="saveAsLabel"
            :disabled="saving || paintMode"
            @click="saveAs"
          >
            <Download :size="18" />
          </button>
          <button
            type="button"
            class="image-lightbox-action image-lightbox-action-close"
            :title="closeLabel"
            @click="emit('close')"
          >
            <X :size="18" />
          </button>
        </div>

        <div
          ref="stageRef"
          class="image-lightbox-stage"
          :class="{
            dragging: dragging && !paintMode,
            pannable: !paintMode && scale > 1,
            painting: paintMode,
          }"
          @wheel.prevent="onStageWheel"
          @pointerdown="onStagePointerDown"
          @pointermove="onStagePointerMove"
          @pointerup="onStagePointerUp"
          @pointercancel="onStagePointerUp"
          @dblclick="onStageDoubleClick"
        >
          <button
            v-if="canGoPrev && !paintMode"
            type="button"
            class="image-lightbox-nav image-lightbox-nav-prev"
            :title="prevLabel"
            @click.stop="goPrev"
          >
            <ChevronLeft :size="20" />
          </button>

          <div
            v-if="resolvedSource"
            ref="paintStageRef"
            class="image-lightbox-frame"
            :class="{ 'is-painting': paintMode }"
            :style="frameStyle"
          >
            <img
              ref="imageRef"
              :src="resolvedSource"
              alt=""
              draggable="false"
              @load="handleImageLoad"
            />
            <canvas v-if="paintMode" class="paint-canvas" aria-hidden="true" />
            <p v-if="paintMode" class="select-hint">{{ selectHintLabel }}</p>
          </div>
          <p v-else class="image-lightbox-empty">{{ emptyLabel }}</p>

          <button
            v-if="canGoNext && !paintMode"
            type="button"
            class="image-lightbox-nav image-lightbox-nav-next"
            :title="nextLabel"
            @click.stop="goNext"
          >
            <ChevronRight :size="20" />
          </button>
        </div>

        <div v-if="paintMode" class="image-lightbox-paint-bar">
          <label class="brush-slider" :aria-label="brushSizeLabel">
            <span class="brush-size-label">{{ brushSizeLabel }}</span>
            <input
              v-model.number="brushSlider"
              class="brush-size-slider"
              type="range"
              min="0"
              max="100"
              step="1"
            />
            <span
              class="brush-preview-dot"
              aria-hidden="true"
              :style="{ width: `${brushPreviewPx}px`, height: `${brushPreviewPx}px` }"
            />
          </label>
          <button type="button" class="paint-bar-btn" :disabled="!hasPaint" @click="clearStrokes">
            {{ clearLabel }}
          </button>
          <button type="button" class="paint-bar-btn" @click="cancelPaintMode">
            {{ cancelLabel }}
          </button>
          <button
            type="button"
            class="paint-bar-btn primary"
            :disabled="!hasPaint || editBusy"
            @click="confirmRegion"
          >
            {{ useRegionLabel }}
          </button>
        </div>

        <div v-else class="image-lightbox-zoom">
          <button type="button" :title="zoomOutLabel" @click="zoomOut">
            <Minus :size="16" />
          </button>
          <span>{{ Math.round(scale * 100) }}%</span>
          <button type="button" :title="zoomInLabel" @click="zoomIn">
            <Plus :size="16" />
          </button>
          <button type="button" :title="fitLabel" @click="resetView">
            <Scan :size="15" />
          </button>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch, type CSSProperties } from "vue";
import {
  Brush,
  ChevronLeft,
  ChevronRight,
  Download,
  Minus,
  Pencil,
  Plus,
  Scan,
  X,
} from "@lucide/vue";
import {
  useGeneratedImagePaint,
  BRUSH_RADIUS_MIN,
  BRUSH_RADIUS_MAX,
} from "@/composables/chat/useGeneratedImagePaint";
import { resolveChatImageSrc } from "@/services/chat/localImageSrc";
import {
  prepareFullImageEdit,
  prepareStrokeImageEdit,
  type ImageEditReferencePayload,
} from "@/services/chat/imageEditReference";
import { saveChatImage } from "@/services/chat/saveChatImage";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";

const props = defineProps<{
  open: boolean;
  sources: string[];
  selectedSource: string;
}>();

const emit = defineEmits<{
  close: [];
  select: [source: string];
  editFromImage: [payload: ImageEditReferencePayload];
}>();

const settingStore = useSettingStore();
const stageRef = ref<HTMLElement | null>(null);
const paintStageRef = ref<HTMLElement | null>(null);
const imageRef = ref<HTMLImageElement | null>(null);
const scale = ref(1);
const offsetX = ref(0);
const offsetY = ref(0);
const dragging = ref(false);
const dragStartX = ref(0);
const dragStartY = ref(0);
const saving = ref(false);
const editBusy = ref(false);
let activePointerId: number | null = null;
let stageResizeObserver: ResizeObserver | undefined;

const {
  selectingSource,
  strokes,
  brushSlider,
  brushRadius,
  hasPaint,
  startSelect,
  cancelSelect,
  clearStrokes,
  refreshPaintCanvas,
  syncPaintCanvasFromStage,
  onPaintPointerDown,
  onPaintPointerMove,
  onPaintPointerUp,
  onBrushWheel,
} = useGeneratedImagePaint({ stageRef: paintStageRef });

const activeSource = computed(() =>
  props.sources.includes(props.selectedSource) ? props.selectedSource : (props.sources[0] ?? ""),
);
const activeIndex = computed(() => props.sources.indexOf(activeSource.value));
const canGoPrev = computed(() => activeIndex.value > 0);
const canGoNext = computed(
  () => activeIndex.value >= 0 && activeIndex.value < props.sources.length - 1,
);
const resolvedSource = computed(() => resolveChatImageSrc(activeSource.value));
const paintMode = computed(
  () => Boolean(selectingSource.value) && selectingSource.value === activeSource.value,
);

const previewLabel = computed(() => tr(settingStore.language, "image.preview"));
const closeLabel = computed(() => tr(settingStore.language, "image.close"));
const zoomInLabel = computed(() => tr(settingStore.language, "image.zoomIn"));
const zoomOutLabel = computed(() => tr(settingStore.language, "image.zoomOut"));
const fitLabel = computed(() => tr(settingStore.language, "image.fit"));
const saveAsLabel = computed(() => tr(settingStore.language, "image.saveAs"));
const emptyLabel = computed(() => tr(settingStore.language, "image.empty"));
const editFromTitle = computed(() => tr(settingStore.language, "image.editFromTitle"));
const selectRegionTitle = computed(() => tr(settingStore.language, "image.selectRegionTitle"));
const selectHintLabel = computed(() => tr(settingStore.language, "image.selectHint"));
const useRegionLabel = computed(() => tr(settingStore.language, "image.useRegion"));
const cancelLabel = computed(() => tr(settingStore.language, "image.cancelSelect"));
const clearLabel = computed(() => tr(settingStore.language, "image.clearPaint"));
const brushSizeLabel = computed(() => tr(settingStore.language, "image.brushSize"));
const brushPreviewPx = computed(() => {
  const range = BRUSH_RADIUS_MAX - BRUSH_RADIUS_MIN;
  const t = range <= 0 ? 0.5 : (brushRadius.value - BRUSH_RADIUS_MIN) / range;
  return Math.round(6 + t * 12);
});
const prevLabel = computed(() => tr(settingStore.language, "findPrevious"));
const nextLabel = computed(() => tr(settingStore.language, "findNext"));

const frameStyle = computed<CSSProperties>(() => ({
  transform: `translate3d(${offsetX.value}px, ${offsetY.value}px, 0) scale(${scale.value})`,
}));

function stageInsets() {
  const stage = stageRef.value;
  if (!stage) return { width: 0, height: 0 };
  const style = getComputedStyle(stage);
  return {
    width:
      stage.clientWidth -
      Number.parseFloat(style.paddingLeft) -
      Number.parseFloat(style.paddingRight),
    height:
      stage.clientHeight -
      Number.parseFloat(style.paddingTop) -
      Number.parseFloat(style.paddingBottom),
  };
}

function panBounds() {
  const image = imageRef.value;
  const { width, height } = stageInsets();
  if (!image || scale.value <= 1 || width <= 0 || height <= 0) return { x: 0, y: 0 };
  return {
    x: Math.max(0, (image.offsetWidth * scale.value - width) / 2),
    y: Math.max(0, (image.offsetHeight * scale.value - height) / 2),
  };
}

function clampOffsets() {
  const bounds = panBounds();
  offsetX.value = Math.min(bounds.x, Math.max(-bounds.x, offsetX.value));
  offsetY.value = Math.min(bounds.y, Math.max(-bounds.y, offsetY.value));
}

function setScale(next: number) {
  scale.value = Math.min(8, Math.max(0.1, next));
  if (scale.value <= 1) {
    offsetX.value = 0;
    offsetY.value = 0;
  } else {
    void nextTick(clampOffsets);
  }
}

function zoomIn() {
  setScale(scale.value + 0.25);
}
function zoomOut() {
  setScale(scale.value - 0.25);
}
function resetView() {
  scale.value = 1;
  offsetX.value = 0;
  offsetY.value = 0;
  dragging.value = false;
  activePointerId = null;
}
function toggleZoom() {
  if (scale.value === 1) setScale(2);
  else resetView();
}

function onStageWheel(event: WheelEvent) {
  if (paintMode.value) {
    onBrushWheel(event);
    return;
  }
  setScale(scale.value + (event.deltaY < 0 ? 0.12 : -0.12));
}

function onStagePointerDown(event: PointerEvent) {
  if (paintMode.value) {
    onPaintPointerDown(event, activeSource.value);
    return;
  }
  startDrag(event);
}

function onStagePointerMove(event: PointerEvent) {
  if (paintMode.value) {
    onPaintPointerMove(event);
    return;
  }
  moveDrag(event);
}

function onStagePointerUp(event: PointerEvent) {
  if (paintMode.value) {
    onPaintPointerUp(event);
    return;
  }
  endDrag(event);
}

function onStageDoubleClick() {
  if (paintMode.value) return;
  toggleZoom();
}

function startDrag(event: PointerEvent) {
  if (!resolvedSource.value || event.button !== 0 || scale.value <= 1) return;
  event.preventDefault();
  dragging.value = true;
  activePointerId = event.pointerId;
  dragStartX.value = event.clientX - offsetX.value;
  dragStartY.value = event.clientY - offsetY.value;
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}

function moveDrag(event: PointerEvent) {
  if (!dragging.value || event.pointerId !== activePointerId) return;
  event.preventDefault();
  offsetX.value = event.clientX - dragStartX.value;
  offsetY.value = event.clientY - dragStartY.value;
  clampOffsets();
}

function endDrag(event: PointerEvent) {
  if (event.pointerId !== activePointerId) return;
  const target = event.currentTarget as HTMLElement;
  if (target.hasPointerCapture(event.pointerId)) target.releasePointerCapture(event.pointerId);
  dragging.value = false;
  activePointerId = null;
  clampOffsets();
}

function handleImageLoad(event: Event) {
  resetView();
  syncPaintCanvasFromStage(event);
  void nextTick(clampOffsets);
}

function goPrev() {
  const index = activeIndex.value;
  if (index <= 0) return;
  emit("select", props.sources[index - 1]!);
}

function goNext() {
  const index = activeIndex.value;
  if (index < 0 || index >= props.sources.length - 1) return;
  emit("select", props.sources[index + 1]!);
}

async function saveAs() {
  if (!activeSource.value || saving.value) return;
  saving.value = true;
  try {
    await saveChatImage(activeSource.value);
  } finally {
    saving.value = false;
  }
}

async function editFromActive() {
  if (!activeSource.value || editBusy.value) return;
  editBusy.value = true;
  try {
    const payload = await prepareFullImageEdit(activeSource.value);
    emit("editFromImage", payload);
    emit("close");
  } catch (error) {
    console.error("editFromImage failed:", error);
  } finally {
    editBusy.value = false;
  }
}

async function togglePaintMode() {
  if (paintMode.value) {
    cancelPaintMode();
    return;
  }
  if (!activeSource.value) return;
  resetView();
  await startSelect(activeSource.value);
}

function cancelPaintMode() {
  cancelSelect();
}

async function confirmRegion() {
  if (!activeSource.value || !hasPaint.value || editBusy.value) return;
  editBusy.value = true;
  try {
    const payload = await prepareStrokeImageEdit(activeSource.value, strokes.value);
    emit("editFromImage", payload);
    cancelSelect();
    emit("close");
  } catch (error) {
    console.error("region edit failed:", error);
  } finally {
    editBusy.value = false;
  }
}

function onBackdropClick() {
  if (paintMode.value) {
    cancelPaintMode();
    return;
  }
  emit("close");
}

function onKeydown(event: KeyboardEvent) {
  if (!props.open) return;
  if (event.key === "Escape") {
    event.preventDefault();
    if (paintMode.value) {
      cancelPaintMode();
      return;
    }
    emit("close");
    return;
  }
  if (paintMode.value) return;
  if (event.key === "ArrowLeft") {
    event.preventDefault();
    goPrev();
    return;
  }
  if (event.key === "ArrowRight") {
    event.preventDefault();
    goNext();
  }
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      resetView();
      window.addEventListener("keydown", onKeydown);
      if (stageRef.value) {
        stageResizeObserver?.disconnect();
        stageResizeObserver = new ResizeObserver(() => {
          clampOffsets();
          if (paintMode.value) refreshPaintCanvas();
        });
        stageResizeObserver.observe(stageRef.value);
      }
      return;
    }
    cancelSelect();
    window.removeEventListener("keydown", onKeydown);
    stageResizeObserver?.disconnect();
    stageResizeObserver = undefined;
  },
  { immediate: true },
);

watch(activeSource, () => {
  cancelSelect();
  resetView();
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
  stageResizeObserver?.disconnect();
});
</script>

<style scoped>
.image-lightbox {
  position: fixed;
  inset: 0;
  z-index: 80;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  /* Opt out of Chromium forced-dark so zoomed photos keep true colors. */
  color-scheme: only light;
  background: color-mix(in srgb, var(--peek-bg) 6%, rgb(0 0 0 / 78%));
  backdrop-filter: blur(2px);
  user-select: none;
  touch-action: none;
}

.image-lightbox-actions {
  position: absolute;
  top: 18px;
  right: 18px;
  z-index: 2;
  display: flex;
  align-items: center;
  gap: 10px;
}

.image-lightbox-action {
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  padding: 0;
  border: 1px solid var(--peek-border);
  border-radius: 50%;
  background: var(--peek-surface);
  color: var(--peek-text);
  cursor: pointer;
  box-shadow: 0 8px 28px color-mix(in srgb, #000 24%, transparent);
  transition:
    background-color var(--motion-fast, 110ms) var(--motion-ease-out, ease),
    color var(--motion-fast, 110ms) var(--motion-ease-out, ease),
    transform var(--motion-instant, 80ms) var(--motion-ease-out, ease),
    border-color var(--motion-fast, 110ms) var(--motion-ease-out, ease);
}

.image-lightbox-action:hover {
  background: var(--peek-hover-bg);
}

.image-lightbox-action.active {
  color: var(--peek-accent);
  border-color: color-mix(in srgb, var(--peek-accent) 45%, var(--peek-border));
  background: color-mix(in srgb, var(--peek-accent) 12%, var(--peek-surface));
}

.image-lightbox-action:active:not(:disabled) {
  transform: scale(0.96);
}

.image-lightbox-action:disabled {
  opacity: 0.55;
  cursor: wait;
}

.image-lightbox-action-close:hover {
  color: var(--peek-primary-foreground, #fff);
  background: var(--peek-danger);
  border-color: color-mix(in srgb, var(--peek-danger) 72%, var(--peek-border));
}

.image-lightbox-stage {
  box-sizing: border-box;
  flex: 1;
  min-height: 0;
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 56px 72px 72px;
  overflow: hidden;
  cursor: default;
}

.image-lightbox-stage.pannable {
  cursor: grab;
}

.image-lightbox-stage.dragging {
  cursor: grabbing;
}

.image-lightbox-stage.painting {
  cursor: crosshair;
}

.image-lightbox-frame {
  position: relative;
  display: inline-flex;
  max-width: min(calc(100vw - 144px), 960px);
  max-height: calc(100vh - 132px);
  transform-origin: center center;
  transition: transform 100ms ease-out;
  will-change: transform;
}

.image-lightbox-stage.dragging .image-lightbox-frame {
  transition: none;
}

.image-lightbox-frame img {
  display: block;
  width: auto;
  height: auto;
  max-width: min(calc(100vw - 144px), 960px);
  max-height: calc(100vh - 132px);
  object-fit: contain;
  pointer-events: none;
  color-scheme: only light;
}

.image-lightbox-frame.is-painting {
  cursor: crosshair;
  touch-action: none;
}

.image-lightbox-frame.is-painting img {
  outline: 2px solid color-mix(in srgb, var(--peek-accent) 55%, transparent);
  outline-offset: 1px;
  border-radius: 8px;
}

.paint-canvas {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  border-radius: 8px;
  pointer-events: none;
}

.select-hint {
  position: absolute;
  left: 12px;
  bottom: 12px;
  margin: 0;
  padding: 5px 11px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--peek-bg) 18%, rgb(0 0 0 / 72%));
  color: var(--peek-text);
  font-size: 11px;
  line-height: 1.3;
  pointer-events: none;
}

.image-lightbox-empty {
  margin: 0;
  color: var(--peek-faint);
  font-size: 13px;
}

.image-lightbox-nav {
  position: absolute;
  top: 50%;
  z-index: 1;
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  padding: 0;
  border: 1px solid var(--peek-border);
  border-radius: 50%;
  background: color-mix(in srgb, var(--peek-surface) 92%, transparent);
  color: var(--peek-text);
  cursor: pointer;
  transform: translateY(-50%);
  box-shadow: 0 8px 28px color-mix(in srgb, #000 20%, transparent);
}

.image-lightbox-nav:hover {
  background: var(--peek-hover-bg);
}

.image-lightbox-nav-prev {
  left: 20px;
}

.image-lightbox-nav-next {
  right: 20px;
}

.image-lightbox-zoom,
.image-lightbox-paint-bar {
  position: absolute;
  left: 50%;
  bottom: 24px;
  z-index: 2;
  transform: translateX(-50%);
  border: 1px solid var(--peek-border);
  border-radius: 999px;
  background: var(--peek-surface);
  color: var(--peek-text);
  box-shadow: 0 8px 28px color-mix(in srgb, #000 24%, transparent);
}

.image-lightbox-zoom {
  display: flex;
  align-items: center;
  gap: 2px;
  min-width: 156px;
  padding: 6px 10px;
}

.image-lightbox-zoom button {
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: 50%;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}

.image-lightbox-zoom button:hover {
  color: var(--peek-text);
  background: var(--peek-hover-bg);
}

.image-lightbox-zoom span {
  flex: 1;
  min-width: 52px;
  text-align: center;
  font-size: 12px;
  font-weight: 600;
  color: var(--peek-text);
}

.image-lightbox-paint-bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: center;
  gap: 8px;
  max-width: min(calc(100vw - 32px), 720px);
  padding: 8px 12px;
}

.brush-slider {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  min-width: min(240px, 42vw);
}

.brush-size-label {
  flex: none;
  color: var(--peek-muted);
  font-size: 11px;
  font-weight: 550;
  white-space: nowrap;
}

.brush-size-slider {
  flex: 1;
  min-width: 120px;
  height: 4px;
  margin: 0;
  appearance: none;
  border-radius: 999px;
  background: color-mix(in srgb, var(--peek-text) 14%, transparent);
  cursor: pointer;
}

.brush-size-slider:focus {
  outline: none;
}

.brush-size-slider:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--peek-accent) 45%, transparent);
  outline-offset: 2px;
}

.brush-size-slider::-webkit-slider-thumb {
  appearance: none;
  width: 14px;
  height: 14px;
  border: 2px solid var(--peek-surface);
  border-radius: 50%;
  background: var(--peek-accent);
  box-shadow: 0 1px 4px color-mix(in srgb, #000 18%, transparent);
  cursor: pointer;
}

.brush-size-slider::-moz-range-thumb {
  width: 14px;
  height: 14px;
  border: 2px solid var(--peek-surface);
  border-radius: 50%;
  background: var(--peek-accent);
  box-shadow: 0 1px 4px color-mix(in srgb, #000 18%, transparent);
  cursor: pointer;
}

.brush-preview-dot {
  flex: none;
  border-radius: 50%;
  background: var(--peek-text);
  opacity: 0.82;
}

.paint-bar-btn {
  height: 28px;
  margin: 0;
  padding: 0 11px;
  border: 1px solid color-mix(in srgb, var(--peek-text) 12%, transparent);
  border-radius: 999px;
  background: transparent;
  color: var(--peek-text);
  font-size: 11px;
  font-weight: 550;
  cursor: pointer;
}

.paint-bar-btn:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--peek-text) 22%, transparent);
  background: var(--peek-hover-bg);
}

.paint-bar-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.paint-bar-btn.primary {
  border-color: color-mix(in srgb, var(--peek-accent) 40%, transparent);
  background: color-mix(in srgb, var(--peek-accent) 14%, transparent);
  color: var(--peek-accent);
}

.paint-bar-btn.primary:hover:not(:disabled) {
  background: color-mix(in srgb, var(--peek-accent) 22%, transparent);
}

.image-lightbox-enter-active,
.image-lightbox-leave-active {
  transition: opacity 180ms ease;
}

.image-lightbox-enter-from,
.image-lightbox-leave-to {
  opacity: 0;
}

@media (max-width: 720px) {
  .image-lightbox-stage {
    padding: 52px 16px 68px;
  }

  .image-lightbox-frame,
  .image-lightbox-frame img {
    max-width: calc(100vw - 32px);
    max-height: calc(100vh - 120px);
  }

  .image-lightbox-nav-prev {
    left: 10px;
  }

  .image-lightbox-nav-next {
    right: 10px;
  }
}
</style>
