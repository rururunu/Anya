<template>
  <div class="font-family-select">
    <Select :model-value="selectValue" @update:model-value="onSelect">
      <SelectTrigger class="w-full">
        <SelectValue />
      </SelectTrigger>
      <SelectContent position="popper" align="end" :side-offset="6">
        <SelectItem :value="FONT_DEFAULT">{{ defaultFontLabel(kind, language) }}</SelectItem>
        <SelectItem v-for="family in fontOptions" :key="family" :value="family">
          {{ family }}
        </SelectItem>
        <SelectItem :value="FONT_CUSTOM">{{ customFontLabel(language) }}</SelectItem>
      </SelectContent>
    </Select>
    <span v-if="fontLoadState === 'ready'" class="font-source-hint">
      {{
        language === "zh-CN"
          ? `已读取本机 ${systemFonts.length} 款字体`
          : `${systemFonts.length} installed fonts found`
      }}
    </span>
    <span v-else-if="fontLoadState === 'loading'" class="font-source-hint">
      {{ language === "zh-CN" ? "正在读取本机字体…" : "Loading installed fonts…" }}
    </span>
    <button v-else type="button" class="font-source-retry" @click="refreshSystemFonts">
      {{ language === "zh-CN" ? "读取失败，点击重试" : "Could not load fonts. Retry" }}
      <span v-if="fontLoadError">({{ fontLoadError }})</span>
    </button>
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
import { computed, onMounted, ref, watch } from "vue";
import { Input } from "@/components/ui/input";
import { loadSystemFonts } from "@/services/theme/systemFonts";
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
const systemFonts = ref<string[]>([]);
const fontLoadState = ref<"loading" | "ready" | "error">("loading");
const fontLoadError = ref("");
const fontOptions = computed(() =>
  systemFonts.value.length ? systemFonts.value : fontPresetFamilies(props.kind),
);
const forceCustom = ref(false);
const selectValue = computed(() =>
  forceCustom.value ? FONT_CUSTOM : fontSelectValue(stored.value, props.kind, fontOptions.value),
);
const customDraft = ref(selectValue.value === FONT_CUSTOM ? stored.value : "");

async function refreshSystemFonts() {
  fontLoadState.value = "loading";
  fontLoadError.value = "";
  try {
    const families = await loadSystemFonts();
    systemFonts.value = families;
    fontLoadState.value = "ready";
  } catch (error) {
    console.error("Could not load installed fonts:", error);
    fontLoadError.value = String(error);
    fontLoadState.value = "error";
  }
}

onMounted(refreshSystemFonts);

watch(stored, (value) => {
  if (fontSelectValue(value, props.kind, fontOptions.value) === FONT_CUSTOM) {
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
      fontSelectValue(stored.value, props.kind, fontOptions.value) === FONT_CUSTOM
        ? stored.value
        : "";
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
.font-source-hint,
.font-source-retry {
  color: var(--peek-faint);
  font-size: 11px;
  text-align: left;
}
.font-source-retry {
  align-self: flex-start;
}
</style>
