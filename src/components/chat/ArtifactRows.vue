<template>
  <div v-if="rows.length" class="artifact-rows">
    <div v-for="row in rows" :key="row.key" class="artifact-row">
      <component
        :is="row.icon"
        class="artifact-icon"
        :size="14"
        :stroke-width="1.75"
        aria-hidden="true"
      />
      <button type="button" class="artifact-main" :title="row.title" @click="row.onOpen">
        <span class="artifact-label">{{ row.label }}</span>
        <span v-if="row.stats" class="artifact-stats">
          <span v-if="row.stats.added" class="added">+{{ row.stats.added }}</span>
          <span v-if="row.stats.removed" class="removed">-{{ row.stats.removed }}</span>
        </span>
      </button>
      <button
        v-if="row.actionLabel"
        type="button"
        class="artifact-action"
        @click.stop="row.onAction?.()"
      >
        {{ row.actionLabel }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Component } from "vue";

export interface ArtifactRow {
  key: string;
  icon: Component;
  label: string;
  title?: string;
  stats?: { added?: number; removed?: number };
  actionLabel?: string;
  onOpen: () => void;
  onAction?: () => void;
}

defineProps<{
  rows: ArtifactRow[];
}>();
</script>

<style scoped>
.artifact-rows {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
}

.artifact-row {
  display: flex;
  align-items: center;
  gap: 6px;
  min-height: 24px;
  padding: 0 2px;
}

.artifact-icon {
  flex: none;
  color: var(--peek-faint);
}

.artifact-main {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.artifact-main:hover .artifact-label {
  color: var(--peek-text);
}

.artifact-label {
  min-width: 0;
  overflow: hidden;
  color: var(--peek-muted);
  font-size: 12px;
  font-weight: 500;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.artifact-stats {
  flex: none;
  display: inline-flex;
  gap: 5px;
  font: 10px/1.2 var(--font-mono);
  font-variant-numeric: tabular-nums;
}

.added {
  color: #22c55e;
}

.removed {
  color: var(--destructive);
}

.artifact-action {
  flex: none;
  min-height: 22px;
  padding: 0 7px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--peek-faint);
  font-size: 11px;
  cursor: pointer;
}

.artifact-action:hover {
  color: var(--peek-text);
  background: var(--peek-hover-bg);
}
</style>
