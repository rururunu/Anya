<template>
  <details
    class="image-analysis-card"
    :open="open || undefined"
    role="group"
    :aria-label="ariaLabel"
  >
    <summary class="image-analysis-header">
      <ChevronRight class="analysis-chevron" :size="13" aria-hidden="true" />
      <span class="analysis-title">{{ title }}</span>
      <span v-if="model" class="analysis-model">{{ model }}</span>
    </summary>
    <div class="image-analysis-body">{{ text }}</div>
  </details>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { ChevronRight } from "@lucide/vue";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";

const props = defineProps<{
  model: string;
  text: string;
  /** Defaults to collapsed. */
  open?: boolean;
}>();

const settingStore = useSettingStore();
const title = computed(() => tr(settingStore.language, "imageAnalysis"));
const ariaLabel = computed(() => (props.model ? `${title.value} · ${props.model}` : title.value));
</script>

<style scoped>
.image-analysis-card {
  align-self: stretch;
  width: 100%;
  margin: 0;
  border: 0;
  border-radius: 6px;
  background: transparent;
  overflow: hidden;
  box-sizing: border-box;
}

.image-analysis-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  color: var(--peek-muted);
  font-size: 11px;
  line-height: 1.35;
  cursor: pointer;
  list-style: none;
  user-select: none;
  border-radius: 6px;
  transition:
    background 120ms ease,
    color 120ms ease;
}

.image-analysis-header:hover {
  background: color-mix(in srgb, var(--peek-text) 5%, transparent);
  color: var(--peek-text);
}

.image-analysis-header::-webkit-details-marker {
  display: none;
}

.analysis-chevron {
  flex: none;
  color: var(--peek-faint);
  transition: transform 140ms ease;
}

.image-analysis-card[open] > .image-analysis-header .analysis-chevron {
  transform: rotate(90deg);
}

.image-analysis-card[open] > .image-analysis-header {
  color: var(--peek-text);
  border-bottom: 0;
}

.analysis-title {
  color: inherit;
  font-weight: 550;
}

.analysis-model {
  margin-left: auto;
  color: var(--peek-faint);
  font-size: 10px;
  font-weight: 500;
}

.image-analysis-body {
  padding: 2px 6px 8px 28px;
  color: var(--peek-muted);
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
