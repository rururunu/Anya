<template>
  <section class="ask-answer-card" role="status" :aria-label="label">
    <div class="ask-answer-header">
      <Lightbulb class="ask-answer-icon" :size="14" :stroke-width="2" aria-hidden="true" />
      <span class="ask-answer-title">{{ label }}</span>
    </div>

    <div class="ask-answer-body">
      <div
        v-for="(item, index) in items"
        :key="`${item.question ?? item.header ?? 'q'}-${index}`"
        class="ask-answer-row"
      >
        <div v-if="promptFor(item)" class="ask-answer-question">{{ promptFor(item) }}</div>
        <div class="ask-answer-value" :class="{ muted: item.userSupplement }">
          {{ item.userSupplement ? supplementText : item.selected.join("、") }}
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { Lightbulb } from "@lucide/vue";
import { useSettingStore } from "@/stores/setting";
import type { AskUserAnswerItem } from "@/types/chat";
import { tr } from "@/services/i18n";

defineProps<{
  items: AskUserAnswerItem[];
}>();

const settingStore = useSettingStore();
const { language } = storeToRefs(settingStore);

const label = computed(() => tr(language.value, "answers"));
const supplementText = computed(() => tr(language.value, "customAnswer"));

function promptFor(item: AskUserAnswerItem) {
  return item.question?.trim() || item.header?.trim() || "";
}
</script>

<style scoped>
.ask-answer-card {
  align-self: stretch;
  width: 100%;
  max-width: none;
  margin: 8px 0 10px;
  padding: 10px 12px 8px;
  border: 1px solid color-mix(in srgb, var(--peek-border) 88%, transparent);
  border-radius: 12px;
  background: color-mix(in srgb, var(--peek-text) 3.5%, var(--peek-bg, transparent));
  box-sizing: border-box;
}

.ask-answer-header {
  display: flex;
  align-items: center;
  gap: 7px;
  min-height: 22px;
  margin-bottom: 8px;
  color: var(--peek-muted);
}

.ask-answer-icon {
  flex: none;
  color: var(--peek-muted);
}

.ask-answer-title {
  font-size: 12px;
  font-weight: 500;
  line-height: 1.3;
}

.ask-answer-body {
  display: flex;
  flex-direction: column;
}

.ask-answer-row {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
  padding: 8px 0;
}

.ask-answer-row + .ask-answer-row {
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 80%, transparent);
}

.ask-answer-question {
  color: var(--peek-muted);
  font-size: 12px;
  line-height: 1.45;
  overflow-wrap: anywhere;
}

.ask-answer-value {
  color: var(--peek-text);
  font-size: 13px;
  font-weight: 600;
  line-height: 1.5;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}

.ask-answer-value.muted {
  color: var(--peek-muted);
  font-weight: 500;
  font-style: italic;
}
</style>
