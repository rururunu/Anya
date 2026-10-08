<template>
  <section
    class="approval-request-panel peek-scrollbar"
    data-tauri-drag-region="false"
    :aria-label="ariaLabel"
  >
    <header class="approval-request-header">
      <span class="approval-request-icon" aria-hidden="true"><ShieldAlert :size="16" /></span>
      <div class="approval-request-heading">
        <strong>{{ header }}</strong>
        <p v-if="summary">{{ summary }}</p>
      </div>
    </header>
    <pre
      v-if="detail"
      class="approval-request-detail"
      :title="detail"
    ><code>{{ detail }}</code></pre>
    <div class="approval-request-actions" role="listbox" :aria-label="ariaLabel">
      <button
        v-for="(option, index) in options"
        :key="option.slug"
        type="button"
        role="option"
        :aria-selected="index === selectedIndex"
        class="approval-request-action"
        :class="{
          'is-selected': index === selectedIndex,
        }"
        :title="option.description"
        @mouseenter="$emit('hover', index)"
        @click="$emit('select', option.decision)"
      >
        <component :is="option.decision === 'deny' ? X : Check" :size="14" aria-hidden="true" />
        <span class="approval-request-action-copy">
          <span>{{ option.label }}</span>
          <small v-if="option.description">{{ option.description }}</small>
        </span>
      </button>
    </div>
  </section>
</template>

<script setup lang="ts">
import { Check, ShieldAlert, X } from "@lucide/vue";
import type { ToolApprovalDecision } from "@/types/chat";

defineProps<{
  header: string;
  summary?: string;
  detail?: string;
  ariaLabel: string;
  selectedIndex: number;
  options: Array<{
    slug: string;
    label: string;
    description: string;
    decision: ToolApprovalDecision;
  }>;
}>();
defineEmits<{ hover: [index: number]; select: [decision: ToolApprovalDecision] }>();
</script>

<style scoped>
.approval-request-panel {
  box-sizing: border-box;
  width: 100%;
  min-width: 0;
  padding: 14px;
  border: 1px solid
    var(--peek-composer-border, color-mix(in srgb, var(--peek-text) 16%, transparent));
  border-bottom: 0;
  border-radius: 16px 16px 0 0;
  background: var(--peek-interaction-fill, var(--peek-surface));
  color: var(--peek-text);
  overflow: auto;
}
.approval-request-header {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}
.approval-request-icon {
  display: grid;
  place-items: center;
  flex: none;
  width: 30px;
  height: 30px;
  border-radius: 9px;
  color: var(--peek-warning);
  background: color-mix(in srgb, var(--peek-warning) 12%, transparent);
}
.approval-request-heading {
  min-width: 0;
  flex: 1;
}
.approval-request-heading strong {
  display: block;
  font-size: 13px;
  line-height: 19px;
  font-weight: 650;
}
.approval-request-heading p {
  margin: 3px 0 0;
  color: var(--peek-muted);
  font-size: 12px;
  line-height: 18px;
  overflow-wrap: anywhere;
}
.approval-request-detail {
  margin: 12px 0 0;
  padding: 10px 12px;
  max-height: 104px;
  overflow: auto;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  border: 1px solid color-mix(in srgb, var(--peek-text) 7%, transparent);
  border-radius: 8px;
  background: color-mix(in srgb, var(--peek-text) 3%, var(--peek-surface));
  font:
    11.5px/1.6 ui-monospace,
    Consolas,
    monospace;
}
.approval-request-actions {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 10px;
}
.approval-request-action {
  display: inline-flex;
  align-items: center;
  justify-content: flex-start;
  text-align: left;
  gap: 10px;
  min-height: 44px;
  padding: 8px 10px;
  border: 1px solid transparent;
  border-radius: 8px;
  font: inherit;
  font-size: 12px;
  font-weight: 550;
  color: var(--peek-text);
  background: transparent;
  cursor: pointer;
}
.approval-request-action-copy {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.approval-request-action-copy small {
  color: var(--peek-muted);
  font-size: 11.5px;
  line-height: 16px;
}
.approval-request-action:hover {
  background: color-mix(in srgb, var(--peek-text) 4%, transparent);
}
.approval-request-action.is-selected {
  background: color-mix(in srgb, var(--peek-text) 5%, var(--peek-surface));
  border-color: color-mix(in srgb, var(--peek-text) 12%, transparent);
}
.approval-request-action:focus-visible {
  outline: 1px solid var(--peek-accent);
  outline-offset: -1px;
}
@media (max-width: 360px) {
  .approval-request-panel {
    padding: 12px;
  }
}
</style>
