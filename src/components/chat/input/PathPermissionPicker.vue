<template>
  <section
    class="permission-card path-permission-list peek-scrollbar"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
  >
    <header class="permission-card-header">
      <span class="permission-card-icon" aria-hidden="true">
        <Shield :size="16" :stroke-width="1.9" />
      </span>
      <div class="permission-card-copy">
        <strong>{{ header }}</strong>
        <span>{{ question }}</span>
      </div>
    </header>

    <div v-if="path" class="permission-path" :title="path">{{ path }}</div>

    <ul class="permission-options">
      <li
        v-for="(option, index) in options"
        :key="option.slug"
        class="permission-option"
        :class="{ active: index === selectedIndex, danger: option.decision === 'deny' }"
        role="option"
        :aria-selected="index === selectedIndex"
        @mouseenter="$emit('hover', index)"
        @mousedown.prevent="$emit('select', option.decision)"
      >
        <span v-if="option.icon" class="permission-option-icon" aria-hidden="true">
          <component :is="option.icon" :size="14" :stroke-width="2.25" />
        </span>
        <span class="permission-option-text">
          <span class="permission-option-label">{{ option.label }}</span>
          <span v-if="option.description" class="permission-option-desc">
            {{ option.description }}
          </span>
        </span>
      </li>
    </ul>
  </section>
</template>

<script setup lang="ts">
import type { Component } from "vue";
import { Shield } from "@lucide/vue";
import type { PathPermissionDecision } from "@/types/chat";

defineProps<{
  header: string;
  question: string;
  path?: string;
  options: Array<{
    slug: string;
    label: string;
    description: string;
    decision: PathPermissionDecision;
    icon?: Component;
  }>;
  selectedIndex: number;
  ariaLabel: string;
}>();

defineEmits<{
  hover: [index: number];
  select: [decision: PathPermissionDecision];
}>();
</script>

<style scoped>
.permission-card {
  --permission-row-height: 40px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex: none;
  margin: 0;
  padding: 10px 12px 8px;
  max-height: min(var(--interaction-picker-max-height, 48vh), 72vh);
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  /* Outer stroke is overridden by the fused composer shell to match .input-bar. */
  border: 1px solid color-mix(in srgb, var(--peek-text) 16%, transparent);
  border-bottom: 0;
  border-radius: 16px 16px 0 0;
  background: color-mix(in srgb, var(--peek-text) 7%, var(--peek-surface));
  box-shadow: none;
}

.permission-card-header {
  display: flex;
  gap: 10px;
  align-items: flex-start;
}

.permission-card-icon {
  flex: none;
  width: 28px;
  height: 28px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  background: color-mix(in srgb, var(--peek-warning) 16%, transparent);
  color: var(--peek-warning);
}

.permission-card-copy {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.permission-card-copy strong {
  font-size: 13px;
  font-weight: 650;
  line-height: 18px;
  color: var(--peek-text);
}

.permission-card-copy span {
  font-size: 12px;
  line-height: 16px;
  color: var(--peek-muted);
}

.permission-path {
  margin: 0;
  padding: 7px 10px;
  border: 1px solid color-mix(in srgb, var(--peek-text) 10%, transparent);
  border-radius: 8px;
  background: color-mix(in srgb, var(--peek-input-bg) 88%, var(--peek-surface));
  color: var(--peek-text);
  font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, Consolas, monospace);
  font-size: 11.5px;
  line-height: 1.45;
  white-space: normal;
  overflow-wrap: anywhere;
  word-break: break-all;
}

.permission-options {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.permission-option {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  min-height: var(--permission-row-height);
  margin: 0;
  padding: 8px 10px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--peek-text);
  cursor: pointer;
}

.permission-option:hover {
  background: color-mix(in srgb, var(--peek-text) 4%, transparent);
}

.permission-option.active {
  border-color: color-mix(in srgb, var(--peek-accent) 28%, transparent);
  background: color-mix(in srgb, var(--peek-accent) 10%, var(--peek-surface));
}

.permission-option.danger.active {
  border-color: color-mix(in srgb, var(--peek-danger) 28%, transparent);
  background: color-mix(in srgb, var(--peek-danger) 8%, var(--peek-surface));
}

.permission-option-icon {
  flex: none;
  width: 16px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--peek-muted);
}

.permission-option.active .permission-option-icon {
  color: var(--peek-accent);
}

.permission-option.danger.active .permission-option-icon {
  color: var(--peek-danger);
}

.permission-option-text {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.permission-option-label {
  font-size: 13px;
  font-weight: 600;
  line-height: 18px;
  color: var(--peek-text);
}

.permission-option-desc {
  font-size: 11.5px;
  line-height: 15px;
  color: var(--peek-muted);
}
</style>
