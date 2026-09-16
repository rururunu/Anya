<template>
  <ul
    class="command-list ask-user-list peek-scrollbar"
    :class="{ 'plan-switch-list': planSwitch }"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
    :aria-multiselectable="multiSelect ? true : undefined"
  >
    <li class="picker-sticky-head">
      <div class="picker-meta">
        <span class="picker-meta-label" :class="{ 'plan-switch': planSwitch }">
          <ListChecks
            v-if="planSwitch"
            class="picker-meta-icon"
            :size="13"
            :stroke-width="2.25"
            aria-hidden="true"
          />
          {{ header }}
        </span>
        <span v-if="questionCount > 1" class="picker-meta-progress">
          {{ questionIndex + 1 }}/{{ questionCount }}
        </span>
        <span v-else-if="multiSelect && selectedCount > 0" class="picker-meta-progress">
          {{ selectedCountLabel }}
        </span>
      </div>
      <div class="picker-meta question">{{ question }}</div>
    </li>

    <li
      v-for="(option, index) in options"
      :key="`${option.slug}-${index}`"
      class="command-item"
      :class="{
        active: index === selectedIndex,
        selected: multiSelect && !option.isSkip && isOptionSelected(option.label),
      }"
      role="option"
      :aria-selected="
        multiSelect && !option.isSkip ? isOptionSelected(option.label) : index === selectedIndex
      "
      @mouseenter="$emit('hover', index)"
      @mousedown.prevent="$emit('select', option)"
    >
      <span
        v-if="multiSelect && !option.isSkip"
        class="ask-checkbox"
        :class="{ checked: isOptionSelected(option.label) }"
        aria-hidden="true"
      >
        <Check v-if="isOptionSelected(option.label)" :size="11" :stroke-width="2.75" />
      </span>
      <span
        v-else-if="optionLeadingIcon(option)"
        class="ask-leading"
        :class="{ 'plan-accept': option.label === PLAN_SWITCH_ACCEPT_LABEL }"
        aria-hidden="true"
      >
        <component :is="optionLeadingIcon(option)" :size="13" :stroke-width="2.25" />
      </span>

      <span class="ask-body">
        <span class="ask-label">{{ option.label }}</span>
        <span v-if="option.description && option.description !== option.label" class="ask-desc">
          {{ option.description }}
        </span>
      </span>
    </li>

    <li
      v-if="multiSelect"
      class="command-item confirm-item"
      :class="{ active: selectedIndex === confirmRowIndex, disabled: selectedCount === 0 }"
      role="option"
      :aria-disabled="selectedCount === 0"
      @mouseenter="$emit('hover', confirmRowIndex)"
      @mousedown.prevent="selectedCount > 0 && $emit('confirm')"
    >
      <span class="ask-leading confirm-mark" aria-hidden="true">
        <Check :size="13" :stroke-width="2.5" />
      </span>
      <span class="ask-body">
        <span class="ask-label">{{ confirmLabel }}</span>
        <span v-if="selectedCount > 0" class="ask-desc">{{ selectedCountLabel }}</span>
      </span>
    </li>
  </ul>
</template>

<script setup lang="ts">
import type { Component } from "vue";
import { Check, ListChecks, PenLine, Sparkle } from "@lucide/vue";
import type { AskDisplayOption } from "@/types/chat";
import { PLAN_SWITCH_ACCEPT_LABEL, PLAN_SWITCH_DECLINE_LABEL } from "@/services/chat/askUserAnswer";

const props = defineProps<{
  header?: string;
  question?: string;
  questionIndex: number;
  questionCount: number;
  options: AskDisplayOption[];
  multiSelect?: boolean;
  confirmRowIndex: number;
  confirmLabel: string;
  selectedCount: number;
  selectedCountLabel: string;
  selectedIndex: number;
  ariaLabel: string;
  isOptionSelected: (label: string) => boolean;
  planSwitch?: boolean;
}>();

function optionLeadingIcon(option: AskDisplayOption): Component | undefined {
  if (option.isSkip) return PenLine;
  if (!props.planSwitch) return undefined;
  if (option.label === PLAN_SWITCH_ACCEPT_LABEL) return ListChecks;
  if (option.label === PLAN_SWITCH_DECLINE_LABEL) return Sparkle;
  return undefined;
}

defineEmits<{
  hover: [index: number];
  select: [option: AskDisplayOption];
  confirm: [];
}>();
</script>

<style scoped>
.command-list {
  --command-row-height: 36px;
  --command-list-padding: 8px;
  --picker-meta-row-height: 28px;
  --command-list-visible-rows: 8;
  list-style: none;
  margin: 0;
  padding: 4px 0;
  border-bottom: 1px solid var(--peek-border);
  background: var(--peek-list-bg);
  flex: none;
  max-height: min(
    calc(
      var(--command-row-height) * var(--command-list-visible-rows) + var(--command-list-padding)
    ),
    72vh
  );
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.ask-user-list {
  max-height: min(
    var(--interaction-picker-max-height, 48vh),
    calc(
      var(--picker-meta-row-height) * 2 + var(--command-row-height) *
        var(--command-list-visible-rows) + var(--command-list-padding) + 48px
    )
  );
}

.picker-sticky-head {
  position: sticky;
  top: 0;
  z-index: 3;
  margin: 0;
  padding: 0 0 4px;
  list-style: none;
  background: var(--peek-list-bg);
}

.ask-user-list .picker-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 0 12px;
  min-height: 26px;
  font-size: 11px;
  color: var(--peek-muted);
  pointer-events: none;
  background: inherit;
}

.ask-user-list .picker-meta.question {
  min-height: 28px;
  align-items: flex-start;
  padding-top: 2px;
  padding-bottom: 6px;
  line-height: 1.45;
  color: var(--peek-text);
  white-space: normal;
  overflow-wrap: anywhere;
}

.picker-meta-label {
  font-weight: 600;
  color: var(--peek-accent);
}

.picker-meta-label.plan-switch {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.picker-meta-icon {
  flex: none;
}

.ask-leading.plan-accept {
  color: var(--peek-accent);
}

.picker-meta-progress {
  flex: none;
  font-variant-numeric: tabular-nums;
}

.command-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 12px;
  min-height: var(--command-row-height);
  height: auto;
  cursor: default;
}

.command-item.active {
  background: color-mix(in srgb, var(--peek-accent) 10%, transparent);
}

.command-item.selected {
  background: color-mix(in srgb, var(--peek-accent) 8%, transparent);
}

.command-item.selected.active {
  background: color-mix(in srgb, var(--peek-accent) 14%, transparent);
}

.ask-checkbox {
  flex: none;
  box-sizing: border-box;
  width: 15px;
  height: 15px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1.5px solid color-mix(in srgb, var(--peek-text) 28%, transparent);
  border-radius: 3px;
  background: transparent;
  color: transparent;
}

.ask-checkbox.checked {
  border-color: var(--peek-accent);
  background: var(--peek-accent);
  color: #fff;
}

.ask-leading {
  flex: none;
  width: 15px;
  height: 15px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--peek-muted);
}

.ask-leading.confirm-mark {
  color: var(--peek-accent);
}

.ask-body {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.ask-label {
  font-size: 13px;
  line-height: 18px;
  color: var(--peek-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ask-desc {
  font-size: 11px;
  line-height: 15px;
  color: var(--peek-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.confirm-item.disabled {
  opacity: 0.4;
}

.confirm-item .ask-label {
  color: var(--peek-accent);
}
</style>
