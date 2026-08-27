<template>
  <div
    class="command-list image-gen-settings-panel"
    data-tauri-drag-region="false"
    role="dialog"
    :aria-label="ariaLabel"
    @mousedown.stop
  >
    <section class="settings-block">
      <h4 class="settings-label">{{ tr(language, "imageGen.ratioTitle") }}</h4>
      <div class="segment-track ratio-track">
        <button
          v-for="item in IMAGE_GEN_RATIOS"
          :key="item.id"
          type="button"
          class="ratio-option"
          :class="{ selected: modelValue.ratio === item.id }"
          :title="ratioLabel(item)"
          @click="emit('update:modelValue', applyImageGenRatio(modelValue, item.id))"
        >
          <span class="ratio-glyph" aria-hidden="true">
            <span v-if="item.id === 'auto'" class="ratio-smart">
              <i class="corner tl"></i>
              <i class="corner tr"></i>
              <i class="corner bl"></i>
              <i class="corner br"></i>
            </span>
            <span v-else class="ratio-rect" :style="ratioRectStyle(item.w, item.h)"></span>
          </span>
          <span class="ratio-text">{{ ratioLabel(item) }}</span>
        </button>
      </div>
    </section>

    <section class="settings-block">
      <h4 class="settings-label">{{ tr(language, "imageGen.resolutionTitle") }}</h4>
      <div class="segment-track">
        <button
          v-for="item in IMAGE_GEN_RESOLUTIONS"
          :key="item.id"
          type="button"
          class="segment-option"
          :class="{ selected: modelValue.resolution === item.id }"
          @click="emit('update:modelValue', applyImageGenResolution(modelValue, item.id))"
        >
          {{ tr(language, item.labelKey as ChatI18nKey) }}
        </button>
      </div>
    </section>

    <section class="settings-block">
      <h4 class="settings-label">{{ tr(language, "imageGen.countTitle") }}</h4>
      <div class="segment-track">
        <button
          v-for="count in IMAGE_GEN_COUNTS"
          :key="count"
          type="button"
          class="segment-option"
          :class="{ selected: modelValue.count === count }"
          @click="emit('update:modelValue', applyImageGenField(modelValue, 'count', String(count)))"
        >
          {{ count }}
        </button>
      </div>
    </section>

    <section class="settings-block size-block">
      <h4 class="settings-label">{{ tr(language, "imageGen.sizeTitle") }}</h4>
      <div class="size-row">
        <label class="size-field">
          <span>W</span>
          <input
            :value="modelValue.width"
            type="number"
            inputmode="numeric"
            autocomplete="off"
            spellcheck="false"
            :min="IMAGE_GEN_MIN_PX"
            :max="IMAGE_GEN_MAX_PX"
            :step="IMAGE_GEN_SIZE_STEP"
            @change="onWidthChange($event)"
          />
        </label>
        <button
          type="button"
          class="size-lock"
          :class="{ locked: modelValue.sizeLocked }"
          :title="tr(language, modelValue.sizeLocked ? 'imageGen.sizeUnlock' : 'imageGen.sizeLock')"
          :aria-pressed="modelValue.sizeLocked"
          @click="
            emit('update:modelValue', applyImageGenSizeLock(modelValue, !modelValue.sizeLocked))
          "
        >
          <Link2 :size="14" />
        </button>
        <label class="size-field">
          <span>H</span>
          <input
            :value="modelValue.height"
            type="number"
            inputmode="numeric"
            autocomplete="off"
            spellcheck="false"
            :min="IMAGE_GEN_MIN_PX"
            :max="IMAGE_GEN_MAX_PX"
            :step="IMAGE_GEN_SIZE_STEP"
            @change="onHeightChange($event)"
          />
        </label>
        <span class="size-unit">PX</span>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { Link2 } from "@lucide/vue";
import { tr } from "@/services/i18n";
import type { ChatI18nKey } from "@/services/locales/chat";
import type { AppLanguage } from "@/types/setting";
import {
  applyImageGenField,
  applyImageGenHeight,
  applyImageGenRatio,
  applyImageGenResolution,
  applyImageGenSizeLock,
  applyImageGenWidth,
  IMAGE_GEN_COUNTS,
  IMAGE_GEN_MAX_PX,
  IMAGE_GEN_MIN_PX,
  IMAGE_GEN_RATIOS,
  IMAGE_GEN_RESOLUTIONS,
  IMAGE_GEN_SIZE_STEP,
  type ImageGenCompose,
} from "@/services/chat/imageGenMode";

const props = defineProps<{
  modelValue: ImageGenCompose;
  language: AppLanguage;
  ariaLabel: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: ImageGenCompose];
}>();

function ratioLabel(item: (typeof IMAGE_GEN_RATIOS)[number]) {
  return tr(props.language, item.labelKey as ChatI18nKey);
}

function ratioRectStyle(w: number, h: number) {
  const maxW = 22;
  const maxH = 14;
  const scale = Math.min(maxW / w, maxH / h);
  return {
    width: `${Math.max(7, Math.round(w * scale))}px`,
    height: `${Math.max(7, Math.round(h * scale))}px`,
  };
}

function parsePx(event: Event): number | null {
  const target = event.target;
  if (!(target instanceof HTMLInputElement)) return null;
  const next = Number.parseInt(target.value.replace(/[^\d]/g, ""), 10);
  return Number.isFinite(next) ? next : null;
}

function onWidthChange(event: Event) {
  const next = parsePx(event);
  if (next == null) return;
  emit("update:modelValue", applyImageGenWidth(props.modelValue, next));
}

function onHeightChange(event: Event) {
  const next = parsePx(event);
  if (next == null) return;
  emit("update:modelValue", applyImageGenHeight(props.modelValue, next));
}
</script>

<style scoped>
.image-gen-settings-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
  box-sizing: border-box;
  padding: 14px 14px 12px;
  overflow: hidden;
  background: var(--peek-surface, #fff);
}
.settings-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
}
.settings-label {
  margin: 0;
  color: var(--peek-faint);
  font-size: 12px;
  font-weight: 500;
}
.segment-track {
  display: flex;
  align-items: stretch;
  min-width: 0;
  padding: 3px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--peek-text) 6%, transparent);
}
.ratio-track {
  gap: 0;
  overflow-x: auto;
}
.ratio-option,
.segment-option {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  margin: 0;
  border: 0;
  background: transparent;
  color: var(--peek-muted, var(--peek-text));
  cursor: pointer;
}
.ratio-option {
  flex-direction: column;
  gap: 4px;
  min-width: 48px;
  padding: 7px 4px 6px;
  border-radius: 8px;
}
.segment-option {
  min-height: 32px;
  padding: 0 8px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 550;
  white-space: nowrap;
}
.ratio-option.selected,
.segment-option.selected {
  background: var(--peek-surface, #fff);
  color: var(--peek-text);
  box-shadow: 0 1px 3px color-mix(in srgb, #000 10%, transparent);
}
.ratio-text {
  font-size: 11px;
  font-weight: 500;
  line-height: 1;
}
.ratio-glyph {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 16px;
}
.ratio-rect {
  display: block;
  border: 1.5px solid currentColor;
  border-radius: 2px;
  opacity: 0.75;
}
.ratio-option.selected .ratio-rect,
.ratio-option.selected .ratio-smart {
  opacity: 1;
}
.ratio-smart {
  position: relative;
  width: 14px;
  height: 14px;
  opacity: 0.75;
}
.ratio-smart .corner {
  position: absolute;
  display: block;
  width: 4px;
  height: 4px;
  border-color: currentColor;
  border-style: solid;
}
.ratio-smart .tl {
  top: 0;
  left: 0;
  border-width: 1.5px 0 0 1.5px;
}
.ratio-smart .tr {
  top: 0;
  right: 0;
  border-width: 1.5px 1.5px 0 0;
}
.ratio-smart .bl {
  bottom: 0;
  left: 0;
  border-width: 0 0 1.5px 1.5px;
}
.ratio-smart .br {
  right: 0;
  bottom: 0;
  border-width: 0 1.5px 1.5px 0;
}
.size-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.size-field {
  display: flex;
  flex: 1;
  align-items: center;
  gap: 8px;
  min-width: 0;
  height: 34px;
  padding: 0 10px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--peek-text) 6%, transparent);
  color: var(--peek-faint);
  font-size: 12px;
  font-weight: 600;
}
.size-field input {
  flex: 1;
  min-width: 0;
  height: 100%;
  border: 0;
  background: transparent;
  color: var(--peek-text);
  font-size: 13px;
  font-weight: 550;
  text-align: right;
  outline: none;
  appearance: textfield;
}
.size-field input::-webkit-outer-spin-button,
.size-field input::-webkit-inner-spin-button {
  appearance: none;
  margin: 0;
}
.size-lock {
  display: inline-flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  margin: 0;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--peek-faint);
  cursor: pointer;
}
.size-lock.locked,
.size-lock:hover {
  background: color-mix(in srgb, var(--peek-text) 7%, transparent);
  color: var(--peek-text);
}
.size-unit {
  flex: none;
  color: var(--peek-faint);
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.04em;
}
</style>
