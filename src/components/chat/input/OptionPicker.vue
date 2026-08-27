<template>
  <ul
    class="command-list option-picker-list peek-scrollbar"
    :class="{ compact }"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
  >
    <li
      v-for="(option, index) in options"
      :key="option.id"
      class="command-item option-picker-item"
      :class="{
        active: index === selectedIndex,
        current: option.id === selectedId,
      }"
      role="option"
      :aria-selected="index === selectedIndex"
      @mouseenter="$emit('hover', index)"
      @mousedown.prevent="$emit('select', option.id)"
    >
      <span v-if="option.icon" class="option-leading" aria-hidden="true">
        <component :is="option.icon" :size="13" class="option-icon" />
      </span>

      <span class="option-text">
        <span class="option-label">{{ option.label }}</span>
        <span v-if="option.description" class="option-desc">{{ option.description }}</span>
      </span>

      <TooltipProvider v-if="option.hint" :delay-duration="180">
        <Tooltip>
          <TooltipTrigger as-child>
            <button
              type="button"
              class="option-hint"
              :aria-label="option.hint"
              @mousedown.stop.prevent
              @click.stop
            >
              <CircleHelp :size="13" />
            </button>
          </TooltipTrigger>
          <TooltipContent
            side="top"
            :side-offset="6"
            class="max-w-72 text-left z-[80] whitespace-pre-wrap"
          >
            {{ option.hint }}
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>

      <Check v-if="option.id === selectedId" :size="13" class="option-check" aria-hidden="true" />
    </li>
  </ul>
</template>

<script setup lang="ts">
import type { Component } from "vue";
import { Check, CircleHelp } from "@lucide/vue";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";

export type OptionPickerItem = {
  id: string;
  label: string;
  description?: string;
  hint?: string;
  icon?: Component;
};

defineProps<{
  options: OptionPickerItem[];
  selectedId: string;
  selectedIndex: number;
  ariaLabel: string;
  /** Narrow single-line rows without icon background chips. */
  compact?: boolean;
}>();

defineEmits<{
  hover: [index: number];
  select: [id: string];
}>();
</script>

<style scoped>
.command-list {
  --command-row-height: 34px;
  --command-list-padding: 6px;
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

.command-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  min-height: var(--command-row-height);
  height: auto;
  cursor: default;
}

.command-item.active {
  background: var(--peek-list-active);
}

.option-picker-item.current:not(.active) {
  background: color-mix(in srgb, var(--peek-accent) 7%, transparent);
}

.option-leading {
  flex: none;
  width: 22px;
  height: 22px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: color-mix(in srgb, var(--peek-text) 5%, transparent);
  color: var(--peek-muted);
}

.option-picker-item.active .option-leading,
.option-picker-item.current .option-leading {
  background: color-mix(in srgb, var(--peek-accent) 14%, transparent);
  color: var(--peek-text);
}

.option-picker-list.compact .option-leading {
  width: 16px;
  height: 16px;
  border-radius: 0;
  background: transparent;
  color: var(--peek-muted);
}

.option-picker-list.compact .option-picker-item.active .option-leading,
.option-picker-list.compact .option-picker-item.current .option-leading {
  background: transparent;
  color: var(--peek-text);
}

.option-picker-list.compact .command-item {
  gap: 6px;
  padding: 0 10px;
}

.option-icon {
  opacity: 0.92;
}

.option-text {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
  padding: 6px 0;
}

.option-picker-list.compact .option-text {
  padding: 0;
}

.option-label {
  font-size: 13px;
  font-weight: 500;
  line-height: 16px;
  color: var(--peek-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.option-desc {
  font-size: 11px;
  line-height: 14px;
  color: var(--peek-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.option-check {
  flex: none;
  color: var(--peek-accent);
  opacity: 0.95;
}

.option-hint {
  display: inline-flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  margin: 0;
  padding: 0;
  border: 0;
  border-radius: 999px;
  background: transparent;
  color: var(--peek-faint);
  cursor: help;
}
.option-hint:hover {
  background: color-mix(in srgb, var(--peek-text) 8%, transparent);
  color: var(--peek-text);
}
</style>
