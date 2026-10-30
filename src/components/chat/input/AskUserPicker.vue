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
          <CircleHelp
            v-else
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
        'custom-option': Boolean(option.isSkip),
        'custom-input-active': Boolean(option.isSkip && customInputOpen),
      }"
      role="option"
      :aria-selected="
        multiSelect && !option.isSkip ? isOptionSelected(option.label) : index === selectedIndex
      "
      @mouseenter="$emit('hover', index)"
      @mousedown="onOptionMouseDown($event, option)"
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
        :class="{
          'plan-accept': option.label === PLAN_SWITCH_ACCEPT_LABEL,
          'custom-active': Boolean(option.isSkip && customInputOpen),
        }"
        aria-hidden="true"
      >
        <component :is="optionLeadingIcon(option)" :size="13" :stroke-width="2.25" />
      </span>

      <div v-if="option.isSkip && customInputOpen" class="ask-custom-input-wrap" @mousedown.stop>
        <input
          :ref="setCustomInputRef"
          type="text"
          class="ask-custom-input"
          :value="customText"
          :placeholder="customPlaceholder"
          @input="onCustomInput"
          @keydown="onCustomInputKeydown"
          @mousedown.stop
          @click.stop
        />
        <button
          type="button"
          class="ask-custom-submit-btn"
          :disabled="!customText?.trim()"
          @mousedown.prevent.stop="$emit('submitCustom')"
          @click.stop="$emit('submitCustom')"
        >
          {{ sendLabel }}
        </button>
      </div>

      <span v-else class="ask-body">
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
import { nextTick, ref, watch, type Component, type ComponentPublicInstance } from "vue";
import { Check, CircleHelp, ListChecks, PenLine, Sparkle } from "@lucide/vue";
import type { AskDisplayOption } from "@/types/chat";
import { PLAN_SWITCH_ACCEPT_LABEL, PLAN_SWITCH_DECLINE_LABEL } from "@/services/chat/askUserAnswer";

const props = withDefaults(
  defineProps<{
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
    customInputOpen?: boolean;
    customText?: string;
    customPlaceholder?: string;
    sendLabel?: string;
  }>(),
  {
    customInputOpen: false,
    customText: "",
    customPlaceholder: "",
    sendLabel: "Send",
  },
);

const emit = defineEmits<{
  hover: [index: number];
  select: [option: AskDisplayOption];
  confirm: [];
  "update:customText": [value: string];
  submitCustom: [];
  cancelCustom: [];
  navigateFromCustom: [delta: number];
}>();

const customInputEl = ref<HTMLInputElement | null>(null);

function setCustomInputRef(el: Element | ComponentPublicInstance | null) {
  customInputEl.value = el instanceof HTMLInputElement ? el : null;
}

function optionLeadingIcon(option: AskDisplayOption): Component | undefined {
  if (option.isSkip) return PenLine;
  if (!props.planSwitch) return undefined;
  if (option.label === PLAN_SWITCH_ACCEPT_LABEL) return ListChecks;
  if (option.label === PLAN_SWITCH_DECLINE_LABEL) return Sparkle;
  return undefined;
}

function onOptionMouseDown(event: MouseEvent, option: AskDisplayOption) {
  if (option.isSkip && props.customInputOpen) {
    return;
  }
  event.preventDefault();
  emit("select", option);
}

function onCustomInput(event: Event) {
  const target = event.target as HTMLInputElement | null;
  emit("update:customText", target?.value ?? "");
}

function onCustomInputKeydown(event: KeyboardEvent) {
  if (event.isComposing || event.keyCode === 229) {
    event.stopPropagation();
    return;
  }
  if (event.key === "Enter") {
    event.preventDefault();
    event.stopPropagation();
    if (props.customText?.trim()) {
      emit("submitCustom");
    }
    return;
  }
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    emit("cancelCustom");
    return;
  }
  if (event.key === "ArrowUp" || event.key === "ArrowDown") {
    event.preventDefault();
    event.stopPropagation();
    emit("navigateFromCustom", event.key === "ArrowDown" ? 1 : -1);
    return;
  }
  event.stopPropagation();
}

watch(
  [() => props.customInputOpen, () => props.questionIndex],
  async ([open]) => {
    if (!open) return;
    await nextTick();
    customInputEl.value?.focus({ preventScroll: true });
  },
  { immediate: true },
);
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
  margin: 0 0 4px;
  padding: 4px 0 6px;
  list-style: none;
  background: var(--peek-list-bg);
  border-bottom: 1px solid color-mix(in srgb, var(--peek-text) 7%, transparent);
}

.ask-user-list .picker-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 0 12px;
  min-height: 24px;
  font-size: 11px;
  color: var(--peek-muted);
  pointer-events: none;
  background: inherit;
}

.ask-user-list .picker-meta.question {
  min-height: 26px;
  align-items: flex-start;
  padding-top: 2px;
  padding-bottom: 2px;
  font-size: 12.5px;
  font-weight: 550;
  line-height: 1.45;
  color: var(--peek-text);
  white-space: normal;
  overflow-wrap: anywhere;
}

.picker-meta-label {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-weight: 600;
  color: var(--peek-accent);
}

.picker-meta-label.plan-switch {
  gap: 6px;
}

.picker-meta-icon {
  flex: none;
}

.ask-leading.plan-accept,
.ask-leading.custom-active {
  color: var(--peek-accent);
}

.picker-meta-progress {
  flex: none;
  font-variant-numeric: tabular-nums;
  padding: 1px 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--peek-text) 7%, transparent);
}

.command-item {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 2px 6px;
  padding: 7px 10px;
  min-height: var(--command-row-height);
  height: auto;
  border-radius: 8px;
  border: 1px solid transparent;
  cursor: pointer;
  transition:
    background 120ms ease,
    border-color 120ms ease;
}

.command-item.active {
  border-color: color-mix(in srgb, var(--peek-accent) 18%, transparent);
  background: color-mix(in srgb, var(--peek-accent) 9%, transparent);
}

.command-item.selected {
  border-color: color-mix(in srgb, var(--peek-accent) 22%, transparent);
  background: color-mix(in srgb, var(--peek-accent) 8%, transparent);
}

.command-item.selected.active {
  border-color: color-mix(in srgb, var(--peek-accent) 30%, transparent);
  background: color-mix(in srgb, var(--peek-accent) 14%, transparent);
}

.command-item.custom-input-active {
  border-color: color-mix(in srgb, var(--peek-accent) 32%, transparent);
  background: color-mix(in srgb, var(--peek-accent) 7%, transparent);
  cursor: default;
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
  gap: 2px;
  min-width: 0;
}

.ask-label {
  font-size: 13px;
  font-weight: 520;
  line-height: 18px;
  color: var(--peek-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ask-desc {
  font-size: 11px;
  line-height: 1.4;
  color: var(--peek-muted);
  white-space: normal;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.ask-custom-input-wrap {
  display: flex;
  flex: 1;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.ask-custom-input {
  flex: 1;
  min-width: 0;
  height: 28px;
  padding: 0 9px;
  border-radius: 6px;
  border: 1px solid color-mix(in srgb, var(--peek-text) 16%, transparent);
  background: var(--peek-input-bg, var(--peek-surface, #ffffff));
  color: var(--peek-text);
  font-size: 12.5px;
  line-height: 26px;
  outline: none;
  box-sizing: border-box;
  transition:
    border-color 120ms ease,
    box-shadow 120ms ease;
}

.ask-custom-input::placeholder {
  color: var(--peek-faint, var(--peek-muted));
}

.ask-custom-input:focus {
  border-color: var(--peek-accent);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--peek-accent) 14%, transparent);
}

.ask-custom-submit-btn {
  flex: none;
  height: 28px;
  padding: 0 11px;
  border-radius: 6px;
  border: none;
  background: var(--peek-accent);
  color: var(--peek-surface, #ffffff);
  font-size: 11.5px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 120ms ease;
}

.ask-custom-submit-btn:hover:not(:disabled) {
  opacity: 0.9;
}

.ask-custom-submit-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.confirm-item.disabled {
  opacity: 0.4;
}

.confirm-item .ask-label {
  color: var(--peek-accent);
}
</style>
