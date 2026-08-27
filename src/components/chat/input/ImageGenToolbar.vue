<template>
  <div class="image-gen-toolbar" data-tauri-drag-region="false">
    <button
      type="button"
      class="image-gen-pill"
      data-picker-trigger
      data-tauri-drag-region="false"
      :class="{ open: settingsOpen }"
      :aria-expanded="settingsOpen"
      aria-haspopup="dialog"
      :title="tr(language, 'imageGen.ratioTitle')"
      @mousedown.stop
      @click.stop="onOpen('ratio', $event)"
    >
      <span class="image-gen-seg ratio-seg">
        <span class="ratio-smart" aria-hidden="true">
          <i class="corner tl"></i>
          <i class="corner tr"></i>
          <i class="corner bl"></i>
          <i class="corner br"></i>
        </span>
        <span>{{ ratioLabel }}</span>
      </span>
      <span class="image-gen-split" aria-hidden="true"></span>
      <span class="image-gen-seg">{{ resolutionLabel }}</span>
      <span class="image-gen-split" aria-hidden="true"></span>
      <span class="image-gen-seg">{{ modelValue.count }}</span>
    </button>

    <button
      type="button"
      class="image-gen-style"
      data-picker-trigger
      data-tauri-drag-region="false"
      :class="{ open: openField === 'style' }"
      :aria-expanded="openField === 'style'"
      aria-haspopup="listbox"
      :title="tr(language, 'imageGen.style')"
      @mousedown.stop
      @click.stop="onOpen('style', $event)"
    >
      {{ styleLabel }}
    </button>

    <button
      type="button"
      class="image-gen-style image-gen-model"
      data-picker-trigger
      data-tauri-drag-region="false"
      :class="{ open: openField === 'model' }"
      :aria-expanded="openField === 'model'"
      aria-haspopup="listbox"
      :title="modelLabel"
      @mousedown.stop
      @click.stop="onOpen('model', $event)"
    >
      {{ modelLabel }}
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { tr } from "@/services/i18n";
import type { ChatI18nKey } from "@/services/locales/chat";
import type { AppLanguage } from "@/types/setting";
import {
  IMAGE_GEN_RATIOS,
  IMAGE_GEN_RESOLUTIONS,
  IMAGE_GEN_STYLE_PRESETS,
  isImageGenSettingsField,
  listImageModelChoices,
  selectedImageModelChoiceId,
  decodeImageModelSelection,
  type ImageGenCompose,
  type ImageGenFieldId,
} from "@/services/chat/imageGenMode";
import { useSettingStore } from "@/stores/setting";

const props = defineProps<{
  modelValue: ImageGenCompose;
  language: AppLanguage;
  openField?: ImageGenFieldId | null;
}>();
const emit = defineEmits<{
  open: [id: ImageGenFieldId, button: HTMLElement];
}>();

const settingStore = useSettingStore();

const settingsOpen = computed(() => isImageGenSettingsField(props.openField));

const ratioLabel = computed(() => {
  if (props.modelValue.ratio === "auto") {
    return tr(props.language, "imageGen.ratioSmart");
  }
  const ratio = IMAGE_GEN_RATIOS.find((item) => item.id === props.modelValue.ratio);
  return tr(props.language, (ratio?.labelKey ?? "imageGen.ratio.auto") as ChatI18nKey);
});

const resolutionLabel = computed(
  () =>
    IMAGE_GEN_RESOLUTIONS.find((item) => item.id === props.modelValue.resolution)?.shortLabel ??
    "2K",
);

const styleLabel = computed(() => {
  const builtin = IMAGE_GEN_STYLE_PRESETS.find((item) => item.id === props.modelValue.styleId);
  if (builtin) return tr(props.language, builtin.labelKey as ChatI18nKey);
  const custom = settingStore.imageStyleTemplates.find(
    (item) => item.id === props.modelValue.styleId,
  );
  return custom?.name ?? tr(props.language, "imageGen.style.none");
});

const modelLabel = computed(() => {
  const choices = listImageModelChoices(settingStore.imageProviders);
  const selected = selectedImageModelChoiceId(
    settingStore.imageModelProvider,
    settingStore.imageModel,
    choices,
  );
  if (selected) {
    return decodeImageModelSelection(selected)?.model ?? settingStore.imageModel;
  }
  return settingStore.imageModel.trim() || tr(props.language, "imageGen.model");
});

function onOpen(id: ImageGenFieldId, event: MouseEvent) {
  const button = event.currentTarget;
  if (button instanceof HTMLElement) {
    emit("open", id, button);
  }
}
</script>

<style scoped>
.image-gen-toolbar {
  display: flex;
  flex-wrap: nowrap;
  align-items: center;
  gap: 8px;
  width: 100%;
  min-width: 0;
}
.image-gen-pill,
.image-gen-style {
  display: inline-flex;
  align-items: center;
  flex: none;
  height: 28px;
  margin: 0;
  border: 1px solid color-mix(in srgb, var(--peek-text) 12%, transparent);
  background: var(--peek-surface, #fff);
  color: var(--peek-text);
  cursor: pointer;
  white-space: nowrap;
}
.image-gen-pill {
  gap: 0;
  padding: 0 4px;
  border-radius: 8px;
}
.image-gen-style {
  min-width: 0;
  max-width: 140px;
  padding: 0 10px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 550;
  overflow: hidden;
  text-overflow: ellipsis;
}
.image-gen-pill.open,
.image-gen-style.open,
.image-gen-pill:hover,
.image-gen-style:hover {
  border-color: color-mix(in srgb, var(--peek-text) 20%, transparent);
  background: color-mix(in srgb, var(--peek-text) 4%, var(--peek-surface, #fff));
}
.image-gen-seg {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 0 10px;
  font-size: 12px;
  font-weight: 550;
  line-height: 1;
}
.image-gen-split {
  width: 1px;
  height: 12px;
  background: color-mix(in srgb, var(--peek-text) 14%, transparent);
}
.ratio-smart {
  position: relative;
  flex: none;
  width: 13px;
  height: 13px;
  color: var(--peek-faint);
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
</style>
