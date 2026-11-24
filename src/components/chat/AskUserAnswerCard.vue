<template>
  <section class="ask-answer-card" role="status" :aria-label="label">
    <div class="ask-answer-header">
      <component
        :is="planSwitch ? ListChecks : CircleHelp"
        class="ask-answer-icon"
        :size="14"
        :stroke-width="2"
        aria-hidden="true"
      />
      <span class="ask-answer-title">{{ label }}</span>
    </div>

    <div class="ask-answer-body">
      <div
        v-for="(item, index) in items"
        :key="`${item.question ?? item.header ?? 'q'}-${index}`"
        class="ask-answer-row"
      >
        <div v-if="item.header && item.question" class="ask-answer-header-text">
          {{ item.header }}
        </div>
        <div v-if="promptFor(item)" class="ask-answer-question">{{ promptFor(item) }}</div>
        <ul v-if="item.options?.length" class="ask-answer-options">
          <li
            v-for="(option, optionIndex) in item.options"
            :key="optionIndex"
            :class="{ chosen: item.selected.includes(option.label.trim()) }"
          >
            <span class="ask-answer-mark">
              <Check
                v-if="item.selected.includes(option.label.trim())"
                :size="13"
                aria-hidden="true"
              />
            </span>
            <span>
              {{ option.label }}
              <small v-if="option.description">{{ option.description }}</small>
            </span>
          </li>
        </ul>
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
import { Check, CircleHelp, ListChecks } from "@lucide/vue";
import { useSettingStore } from "@/stores/setting";
import type { AskUserAnswerItem } from "@/types/chat";
import { tr } from "@/services/i18n";
import { isPlanSwitchAsk } from "@/services/chat/askUserAnswer";

const props = defineProps<{
  items: AskUserAnswerItem[];
}>();

const settingStore = useSettingStore();
const { language } = storeToRefs(settingStore);

const planSwitch = computed(() => isPlanSwitchAsk(props.items));
const label = computed(() =>
  planSwitch.value ? tr(language.value, "chatModePlan") : tr(language.value, "answers"),
);
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
  padding: 14px;
  border: 1px solid color-mix(in srgb, var(--peek-border) 88%, transparent);
  border-radius: 16px;
  background: color-mix(in srgb, var(--peek-text) 3%, var(--peek-bg));
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
  font-size: 14px;
  font-weight: 600;
  line-height: 1.55;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.ask-answer-header-text {
  color: var(--peek-muted);
  font-size: 12px;
}

.ask-answer-options {
  display: grid;
  gap: 6px;
  margin: 5px 0;
  padding: 0;
  list-style: none;
}

.ask-answer-options li {
  display: flex;
  align-items: flex-start;
  gap: 9px;
  padding: 8px 10px;
  border: 1px solid var(--peek-border);
  border-radius: 10px;
  background: var(--peek-bg);
  font-size: 13px;
  overflow-wrap: anywhere;
}

.ask-answer-options small {
  display: block;
  color: var(--peek-muted);
  font-size: 12px;
}

.ask-answer-options .chosen {
  background: color-mix(in srgb, var(--peek-text) 7%, var(--peek-bg));
  font-weight: 600;
}

.ask-answer-mark {
  display: grid;
  place-items: center;
  flex: 0 0 15px;
  height: 15px;
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
