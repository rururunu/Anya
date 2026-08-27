<template>
  <section class="generated-image-card" :class="[activity.status]">
    <p v-if="activity.status === 'running'" class="generated-image-placeholder">
      <Paintbrush :size="13" class="generated-image-draw" aria-hidden="true" />
      {{ generatingLabel }}
    </p>
    <div v-else-if="activity.status === 'error' && errorText" class="generated-image-error">
      <Markdown :content="errorMarkdown" />
    </div>
    <div
      v-for="source in sources"
      :key="source"
      class="generated-image-media"
      :class="{ selecting: selectingSource === source }"
    >
      <div class="generated-image-frame" :class="{ enlarged: selectingSource === source }">
        <button
          v-if="selectingSource !== source"
          type="button"
          class="generated-image-hit"
          :aria-label="previewLabel"
          @click="emit('previewImage', source)"
        >
          <img :src="resolveChatImageSrc(source)" alt="" draggable="false" />
        </button>

        <div
          v-else
          class="generated-image-select-stage"
          @pointerdown="onPaintPointerDown($event, source)"
          @pointermove="onPaintPointerMove"
          @pointerup="onPaintPointerUp"
          @pointercancel="onPaintPointerUp"
          @wheel.prevent="onBrushWheel"
        >
          <img
            :src="resolveChatImageSrc(source)"
            alt=""
            draggable="false"
            @load="syncPaintCanvasFromStage"
          />
          <canvas class="paint-canvas" aria-hidden="true" />
          <p class="select-hint">{{ selectHintLabel }}</p>
        </div>

        <div v-if="selectingSource !== source" class="generated-image-overlay">
          <PopoverRoot v-if="promptFor(source)">
            <PopoverTrigger as-child>
              <button
                type="button"
                class="overlay-icon overlay-prompt"
                :aria-label="viewPromptLabel"
                :title="viewPromptLabel"
                @click.stop
              >
                <TextQuote :size="13" :stroke-width="2.2" aria-hidden="true" />
              </button>
            </PopoverTrigger>
            <PopoverPortal>
              <PopoverContent
                class="generated-image-prompt-card"
                side="bottom"
                align="end"
                :side-offset="8"
                :collision-padding="12"
                data-tauri-drag-region="false"
                @click.stop
              >
                <header>{{ promptLabel }}</header>
                <p>{{ promptFor(source) }}</p>
              </PopoverContent>
            </PopoverPortal>
          </PopoverRoot>
          <button
            type="button"
            class="overlay-save"
            :disabled="savingSource === source"
            :aria-label="saveAsLabel"
            @click.stop="saveAs(source)"
          >
            {{ saveAsLabel }}
          </button>
          <button
            type="button"
            class="overlay-icon overlay-expand"
            :aria-label="previewLabel"
            :title="previewLabel"
            @click.stop="emit('previewImage', source)"
          >
            <Maximize2 :size="13" :stroke-width="2.2" aria-hidden="true" />
          </button>
        </div>
      </div>

      <div v-if="activity.status === 'done'" class="generated-image-actions">
        <template v-if="selectingSource === source">
          <div class="brush-size" role="group" :aria-label="brushSizeLabel">
            <span class="brush-size-label">{{ brushSizeLabel }}</span>
            <button
              v-for="option in brushSizeOptions"
              :key="option.id"
              type="button"
              class="brush-size-btn"
              :class="{ active: brushSizeId === option.id }"
              :title="option.title"
              :aria-pressed="brushSizeId === option.id"
              @click="brushSizeId = option.id"
            >
              <span
                class="brush-dot"
                :style="{ width: `${option.dot}px`, height: `${option.dot}px` }"
              />
              {{ option.label }}
            </button>
          </div>
          <button
            type="button"
            class="action-btn ghost"
            :disabled="strokes.length === 0"
            @click="clearStrokes"
          >
            {{ clearLabel }}
          </button>
          <button type="button" class="action-btn ghost" @click="cancelSelect">
            {{ cancelLabel }}
          </button>
          <button
            type="button"
            class="action-btn primary"
            :disabled="!hasPaint || busySource === source"
            @click="confirmRegion(source)"
          >
            {{ useRegionLabel }}
          </button>
        </template>
        <template v-else>
          <button
            type="button"
            class="action-btn"
            :disabled="busySource === source"
            :title="editFromImageTitle"
            @click="editFull(source)"
          >
            <Paintbrush :size="12" :stroke-width="2.2" aria-hidden="true" />
            {{ editFromImageLabel }}
          </button>
          <button
            type="button"
            class="action-btn"
            :disabled="busySource === source"
            :title="selectRegionTitle"
            @click="startSelect(source)"
          >
            <Brush :size="12" :stroke-width="2.2" aria-hidden="true" />
            {{ selectRegionLabel }}
          </button>
        </template>
      </div>
    </div>
    <p v-if="saveError || editError" class="generated-image-save-error" role="alert">
      {{ saveError || editError }}
    </p>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { Brush, Maximize2, Paintbrush, TextQuote } from "@lucide/vue";
import { PopoverContent, PopoverPortal, PopoverRoot, PopoverTrigger } from "reka-ui";
import Markdown from "@/components/chat/Markdown.vue";
import { useGeneratedImagePaint } from "@/composables/chat/useGeneratedImagePaint";
import type { ToolActivity } from "@/types/chat";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import {
  generatedImagePrompt,
  parseGeneratedImageSources,
  resolveChatImageSrc,
} from "@/services/chat/localImageSrc";
import {
  prepareFullImageEdit,
  prepareStrokeImageEdit,
  type ImageEditReferencePayload,
} from "@/services/chat/imageEditReference";
import { saveChatImage } from "@/services/chat/saveChatImage";

const props = defineProps<{
  activity: ToolActivity;
}>();
const emit = defineEmits<{
  previewImage: [source: string];
  editFromImage: [payload: ImageEditReferencePayload];
}>();

const settingStore = useSettingStore();
const savingSource = ref("");
const busySource = ref("");
const saveError = ref("");
const editError = ref("");

const {
  selectingSource,
  strokes,
  brushSizeId,
  hasPaint,
  startSelect: beginPaintSelect,
  cancelSelect,
  clearStrokes,
  syncPaintCanvasFromStage,
  onPaintPointerDown,
  onPaintPointerMove,
  onPaintPointerUp,
  onBrushWheel,
} = useGeneratedImagePaint();

const sources = computed(() => parseGeneratedImageSources(props.activity.result));
const errorText = computed(() => {
  const result = props.activity.result?.trim();
  if (result) return result;
  return props.activity.detail?.trim() ?? "";
});
const errorMarkdown = computed(() => {
  const text = errorText.value;
  if (!text) return "";
  return text.startsWith("```") ? text : `\`\`\`\n${text}\n\`\`\``;
});
const generatingLabel = computed(() => tr(settingStore.language, "image.generating"));
const previewLabel = computed(() => tr(settingStore.language, "image.preview"));
const saveAsLabel = computed(() => tr(settingStore.language, "image.saveAs"));
const viewPromptLabel = computed(() => tr(settingStore.language, "image.viewPrompt"));
const promptLabel = computed(() => tr(settingStore.language, "image.prompt"));
const editFromImageLabel = computed(() => tr(settingStore.language, "image.editFrom"));
const editFromImageTitle = computed(() => tr(settingStore.language, "image.editFromTitle"));
const selectRegionLabel = computed(() => tr(settingStore.language, "image.selectRegion"));
const selectRegionTitle = computed(() => tr(settingStore.language, "image.selectRegionTitle"));
const selectHintLabel = computed(() => tr(settingStore.language, "image.selectHint"));
const useRegionLabel = computed(() => tr(settingStore.language, "image.useRegion"));
const cancelLabel = computed(() => tr(settingStore.language, "image.cancelSelect"));
const clearLabel = computed(() => tr(settingStore.language, "image.clearPaint"));
const brushSizeLabel = computed(() => tr(settingStore.language, "image.brushSize"));
const brushSizeOptions = computed(() => [
  {
    id: "fine" as const,
    label: tr(settingStore.language, "image.brushFine"),
    title: tr(settingStore.language, "image.brushFineTitle"),
    dot: 5,
  },
  {
    id: "medium" as const,
    label: tr(settingStore.language, "image.brushMedium"),
    title: tr(settingStore.language, "image.brushMediumTitle"),
    dot: 9,
  },
  {
    id: "bold" as const,
    label: tr(settingStore.language, "image.brushBold"),
    title: tr(settingStore.language, "image.brushBoldTitle"),
    dot: 14,
  },
]);

function promptFor(source: string) {
  return generatedImagePrompt(props.activity.result, props.activity.arguments, source);
}

async function startSelect(source: string) {
  editError.value = "";
  await beginPaintSelect(source);
}

async function editFull(source: string) {
  if (busySource.value) return;
  busySource.value = source;
  editError.value = "";
  try {
    const payload = await prepareFullImageEdit(source);
    emit("editFromImage", payload);
  } catch (error) {
    console.error("editFromImage failed:", error);
    editError.value = tr(settingStore.language, "image.editPrepareFailed");
  } finally {
    busySource.value = "";
  }
}

async function confirmRegion(source: string) {
  if (!hasPaint.value || busySource.value) return;
  busySource.value = source;
  editError.value = "";
  try {
    const payload = await prepareStrokeImageEdit(source, strokes.value);
    emit("editFromImage", payload);
    cancelSelect();
  } catch (error) {
    console.error("region edit failed:", error);
    editError.value = tr(settingStore.language, "image.editPrepareFailed");
  } finally {
    busySource.value = "";
  }
}

async function saveAs(source: string) {
  if (savingSource.value) return;
  savingSource.value = source;
  saveError.value = "";
  try {
    const result = await saveChatImage(source);
    if (result === "failed") {
      saveError.value = tr(settingStore.language, "image.saveFailed");
    }
  } finally {
    savingSource.value = "";
  }
}
</script>

<style scoped>
.generated-image-card {
  display: flex;
  flex-flow: row wrap;
  align-items: flex-start;
  align-self: flex-start;
  gap: 8px;
  width: fit-content;
  max-width: 100%;
}
.generated-image-placeholder {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin: 0;
  color: var(--peek-muted);
  font-size: 12px;
}
.generated-image-draw {
  flex: none;
  color: var(--peek-accent);
  animation: generated-image-draw 1.6s ease-in-out infinite;
}
@keyframes generated-image-draw {
  0%,
  100% {
    opacity: 0.55;
    transform: rotate(-12deg);
  }
  50% {
    opacity: 1;
    transform: rotate(8deg);
  }
}
.generated-image-error {
  max-width: 100%;
  color: var(--destructive);
  font-size: 12px;
  line-height: 1.45;
  word-break: break-word;
}
.generated-image-error :deep(p) {
  margin: 0;
}
.generated-image-media {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
  max-width: 100%;
  line-height: 0;
}
.generated-image-media.selecting {
  max-width: min(100%, 560px);
}
.generated-image-frame {
  position: relative;
  display: inline-block;
  max-width: 100%;
  transition:
    max-width 160ms ease,
    transform 160ms ease;
}
.generated-image-frame.enlarged {
  max-width: min(100%, 560px);
}
.generated-image-hit {
  display: block;
  max-width: 100%;
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  cursor: zoom-in;
}
.generated-image-hit img {
  display: block;
  width: auto;
  height: auto;
  max-width: min(100%, 360px);
  max-height: 280px;
  object-fit: contain;
  border-radius: 8px;
}
.generated-image-select-stage {
  position: relative;
  display: inline-block;
  max-width: 100%;
  cursor: crosshair;
  touch-action: none;
  user-select: none;
}
.generated-image-select-stage img {
  display: block;
  width: auto;
  height: auto;
  max-width: min(100%, 560px);
  max-height: min(72vh, 520px);
  object-fit: contain;
  border-radius: 10px;
  outline: 2px solid color-mix(in srgb, var(--peek-accent) 55%, transparent);
  outline-offset: 1px;
}
.paint-canvas {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  border-radius: 10px;
  pointer-events: none;
}
.select-hint {
  position: absolute;
  left: 10px;
  bottom: 10px;
  margin: 0;
  padding: 4px 10px;
  border-radius: 999px;
  background: rgb(20 20 20 / 72%);
  color: #fff;
  font-size: 11px;
  line-height: 1.3;
  pointer-events: none;
}
.generated-image-overlay {
  position: absolute;
  inset: 0;
  pointer-events: none;
}
.generated-image-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  line-height: 1.2;
}
.brush-size {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-right: 2px;
}
.brush-size-label {
  color: var(--peek-muted);
  font-size: 11px;
  font-weight: 550;
}
.brush-size-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 26px;
  margin: 0;
  padding: 0 8px;
  border: 1px solid color-mix(in srgb, var(--peek-text) 12%, transparent);
  border-radius: 999px;
  background: var(--peek-surface, #fff);
  color: var(--peek-muted);
  font-size: 11px;
  font-weight: 550;
  cursor: pointer;
}
.brush-size-btn:hover {
  border-color: color-mix(in srgb, var(--peek-text) 22%, transparent);
  color: var(--peek-text);
}
.brush-size-btn.active {
  border-color: color-mix(in srgb, var(--peek-accent) 40%, transparent);
  background: color-mix(in srgb, var(--peek-accent) 10%, var(--peek-surface, #fff));
  color: var(--peek-accent);
}
.brush-dot {
  flex: none;
  border-radius: 50%;
  background: currentColor;
  opacity: 0.85;
}
.action-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 26px;
  margin: 0;
  padding: 0 9px;
  border: 1px solid color-mix(in srgb, var(--peek-text) 12%, transparent);
  border-radius: 999px;
  background: var(--peek-surface, #fff);
  color: var(--peek-text);
  font-size: 11px;
  font-weight: 550;
  cursor: pointer;
}
.action-btn:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--peek-text) 22%, transparent);
  background: color-mix(in srgb, var(--peek-text) 4%, var(--peek-surface, #fff));
}
.action-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
.action-btn.primary {
  border-color: color-mix(in srgb, var(--peek-accent) 40%, transparent);
  background: color-mix(in srgb, var(--peek-accent) 12%, var(--peek-surface, #fff));
  color: var(--peek-accent);
}
.action-btn.primary:disabled {
  cursor: wait;
}
.action-btn.ghost {
  color: var(--peek-muted);
}
.overlay-save,
.overlay-icon {
  pointer-events: auto;
  margin: 0;
  border: 0;
  color: #fff;
  background: rgb(20 20 20 / 72%);
  backdrop-filter: blur(10px);
  cursor: pointer;
}
.overlay-save {
  position: absolute;
  left: 6px;
  bottom: 6px;
  height: 26px;
  padding: 0 10px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.02em;
}
.overlay-icon {
  position: absolute;
  top: 6px;
  right: 6px;
  width: 26px;
  height: 26px;
  display: grid;
  place-items: center;
  padding: 0;
  border-radius: 50%;
}
.overlay-expand {
  top: auto;
  bottom: 6px;
}
.overlay-save:hover,
.overlay-icon:hover,
.overlay-icon[data-state="open"] {
  background: rgb(20 20 20 / 86%);
}
.overlay-save:disabled {
  opacity: 0.65;
  cursor: wait;
}
.generated-image-save-error {
  margin: 0;
  color: var(--destructive);
  font-size: 12px;
  line-height: 1.4;
}
</style>

<style>
.generated-image-prompt-card {
  z-index: 80;
  width: min(360px, calc(100vw - 24px));
  max-height: min(50vh, 320px);
  overflow: auto;
  padding: 12px 14px 10px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--popover);
  color: var(--popover-foreground);
  box-shadow: var(--peek-shadow, 0 10px 30px rgb(0 0 0 / 12%));
  outline: none;
}
.generated-image-prompt-card header {
  margin: 0 0 8px;
  color: var(--peek-muted);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.02em;
}
.generated-image-prompt-card p {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
