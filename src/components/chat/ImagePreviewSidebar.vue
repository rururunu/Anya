<template>
  <section class="image-preview-sidebar" data-tauri-drag-region="false" :aria-label="previewLabel">
    <nav
      ref="tabsRef"
      class="image-tabs peek-card-tabs"
      role="tablist"
      :aria-label="previewLabel"
      @wheel="scrollTabs"
    >
      <div
        v-for="source in sources"
        :key="source"
        class="image-tab-shell peek-card-tab"
        :class="{ active: source === activeSource }"
      >
        <button
          type="button"
          role="tab"
          class="image-tab"
          :aria-selected="source === activeSource"
          :title="sourceName(source)"
          @click="emit('select', source)"
        >
          <ImageIcon :size="15" />
          <span>{{ sourceName(source) }}</span>
        </button>
        <button
          type="button"
          class="image-tab-close"
          :aria-label="closeTabLabel"
          :title="closeTabLabel"
          @click.stop="emit('close', source)"
        >
          <X :size="11" />
        </button>
      </div>
    </nav>

    <header class="image-preview-toolbar">
      <span class="image-name" :title="displayName">{{ displayName }}</span>
      <div class="image-actions">
        <button type="button" :title="zoomOutLabel" @click="zoomOut">
          <ZoomOut :size="14" />
        </button>
        <span class="zoom-value">{{ Math.round(scale * 100) }}%</span>
        <button type="button" :title="zoomInLabel" @click="zoomIn">
          <ZoomIn :size="14" />
        </button>
        <button type="button" :title="resetLabel" @click="resetView">
          <Scan :size="14" />
        </button>
        <button type="button" :title="saveAsLabel" :disabled="saving" @click="saveAs">
          <Download :size="14" />
        </button>
        <button type="button" :title="copyLabel" @click="copySource">
          <Check v-if="copied" :size="14" />
          <Copy v-else :size="14" />
        </button>
      </div>
    </header>

    <div
      ref="stageRef"
      class="image-stage"
      :class="{ dragging, pannable: scale > 1 }"
      data-tauri-drag-region="false"
      @wheel.prevent="handleWheel"
      @pointerdown="startDrag"
      @pointermove="moveDrag"
      @pointerup="endDrag"
      @pointercancel="endDrag"
      @dblclick="toggleZoom"
    >
      <img
        v-if="resolvedSource"
        ref="imageRef"
        :src="resolvedSource"
        alt=""
        draggable="false"
        :style="{ transform: `translate3d(${offsetX}px, ${offsetY}px, 0) scale(${scale})` }"
        @load="handleImageLoad"
      />
      <p v-else>{{ emptyLabel }}</p>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { Check, Copy, Download, Image as ImageIcon, Scan, X, ZoomIn, ZoomOut } from "@lucide/vue";
import { copyText } from "@/services/clipboard";
import { resolveChatImageSrc, unwrapLocalImagePath } from "@/services/chat/localImageSrc";
import { saveChatImage } from "@/services/chat/saveChatImage";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";

const props = defineProps<{
  sources: string[];
  selectedSource: string;
}>();
const emit = defineEmits<{
  select: [source: string];
  close: [source: string];
}>();

const settingStore = useSettingStore();
const tabsRef = ref<HTMLElement | null>(null);
const stageRef = ref<HTMLElement | null>(null);
const imageRef = ref<HTMLImageElement | null>(null);
const scale = ref(1);
const offsetX = ref(0);
const offsetY = ref(0);
const dragging = ref(false);
const dragStartX = ref(0);
const dragStartY = ref(0);
const copied = ref(false);
const saving = ref(false);
let activePointerId: number | null = null;
let copyTimer: number | undefined;
let stageResizeObserver: ResizeObserver | undefined;

const activeSource = computed(() =>
  props.sources.includes(props.selectedSource) ? props.selectedSource : (props.sources[0] ?? ""),
);
const resolvedSource = computed(() => resolveChatImageSrc(activeSource.value));
const displayName = computed(() => sourceName(activeSource.value));
const previewLabel = computed(() => tr(settingStore.language, "image.preview"));
const closeTabLabel = computed(() => tr(settingStore.language, "image.close"));
const zoomInLabel = computed(() => tr(settingStore.language, "image.zoomIn"));
const zoomOutLabel = computed(() => tr(settingStore.language, "image.zoomOut"));
const resetLabel = computed(() => tr(settingStore.language, "image.fit"));
const copyLabel = computed(() =>
  tr(settingStore.language, copied.value ? "image.copied" : "image.copySource"),
);
const saveAsLabel = computed(() => tr(settingStore.language, "image.saveAs"));
const emptyLabel = computed(() => tr(settingStore.language, "image.empty"));

function sourceName(source: string) {
  const value = source.trim();
  if (!value) return previewLabel.value;
  if (value.startsWith("data:")) return tr(settingStore.language, "image.pasted");
  const clean = unwrapLocalImagePath(value);
  return clean.split(/[\\/]/).filter(Boolean).pop() || clean;
}

function panBounds() {
  const stage = stageRef.value;
  const image = imageRef.value;
  if (!stage || !image || scale.value <= 1) return { x: 0, y: 0 };
  const style = getComputedStyle(stage);
  const availableWidth =
    stage.clientWidth -
    Number.parseFloat(style.paddingLeft) -
    Number.parseFloat(style.paddingRight);
  const availableHeight =
    stage.clientHeight -
    Number.parseFloat(style.paddingTop) -
    Number.parseFloat(style.paddingBottom);
  return {
    x: Math.max(0, (image.offsetWidth * scale.value - availableWidth) / 2),
    y: Math.max(0, (image.offsetHeight * scale.value - availableHeight) / 2),
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
function handleWheel(event: WheelEvent) {
  setScale(scale.value + (event.deltaY < 0 ? 0.12 : -0.12));
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

function handleImageLoad() {
  resetView();
  void nextTick(clampOffsets);
}

function scrollTabs(event: WheelEvent) {
  const tabs = tabsRef.value;
  if (!tabs || tabs.scrollWidth <= tabs.clientWidth) return;
  event.preventDefault();
  tabs.scrollLeft += Math.abs(event.deltaY) >= Math.abs(event.deltaX) ? event.deltaY : event.deltaX;
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

async function copySource() {
  if (!activeSource.value) return;
  await copyText(activeSource.value);
  copied.value = true;
  if (copyTimer) window.clearTimeout(copyTimer);
  copyTimer = window.setTimeout(() => {
    copied.value = false;
  }, 1400);
}

watch(activeSource, () => {
  copied.value = false;
  resetView();
});

onMounted(() => {
  if (stageRef.value) {
    stageResizeObserver = new ResizeObserver(clampOffsets);
    stageResizeObserver.observe(stageRef.value);
  }
});

onUnmounted(() => {
  stageResizeObserver?.disconnect();
  if (copyTimer) window.clearTimeout(copyTimer);
});
</script>

<style scoped>
.image-preview-sidebar {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.image-tabs {
  flex: none;
}
.image-tab-shell {
  flex: 0 0 190px;
  width: 190px;
  min-width: 190px;
  max-width: 240px;
}
.image-tab {
  flex: 1;
  min-width: 0;
  height: 100%;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 0 4px 0 9px;
  border: 0;
  background: transparent;
  color: inherit;
  cursor: pointer;
}
.image-tab > svg {
  flex: none;
  color: color-mix(in srgb, var(--peek-accent) 82%, var(--peek-text));
}
.image-tab span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 10px;
}
.image-tab-close {
  position: relative;
  z-index: 1;
  flex: none;
  width: 24px;
  height: 24px;
  display: inline-grid;
  place-items: center;
  margin-right: 4px;
  padding: 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--peek-faint);
  cursor: pointer;
  opacity: 0;
}
.image-tab-shell:hover .image-tab-close,
.image-tab-shell.active .image-tab-close {
  opacity: 1;
}
.image-tab-close:hover {
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 8%, transparent);
}
.image-preview-toolbar {
  flex: none;
  height: 36px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 8px 0 12px;
  border-bottom: 1px solid var(--peek-border);
}
.image-name {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  color: var(--peek-muted);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.image-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}
.image-actions button {
  width: 26px;
  height: 26px;
  display: grid;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}
.image-actions button:hover {
  background: color-mix(in srgb, var(--peek-text) 7%, transparent);
  color: var(--peek-text);
}
.image-actions button:disabled {
  opacity: 0.5;
  cursor: wait;
}
.zoom-value {
  width: 40px;
  color: var(--peek-faint);
  font-size: 10px;
  text-align: center;
}
.image-stage {
  box-sizing: border-box;
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: grid;
  place-items: center;
  overflow: hidden;
  padding: 18px;
  background: color-mix(in srgb, var(--peek-bg) 72%, transparent);
  cursor: default;
  user-select: none;
  touch-action: none;
}
.image-stage.pannable {
  cursor: grab;
}
.image-stage.dragging {
  cursor: grabbing;
}
.image-stage img {
  display: block;
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  pointer-events: none;
  transform-origin: center;
  transition: transform 100ms ease-out;
  will-change: transform;
}
.image-stage.dragging img {
  transition: none;
}
.image-stage p {
  color: var(--peek-faint);
  font-size: 12px;
}
@container workspace-sidebar (max-width: 560px) {
  .image-tab-shell {
    flex-basis: 164px;
    width: 164px;
    min-width: 164px;
  }
  .image-preview-toolbar {
    padding-left: 8px;
  }
  .image-name {
    display: none;
  }
  .image-actions {
    width: 100%;
    justify-content: flex-end;
  }
}
</style>
