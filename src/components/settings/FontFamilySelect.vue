<template>
  <div class="font-family-select">
    <Select :model-value="selectValue" @update:model-value="onSelect">
      <SelectTrigger class="w-full">
        <SelectValue />
      </SelectTrigger>
      <SelectContent position="popper" align="end" :side-offset="6">
        <SelectItem :value="FONT_DEFAULT">{{ defaultFontLabel(kind, language) }}</SelectItem>
        <SelectItem v-for="family in fontPresetFamilies(kind)" :key="family" :value="family">
          {{ family }}
        </SelectItem>
        <SelectItem :value="FONT_CUSTOM">{{ customFontLabel(language) }}</SelectItem>
      </SelectContent>
    </Select>
    <Input
      v-if="selectValue === FONT_CUSTOM"
      :model-value="customDraft"
      :placeholder="customFontPlaceholder(language)"
      class="font-custom-input"
      @update:model-value="onCustomDraft"
      @blur="commitCustom"
      @keydown.enter.prevent="commitCustom"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  FONT_CUSTOM,
  FONT_DEFAULT,
  customFontLabel,
  customFontPlaceholder,
  defaultFontLabel,
  fontPresetFamilies,
  fontSelectValue,
  fontSettingKey,
  sanitizeFontName,
  type FontKind,
} from "@/services/theme/fonts";
import { useSettingStore } from "@/stores/setting";

const props = defineProps<{
  kind: FontKind;
}>();

const settingStore = useSettingStore();
const language = computed(() => settingStore.language);
const stored = computed(() => settingStore[fontSettingKey(props.kind)] ?? "");
const forceCustom = ref(fontSelectValue(stored.value, props.kind) === FONT_CUSTOM);
const selectValue = computed(() =>
  forceCustom.value ? FONT_CUSTOM : fontSelectValue(stored.value, props.kind),
);
const customDraft = ref(selectValue.value === FONT_CUSTOM ? stored.value : "");

watch(stored, (value) => {
  if (fontSelectValue(value, props.kind) === FONT_CUSTOM) {
    forceCustom.value = true;
    customDraft.value = value;
  }
});

function persist(family: string) {
  const key = fontSettingKey(props.kind);
  if (settingStore[key] === family) return;
  void settingStore.update({ [key]: family });
}

function onSelect(value: unknown) {
  if (typeof value !== "string") return;
  if (value === FONT_CUSTOM) {
    forceCustom.value = true;
    customDraft.value =
      fontSelectValue(stored.value, props.kind) === FONT_CUSTOM ? stored.value : "";
    return;
  }
  forceCustom.value = false;
  persist(value === FONT_DEFAULT ? "" : value);
}

function onCustomDraft(value: string | number) {
  customDraft.value = String(value);
}

function commitCustom() {
  const next = sanitizeFontName(customDraft.value);
  if (!next) {
    forceCustom.value = false;
    persist("");
    return;
  }
  persist(next);
}
</script>

<style scoped>
.font-family-select {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
  min-width: 0;
}
</style>
