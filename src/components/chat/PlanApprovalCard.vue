<template>
  <section
    v-if="visible && (variant === 'proposal' ? title || summary : tasks.length || title)"
    class="build-plan-card"
    :class="variant"
  >
    <button type="button" class="build-plan-kicker" @click="$emit('previewPlan')">
      <span class="build-plan-label">{{ kicker }}</span>
      <FileText :size="13" :stroke-width="1.8" aria-hidden="true" />
      <span class="build-plan-heading">{{ heading }}</span>
    </button>

    <p v-if="variant === 'proposal' && summary" class="build-plan-summary">{{ summary }}</p>

    <button
      v-if="variant === 'build' && tasks.length"
      type="button"
      class="build-plan-toggle"
      :aria-expanded="expanded"
      @click="expanded = !expanded"
    >
      <ChevronRight class="build-plan-chevron" :class="{ open: expanded }" :size="13" />
      <span class="build-plan-current">{{ toggleLabel }}</span>
      <span v-if="!allCompleted" class="build-plan-progress">{{ progressLabel }}</span>
    </button>

    <ul v-if="variant === 'build' && tasks.length && expanded" class="build-plan-list">
      <li
        v-for="(task, index) in tasks"
        :key="`${index}-${task.content}`"
        class="build-plan-item"
        :class="statusClass(task.status)"
      >
        <span class="build-plan-marker" aria-hidden="true">
          <Check v-if="isCompleted(task.status)" :size="12" :stroke-width="2.5" />
          <span v-else class="build-plan-dot" />
        </span>
        <span class="build-plan-text">{{ task.content }}</span>
      </li>
    </ul>

    <div v-if="variant === 'proposal' && showActions" class="build-plan-footer">
      <button type="button" class="build-plan-link" @click="$emit('previewPlan')">
        <Eye :size="13" :stroke-width="1.8" aria-hidden="true" />
        {{ tr(settingStore.language, "viewPlan") }}
      </button>
      <button type="button" class="build-plan-action" :disabled="busy" @click="$emit('approve')">
        <Play :size="12" :stroke-width="2.2" aria-hidden="true" />
        {{ tr(settingStore.language, "planModeApprove") }}
      </button>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { Check, ChevronRight, Eye, FileText, Play } from "@lucide/vue";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import type { TaskItem } from "@/types/chat";

const props = withDefaults(
  defineProps<{
    title: string;
    summary?: string;
    tasks?: TaskItem[];
    visible?: boolean;
    busy?: boolean;
    variant?: "proposal" | "build";
    showActions?: boolean;
  }>(),
  {
    summary: "",
    tasks: () => [],
    visible: true,
    busy: false,
    variant: "proposal",
    showActions: true,
  },
);

defineEmits<{
  approve: [];
  reject: [];
  previewPlan: [];
}>();

const settingStore = useSettingStore();
const expanded = ref(props.variant === "proposal");

const kicker = computed(() =>
  tr(settingStore.language, props.variant === "build" ? "planBuild" : "createdPlan"),
);

const heading = computed(() => {
  const title = props.title.trim();
  if (!title) return tr(settingStore.language, "planProposalTitle");
  return title.startsWith("#") ? title : `# ${title}`;
});

const completedCount = computed(
  () => props.tasks.filter((task) => isCompleted(task.status)).length,
);

const allCompleted = computed(
  () => props.tasks.length > 0 && completedCount.value === props.tasks.length,
);

const currentTask = computed(
  () =>
    props.tasks.find((task) => isActive(task.status)) ??
    props.tasks.find((task) => !isCompleted(task.status)) ??
    props.tasks[0] ??
    null,
);

const progressLabel = computed(() => {
  const total = props.tasks.length;
  if (!total) return "";
  const currentIndex = props.tasks.findIndex((task) => isActive(task.status));
  const step = currentIndex >= 0 ? currentIndex + 1 : Math.max(completedCount.value, 0);
  return `${step}/${total}`;
});

const toggleLabel = computed(() => {
  if (allCompleted.value) {
    return tr(settingStore.language, "planTodosDone", {
      done: completedCount.value,
      total: props.tasks.length,
    });
  }
  return currentTask.value?.content ?? tasksLabel();
});

function tasksLabel() {
  return tr(settingStore.language, "planTodos", { count: props.tasks.length });
}

function isCompleted(status: string) {
  const value = status.toLowerCase();
  return value === "completed" || value === "done" || value === "complete";
}

function isActive(status: string) {
  const value = status.toLowerCase();
  return value === "in_progress" || value === "active" || value === "running";
}

function statusClass(status: string) {
  if (isCompleted(status)) return "completed";
  if (isActive(status)) return "active";
  return "pending";
}
</script>

<style scoped>
.build-plan-card {
  display: flex;
  flex-direction: column;
  width: 100%;
  margin: 8px 0 4px;
  padding: 8px 10px 8px;
  border: 1px solid color-mix(in srgb, var(--peek-border) 88%, transparent);
  border-radius: 12px;
  background: color-mix(in srgb, var(--peek-text) 3.5%, var(--peek-bg, var(--peek-surface)));
  box-sizing: border-box;
}

.build-plan-kicker {
  display: flex;
  align-items: center;
  gap: 7px;
  width: 100%;
  margin: 0;
  padding: 2px 4px 6px;
  border: 0;
  background: transparent;
  color: var(--peek-muted);
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.build-plan-kicker:hover {
  color: var(--peek-text);
}

.build-plan-label {
  font-size: 12.5px;
  font-weight: 650;
  color: var(--peek-text);
}

.build-plan-heading {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12.5px;
  font-weight: 500;
}

.build-plan-summary {
  margin: 0 4px 8px;
  color: var(--peek-muted);
  font-size: 12.5px;
  line-height: 1.45;
}

.build-plan-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  min-height: 28px;
  margin: 0;
  padding: 4px 6px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--peek-text);
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.build-plan-toggle:hover {
  background: color-mix(in srgb, var(--peek-text) 5%, transparent);
}

.build-plan-chevron {
  flex: none;
  color: var(--peek-muted);
  transition: transform 140ms ease;
}

.build-plan-chevron.open {
  transform: rotate(90deg);
}

.build-plan-current {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12.5px;
  line-height: 1.4;
}

.build-plan-progress {
  flex: none;
  color: var(--peek-muted);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}

.build-plan-list {
  list-style: none;
  margin: 2px 0 0;
  padding: 0;
  display: flex;
  flex-direction: column;
}

.build-plan-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  min-height: 28px;
  padding: 5px 8px;
  border-radius: 8px;
  color: var(--peek-text);
  font-size: 12.5px;
  line-height: 1.4;
}

.build-plan-item.active {
  background: color-mix(in srgb, var(--peek-text) 7%, transparent);
}

.build-plan-item.completed {
  color: var(--peek-muted);
}

.build-plan-item.completed .build-plan-text {
  text-decoration: line-through;
  text-decoration-color: color-mix(in srgb, var(--peek-muted) 50%, transparent);
}

.build-plan-marker {
  flex: none;
  width: 16px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--peek-muted);
}

.build-plan-dot {
  width: 13px;
  height: 13px;
  box-sizing: border-box;
  border-radius: 50%;
  border: 1.5px solid color-mix(in srgb, var(--peek-muted) 50%, transparent);
  background: transparent;
}

.build-plan-item.active .build-plan-dot {
  border-color: var(--peek-text);
  background: radial-gradient(circle at center, var(--peek-text) 3.2px, transparent 3.6px);
}

.build-plan-text {
  flex: 1;
  min-width: 0;
}

.build-plan-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 8px;
  padding: 2px 4px 0;
}

.build-plan-spacer {
  flex: 1;
}

.build-plan-link {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--peek-muted);
  font: inherit;
  font-size: 12.5px;
  font-weight: 500;
  cursor: pointer;
}

.build-plan-link:hover {
  color: var(--peek-text);
}

.build-plan-action {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  padding: 0 12px;
  border: 0;
  border-radius: 8px;
  background: color-mix(in srgb, var(--peek-accent) 18%, var(--peek-surface));
  color: var(--peek-text);
  font: inherit;
  font-size: 12.5px;
  font-weight: 600;
  cursor: pointer;
}

.build-plan-action:hover:not(:disabled) {
  background: color-mix(in srgb, var(--peek-accent) 28%, var(--peek-surface));
}

.build-plan-action:disabled {
  opacity: 0.55;
  cursor: default;
}
</style>
