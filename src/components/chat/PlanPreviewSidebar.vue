<template>
  <aside
    class="plan-preview-sidebar"
    :class="{ embedded }"
    :style="width ? { width: `${width}px` } : undefined"
    data-tauri-drag-region="false"
    data-no-drag
    :aria-label="tr(settingStore.language, 'planProposalTitle')"
  >
    <header class="plan-sidebar-header">
      <div class="plan-sidebar-title">
        <FileText :size="15" :stroke-width="1.8" aria-hidden="true" />
        <strong>{{ tr(settingStore.language, "planProposalTitle") }}</strong>
        <span v-if="planPath" class="plan-file-badge" :title="planPath">{{ planPath }}</span>
      </div>
      <div class="plan-sidebar-actions">
        <button
          v-if="planContent"
          type="button"
          class="small-icon-button"
          :title="tr(settingStore.language, copied ? 'copied' : 'copy')"
          :aria-label="tr(settingStore.language, copied ? 'copied' : 'copy')"
          @click="handleCopy"
        >
          <Check v-if="copied" :size="13" />
          <Copy v-else :size="13" />
        </button>
      </div>
    </header>

    <div class="plan-sidebar-content peek-scrollbar">
      <div v-if="planContent" class="plan-markdown-wrapper">
        <Markdown :content="planContent" />
      </div>
      <div v-else class="plan-sidebar-empty">
        <FileQuestion :size="32" :stroke-width="1.4" class="plan-empty-icon" />
        <p class="plan-empty-text">
          {{ tr(settingStore.language, "planNoProposalYet") }}
        </p>
      </div>

      <div v-if="showTodos" class="plan-doc-todos">
        <div class="plan-doc-todos-header">
          <span>{{ tr(settingStore.language, "planTodos", { count: tasks.length }) }}</span>
          <button
            v-if="canEditTodos"
            type="button"
            class="plan-add-todo-btn"
            :class="{ active: addingTodo }"
            :title="tr(settingStore.language, 'planAddTodo')"
            :aria-label="tr(settingStore.language, 'planAddTodo')"
            @click="toggleAddTodo"
          >
            <Plus :size="14" :stroke-width="2" aria-hidden="true" />
          </button>
        </div>
        <div
          v-for="(task, index) in tasks"
          :key="`${index}-${task.content}`"
          class="plan-task-row"
          :class="taskStatusClass(task.status)"
        >
          <span class="plan-task-bullet" aria-hidden="true">
            <Check v-if="isTaskDone(task.status)" :size="11" :stroke-width="2.5" />
            <span v-else class="plan-task-dot" />
          </span>
          <span class="plan-task-text">{{ task.content }}</span>
          <button
            v-if="canEditTodos"
            type="button"
            class="plan-remove-todo-btn"
            :title="tr(settingStore.language, 'planRemoveTodo')"
            :aria-label="tr(settingStore.language, 'planRemoveTodo')"
            @click="removeTodo(index)"
          >
            <X :size="12" :stroke-width="2" aria-hidden="true" />
          </button>
        </div>
        <form
          v-if="addingTodo"
          class="plan-task-row plan-add-todo-row"
          @submit.prevent="commitAddTodo"
        >
          <span class="plan-task-bullet" aria-hidden="true">
            <span class="plan-task-dot" />
          </span>
          <input
            ref="todoDraftInput"
            v-model="todoDraft"
            class="plan-add-todo-input"
            :placeholder="tr(settingStore.language, 'planAddTodoPlaceholder')"
            maxlength="240"
            @keydown.escape.prevent="cancelAddTodo"
          />
        </form>
      </div>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { Check, Copy, FileQuestion, FileText, Plus, X } from "@lucide/vue";
import Markdown from "@/components/chat/Markdown.vue";
import { copyText } from "@/services/clipboard";
import { tr } from "@/services/i18n";
import { useChatStore } from "@/stores/chat";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage, TaskItem } from "@/types/chat";

const props = withDefaults(
  defineProps<{
    sessionId?: string;
    width?: number;
    embedded?: boolean;
    messages?: ChatMessage[];
  }>(),
  {
    sessionId: "",
    width: 0,
    embedded: false,
    messages: () => [],
  },
);

const settingStore = useSettingStore();
const chatStore = useChatStore();

const copied = ref(false);
const addingTodo = ref(false);
const todoDraft = ref("");
const todoDraftInput = ref<HTMLInputElement | null>(null);

const livePlan = computed(() => (props.sessionId ? chatStore.sessionPlans[props.sessionId] : null));

const historicalPlan = computed<{ path: string; content: string } | null>(() => {
  if (livePlan.value?.content) return livePlan.value;
  // Look backwards for save_plan tool activity in messages
  for (let i = props.messages.length - 1; i >= 0; i -= 1) {
    const msg = props.messages[i];
    if (!msg?.toolActivities?.length) continue;
    for (let j = msg.toolActivities.length - 1; j >= 0; j -= 1) {
      const act = msg.toolActivities[j];
      if (act?.toolName === "save_plan" && act.arguments?.content) {
        let extractedPath = String(act.arguments.path ?? "");
        if (!extractedPath && typeof act.result === "string") {
          const m = act.result.match(/Saved plan proposal to `([^`]+)`/);
          if (m?.[1]) extractedPath = m[1];
        }
        return {
          path: extractedPath || ".anya/plan.md",
          content: String(act.arguments.content ?? ""),
        };
      }
    }
  }
  return null;
});

const planPath = computed(
  () => livePlan.value?.path ?? historicalPlan.value?.path ?? ".anya/plan.md",
);
const planContent = computed(() => livePlan.value?.content ?? historicalPlan.value?.content ?? "");

const tasks = computed<TaskItem[]>(() => {
  if (props.sessionId && props.sessionId in chatStore.sessionTasks) {
    return chatStore.sessionTasks[props.sessionId] ?? [];
  }
  for (let i = props.messages.length - 1; i >= 0; i -= 1) {
    const msg = props.messages[i];
    if (!msg?.toolActivities?.length) continue;
    for (let j = msg.toolActivities.length - 1; j >= 0; j -= 1) {
      const act = msg.toolActivities[j];
      if (
        (act?.toolName === "update_tasks" || act?.toolName === "todo_write") &&
        Array.isArray(act.arguments?.tasks)
      ) {
        return act.arguments.tasks as TaskItem[];
      }
    }
  }
  return [];
});

const canEditTodos = computed(() => Boolean(props.sessionId));
const showTodos = computed(
  () =>
    tasks.value.length > 0 || addingTodo.value || Boolean(planContent.value && canEditTodos.value),
);

function toggleAddTodo() {
  addingTodo.value = !addingTodo.value;
  if (!addingTodo.value) {
    todoDraft.value = "";
    return;
  }
  void nextTick(() => todoDraftInput.value?.focus());
}

function cancelAddTodo() {
  addingTodo.value = false;
  todoDraft.value = "";
}

function writeTodos(next: TaskItem[]) {
  if (!props.sessionId) return;
  chatStore.setSessionTasks(props.sessionId, next);
}

function commitAddTodo() {
  const content = todoDraft.value.trim();
  if (!content || !props.sessionId) return;
  writeTodos([...tasks.value, { content, status: "pending" }]);
  todoDraft.value = "";
  void nextTick(() => todoDraftInput.value?.focus());
}

function removeTodo(index: number) {
  if (!props.sessionId || index < 0 || index >= tasks.value.length) return;
  writeTodos(tasks.value.filter((_, itemIndex) => itemIndex !== index));
}

function isTaskDone(status: string) {
  const value = status.toLowerCase();
  return value === "completed" || value === "done" || value === "complete";
}

function taskStatusClass(status: string) {
  if (isTaskDone(status)) return "completed";
  const value = status.toLowerCase();
  if (value === "in_progress" || value === "active" || value === "running") return "active";
  return "pending";
}

async function handleCopy() {
  if (!planContent.value) return;
  await copyText(planContent.value);
  copied.value = true;
  setTimeout(() => {
    copied.value = false;
  }, 1800);
}
</script>

<style scoped>
.plan-preview-sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background: var(--peek-surface);
  color: var(--peek-text);
  overflow: hidden;
}

.plan-preview-sidebar.embedded {
  width: 100%;
}

.plan-sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--peek-border);
  flex: none;
}

.plan-sidebar-title {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.plan-sidebar-title strong {
  font-size: 13px;
  font-weight: 650;
  white-space: nowrap;
}

.plan-file-badge {
  font-size: 11px;
  padding: 1px 6px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--peek-accent) 12%, var(--peek-surface));
  color: var(--peek-accent);
  white-space: nowrap;
  text-overflow: ellipsis;
  overflow: hidden;
  max-width: 160px;
}

.plan-sidebar-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: none;
}

.plan-sidebar-content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 14px 16px 20px;
}

.plan-markdown-wrapper {
  font-size: 13px;
  line-height: 1.6;
}

.plan-doc-todos {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-top: 28px;
  padding-top: 16px;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 80%, transparent);
}

.plan-doc-todos-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 8px;
  font-size: 13px;
  font-weight: 600;
  color: var(--peek-text);
}

.plan-add-todo-btn {
  display: inline-grid;
  place-items: center;
  flex: none;
  width: 24px;
  height: 24px;
  padding: 0;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}

.plan-add-todo-btn:hover,
.plan-add-todo-btn.active,
.plan-add-todo-btn:focus-visible {
  color: var(--peek-text);
  background: var(--peek-hover-bg);
  outline: none;
}

.plan-add-todo-input {
  flex: 1;
  min-width: 0;
  margin: 0;
  padding: 0;
  border: 0;
  outline: none;
  background: transparent;
  color: var(--peek-text);
  font: inherit;
  line-height: inherit;
}

.plan-add-todo-input::placeholder {
  color: color-mix(in srgb, var(--peek-muted) 80%, transparent);
}

.plan-add-todo-row {
  padding-right: 2px;
}

.plan-task-row {
  position: relative;
  display: flex;
  align-items: flex-start;
  gap: 8px;
  font-size: 13px;
  line-height: 18px;
  padding: 6px 22px 6px 2px;
  border-radius: 6px;
  color: var(--peek-text);
}

.plan-task-row.active {
  color: var(--peek-text);
}

.plan-task-row.completed {
  color: var(--peek-muted);
}

.plan-task-row.completed .plan-task-text {
  text-decoration: line-through;
  text-decoration-color: color-mix(in srgb, var(--peek-muted) 55%, transparent);
}

.plan-task-bullet {
  flex: none;
  width: 16px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--peek-muted);
}

.plan-task-dot {
  width: 13px;
  height: 13px;
  box-sizing: border-box;
  border-radius: 50%;
  border: 1.5px solid color-mix(in srgb, var(--peek-muted) 55%, transparent);
  background: transparent;
}

.plan-task-row.active .plan-task-dot {
  border-color: var(--peek-accent);
  background: color-mix(in srgb, var(--peek-accent) 18%, transparent);
}

.plan-task-text {
  flex: 1;
  min-width: 0;
}

.plan-remove-todo-btn {
  position: absolute;
  top: 3px;
  right: 0;
  display: inline-grid;
  place-items: center;
  flex: none;
  width: 20px;
  height: 20px;
  padding: 0;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
  opacity: 0;
}

.plan-task-row:hover .plan-remove-todo-btn,
.plan-task-row:focus-within .plan-remove-todo-btn {
  opacity: 1;
}

.plan-remove-todo-btn:hover,
.plan-remove-todo-btn:focus-visible {
  color: var(--peek-text);
  background: var(--peek-hover-bg);
  outline: none;
  opacity: 1;
}

.plan-sidebar-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: 48px 16px;
  color: var(--peek-muted);
}

.plan-empty-icon {
  margin-bottom: 12px;
  opacity: 0.6;
}

.plan-empty-text {
  font-size: 12.5px;
  line-height: 1.5;
  max-width: 240px;
  margin: 0;
}
</style>
