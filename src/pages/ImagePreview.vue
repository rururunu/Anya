<template>
  <div class="preview-shell" @wheel.prevent="handleWheel">
    <header class="preview-bar" data-tauri-drag-region>
      <span class="preview-title" data-tauri-drag-region>{{ title }}</span>
      <div class="preview-actions" data-tauri-drag-region="false">
        <button type="button" class="bar-btn" :title="zoomOutLabel" @click="zoomOut">
          <ZoomOut :size="15" />
        </button>
        <span class="zoom-label">{{ Math.round(scale * 100) }}%</span>
        <button type="button" class="bar-btn" :title="zoomInLabel" @click="zoomIn">
          <ZoomIn :size="15" />
        </button>
        <button type="button" class="bar-btn" :title="resetLabel" @click="resetZoom">
          <RotateCcw :size="15" />
        </button>
        <button
          type="button"
          class="bar-btn"
          :title="saveAsLabel"
          :disabled="saving"
          @click="saveAs"
        >
          <Download :size="15" />
        </button>
        <button type="button" class="bar-btn close" :title="closeLabel" @click="closeWindow">
          <X :size="16" />
        </button>
      </div>
    </header>

    <div
      class="preview-stage"
      data-tauri-drag-region="false"
      @mousedown="handleDragStart"
      @mousemove="handleDragMove"
      @mouseup="handleDragEnd"
      @mouseleave="handleDragEnd"
      @dblclick="handleDoubleClick"
    >
      <img
        v-if="imageSrc"
        :src="imageSrc"
        alt=""
        class="preview-img"
        draggable="false"
        :style="{
          transform: `translate(${offsetX}px, ${offsetY}px) scale(${scale})`,
        }"
      />
      <p v-else class="preview-empty">{{ emptyLabel }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Download, ZoomIn, ZoomOut, RotateCcw, X } from "@lucide/vue";
import { resolveChatImageSrc } from "@/services/chat/localImageSrc";
import { saveChatImage } from "@/services/chat/saveChatImage";
import { getPreviewImage, setOverlayPopupOpen } from "@/services/ipc";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";

const settingStore = useSettingStore();
const rawSource = ref("");
const imageSrc = computed(() => resolveChatImageSrc(rawSource.value));
const saving = ref(false);
const scale = ref(1);
const offsetX = ref(0);
const offsetY = ref(0);
const isDragging = ref(false);
const startX = ref(0);
const startY = ref(0);
let unlistenUpdated: UnlistenFn | null = null;

const title = computed(() => tr(settingStore.language, "image.preview"));
const zoomInLabel = computed(() => tr(settingStore.language, "image.zoomIn"));
const zoomOutLabel = computed(() => tr(settingStore.language, "image.zoomOut"));
const resetLabel = computed(() => tr(settingStore.language, "image.reset"));
const closeLabel = computed(() => tr(settingStore.language, "image.close"));
const saveAsLabel = computed(() => tr(settingStore.language, "image.saveAs"));
const emptyLabel = computed(() => tr(settingStore.language, "image.empty"));

async function loadPreviewImage() {
  try {
    rawSource.value = await getPreviewImage();
    resetZoom();
  } catch (err) {
    console.error("Failed to load preview image:", err);
  }
}

async function saveAs() {
  if (!rawSource.value || saving.value) return;
  saving.value = true;
  try {
    await saveChatImage(rawSource.value);
  } finally {
    saving.value = false;
  }
}

function handleWheel(e: WheelEvent) {
  const next = scale.value + (e.deltaY < 0 ? 0.12 : -0.12);
  scale.value = Math.min(Math.max(0.1, next), 8);
  if (scale.value <= 1) {
    offsetX.value = 0;
    offsetY.value = 0;
  }
}

function zoomIn() {
  scale.value = Math.min(scale.value + 0.25, 8);
}

function zoomOut() {
  scale.value = Math.max(scale.value - 0.25, 0.1);
  if (scale.value <= 1) {
    offsetX.value = 0;
    offsetY.value = 0;
  }
}

function resetZoom() {
  scale.value = 1;
  offsetX.value = 0;
  offsetY.value = 0;
}

function handleDoubleClick() {
  if (scale.value === 1) {
    scale.value = 2;
  } else {
    resetZoom();
  }
}

function handleDragStart(e: MouseEvent) {
  if (!imageSrc.value) return;
  isDragging.value = true;
  startX.value = e.clientX - offsetX.value;
  startY.value = e.clientY - offsetY.value;
}

function handleDragMove(e: MouseEvent) {
  if (!isDragging.value) return;
  offsetX.value = e.clientX - startX.value;
  offsetY.value = e.clientY - startY.value;
}

function handleDragEnd() {
  isDragging.value = false;
}

function closeWindow() {
  void getCurrentWebviewWindow().close();
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") closeWindow();
}

onMounted(async () => {
  window.addEventListener("keydown", onKeydown);
  await loadPreviewImage();
  // Retry once — window may mount before cache write is visible.
  if (!imageSrc.value) {
    await new Promise((r) => setTimeout(r, 80));
    await loadPreviewImage();
  }
  unlistenUpdated = await listen("preview-image-updated", () => {
    void loadPreviewImage();
  });
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
  if (unlistenUpdated) {
    unlistenUpdated();
    unlistenUpdated = null;
  }
  void setOverlayPopupOpen("overlay", false).catch(() => undefined);
});
</script>

<style scoped>
.preview-shell {
  display: flex;
  flex-direction: column;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  user-select: none;
  color: var(--peek-text);
  background: var(--peek-bg);
  font-family: var(--peek-font-sans, system-ui, sans-serif);
}

.preview-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  height: 40px;
  padding: 0 10px 0 14px;
  border-bottom: 1px solid var(--peek-border);
  background: var(--peek-surface);
}

.preview-title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
  font-weight: 500;
  color: var(--peek-muted);
}

.preview-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: none;
}

.bar-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  margin: 0;
  padding: 0;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}

.bar-btn:hover {
  background: color-mix(in srgb, var(--peek-accent) 14%, transparent);
  color: var(--peek-text);
}

.bar-btn:disabled {
  opacity: 0.5;
  cursor: wait;
}

.bar-btn.close:hover {
  background: color-mix(in srgb, #e81123 16%, transparent);
  color: #e81123;
}

.zoom-label {
  min-width: 40px;
  text-align: center;
  font-size: 11px;
  font-weight: 600;
  color: var(--peek-faint);
}

.preview-stage {
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  cursor: grab;
  color-scheme: only light;
  background: color-mix(in srgb, var(--peek-bg) 88%, black 12%);
}

.preview-stage:active {
  cursor: grabbing;
}

.preview-img {
  max-width: min(92vw, 100%);
  max-height: calc(100vh - 56px);
  object-fit: contain;
  border-radius: 10px;
  transform-origin: center center;
  will-change: transform;
  pointer-events: none;
  color-scheme: only light;
}

.preview-empty {
  margin: 0;
  color: var(--peek-faint);
  font-size: 13px;
}
</style>
