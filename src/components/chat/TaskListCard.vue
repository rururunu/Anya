<template>
  <section
    v-if="tasks.length"
    class="task-list-card"
    :class="{ open: expanded, embedded }"
    :aria-label="title"
  >
    <button
      type="button"
      class="task-list-toggle"
      :aria-expanded="expanded"
      @click="expanded = !expanded"
    >
      <ChevronRight
        class="task-list-chevron"
        :class="{ open: expanded }"
        :size="13"
        aria-hidden="true"
      />
      <ListTodo :size="13" aria-hidden="true" />
      <span class="task-list-title">{{ title }}</span>
      <span class="task-list-progress">{{ progressLabel }}</span>
      <span v-if="!expanded && activeTask" class="task-list-current">
        {{ activeTask.content }}
      </span>
    </button>

    <ul v-if="expanded" class="task-list-items peek-scrollbar">
      <li
        v-for="(task, index) in tasks"
        :key="`${index}-${task.content}`"
        class="task-list-item"
        :class="statusClass(task.status)"
        :style="{ paddingLeft: `${10 + Math.min(task.level ?? 0, 6) * 12}px` }"
      >
        <span class="task-marker" aria-hidden="true">
          <Check v-if="isCompleted(task.status)" :size="12" :stroke-width="2.5" />
          <Minus v-else-if="isCancelled(task.status)" :size="12" :stroke-width="2.5" />
          <span v-else-if="isActive(task.status)" class="task-dot active" />
          <span v-else class="task-dot" />
        </span>
        <span class="task-content">
          {{ task.content }}
          <small v-if="isActive(task.status) && task.activeForm">{{ task.activeForm }}</small>
        </span>
      </li>
    </ul>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Check, ChevronRight, ListTodo, Minus } from "@lucide/vue";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import type { TaskItem } from "@/types/chat";
import { textIncludesQuery } from "@/services/chat/conversationFind";
import { useExpandForFind } from "@/composables/chat/useConversationFind";

const props = withDefaults(
  defineProps<{
    tasks: TaskItem[];
    /** When true, render in the chat transcript (expanded by default). */
    embedded?: boolean;
  }>(),
  {
    embedded: false,
  },
);

const settingStore = useSettingStore();
const expanded = ref(props.embedded);

const title = computed(() => tr(settingStore.language, "taskListTitle"));

const completedCount = computed(
  () => props.tasks.filter((task) => isCompleted(task.status)).length,
);

const progressLabel = computed(() =>
  tr(settingStore.language, "taskListProgress", {
    done: String(completedCount.value),
    total: String(props.tasks.length),
  }),
);

const activeTask = computed(
  () =>
    props.tasks.find((task) => isActive(task.status)) ??
    props.tasks.find((task) => !isCompleted(task.status) && !isCancelled(task.status)) ??
    null,
);

watch(
  () => props.tasks.map((task) => `${task.status}:${task.content}`).join("|"),
  () => {
    if (props.embedded) {
      // Keep the conversation snapshot readable as the agent updates tasks.
      expanded.value = true;
      return;
    }
    expanded.value = false;
  },
);

useExpandForFind(
  (query) =>
    props.tasks.some(
      (task) => textIncludesQuery(task.content, query) || textIncludesQuery(task.activeForm, query),
    ),
  () => {
    expanded.value = true;
  },
);

function normalizeStatus(status: string): string {
  return status.trim().toLowerCase().replace(/_/g, "-");
}

function isCompleted(status: string): boolean {
  const value = normalizeStatus(status);
  return value === "completed" || value === "done" || value === "complete";
}

function isCancelled(status: string): boolean {
  const value = normalizeStatus(status);
  return value === "cancelled" || value === "canceled";
}

function isActive(status: string): boolean {
  const value = normalizeStatus(status);
  return (
    value === "in-progress" || value === "inprogress" || value === "active" || value === "running"
  );
}

function statusClass(status: string): string {
  if (isCompleted(status)) return "is-done";
  if (isCancelled(status)) return "is-cancelled";
  if (isActive(status)) return "is-active";
  return "is-pending";
}
</script>

<style scoped>
.task-list-card {
  display: flex;
  flex-direction: column;
  width: 100%;
  min-width: 0;
  border: 1px solid color-mix(in srgb, var(--peek-border) 88%, transparent);
  border-radius: 10px;
  background: var(--peek-surface, #fff);
  box-shadow: 0 1px 0 color-mix(in srgb, #000 4%, transparent);
  overflow: hidden;
}

.task-list-card.embedded {
  margin: 4px 0 8px;
  background: color-mix(in srgb, var(--peek-input-bg) 92%, #0b0d10);
  box-shadow: none;
}

.task-list-toggle {
  appearance: none;
  width: 100%;
  min-height: 34px;
  display: flex;
  align-items: center;
  gap: 7px;
  margin: 0;
  padding: 6px 10px;
  border: 0;
  background: transparent;
  color: var(--peek-text);
  cursor: pointer;
  text-align: left;
  font: inherit;
}

.task-list-toggle:hover {
  background: color-mix(in srgb, var(--peek-text) 3.5%, transparent);
}

.task-list-chevron {
  flex: none;
  color: var(--peek-muted);
  transition: transform 140ms ease;
}

.task-list-chevron.open {
  transform: rotate(90deg);
}

.task-list-title {
  flex: none;
  font-size: 12px;
  font-weight: 600;
}

.task-list-progress {
  flex: none;
  color: var(--peek-muted);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}

.task-list-current {
  flex: 1;
  min-width: 0;
  margin-left: 4px;
  overflow: hidden;
  color: var(--peek-muted);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.task-list-current::before {
  content: "·";
  margin-right: 6px;
  color: color-mix(in srgb, var(--peek-muted) 70%, transparent);
}

.task-list-items {
  list-style: none;
  margin: 0;
  padding: 2px 6px 8px;
  display: grid;
  gap: 2px;
  max-height: min(168px, 28vh);
  overflow-x: hidden;
  overflow-y: auto;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 70%, transparent);
  background: var(--peek-surface, #fff);
}

.task-list-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  min-width: 0;
  padding: 5px 6px;
  border-radius: 6px;
  color: var(--peek-text);
  font-size: 12px;
  line-height: 1.4;
}

.task-list-item.is-active {
  background: color-mix(in srgb, var(--peek-accent) 8%, transparent);
}

.task-list-item.is-done {
  color: color-mix(in srgb, var(--peek-text) 48%, var(--peek-muted));
  text-decoration: line-through;
  text-decoration-color: color-mix(in srgb, var(--peek-muted) 50%, transparent);
}

.task-list-item.is-cancelled {
  color: var(--peek-muted);
  text-decoration: line-through;
}

.task-list-item.is-active .task-content {
  font-weight: 560;
}

.task-marker {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  width: 14px;
  height: 16px;
  color: var(--peek-muted);
}

.task-list-item.is-done .task-marker {
  color: color-mix(in srgb, var(--peek-accent) 70%, #2f9e44);
}

.task-dot {
  width: 7px;
  height: 7px;
  border-radius: 999px;
  border: 1.5px solid color-mix(in srgb, var(--peek-muted) 65%, transparent);
  background: transparent;
}

.task-dot.active {
  border-color: var(--peek-accent);
  background: var(--peek-accent);
}

.task-content {
  min-width: 0;
  display: grid;
  gap: 1px;
}

.task-content small {
  color: var(--peek-muted);
  font-size: 11px;
  font-weight: 400;
}
</style>
