<template>
  <aside
    class="subagent-sidebar"
    :class="{ embedded }"
    data-tauri-drag-region="false"
    :aria-label="sidebarTitle"
  >
    <header v-if="!embedded" class="subagent-sidebar-header">
      <SubagentIcon :size="15" />
      <strong>{{ sidebarTitle }}</strong>
      <button type="button" class="close-button" @click="emit('close')"><X :size="14" /></button>
    </header>

    <template v-if="agentEntries.length">
      <nav
        ref="tabsRef"
        class="subagent-tabs peek-card-tabs"
        role="tablist"
        :aria-label="sidebarTitle"
        @wheel="scrollTabs"
      >
        <div
          v-for="entry in agentEntries"
          :key="entry.id"
          class="subagent-tab-shell peek-card-tab"
          :class="{ active: entry.id === activeEntry?.id }"
        >
          <button
            :id="`subagent-tab-${entry.id}`"
            type="button"
            role="tab"
            class="subagent-tab"
            :aria-selected="entry.id === activeEntry?.id"
            :aria-controls="`subagent-panel-${entry.id}`"
            :title="entry.title"
            @click="activeId = entry.id"
          >
            <SubagentIcon :status="entry.status" :size="15" />
            <span>{{ entry.title }}</span>
          </button>
          <button
            type="button"
            class="subagent-tab-close"
            :aria-label="closeTabLabel"
            :title="closeTabLabel"
            @click.stop="emit('closeEntry', entry.id)"
          >
            <X :size="11" />
          </button>
        </div>
      </nav>

      <section
        v-if="activeEntry"
        :id="`subagent-panel-${activeEntry.id}`"
        class="subagent-panel peek-scrollbar"
        role="tabpanel"
        :aria-labelledby="`subagent-tab-${activeEntry.id}`"
      >
        <header class="subagent-panel-header">
          <div class="subagent-identity">
            <SubagentIcon :status="activeEntry.status" :size="14" />
            <strong :title="activeEntry.title">{{ activeEntry.title }}</strong>
            <span class="agent-status" :class="activeEntry.status">
              {{ statusLabel(activeEntry.status) }}
            </span>
          </div>
          <div v-if="activeEntry.model" class="subagent-model-row">
            <span>{{ modelLabel }}</span>
            <code>{{ activeEntry.model }}</code>
          </div>
        </header>

        <div class="subagent-work">
          <section
            class="subagent-task-details"
            :class="{ open: isTaskDetailsOpen(activeEntry.id) }"
          >
            <button
              type="button"
              class="subagent-task-toggle"
              :aria-expanded="isTaskDetailsOpen(activeEntry.id)"
              :aria-controls="`subagent-task-${activeEntry.id}`"
              data-tauri-drag-region="false"
              @click="toggleTaskDetails(activeEntry.id)"
            >
              <ChevronRight :size="12" />
              <span>{{ taskDetailsLabel }}</span>
            </button>
            <div
              v-show="isTaskDetailsOpen(activeEntry.id)"
              :id="`subagent-task-${activeEntry.id}`"
              class="subagent-task-body"
            >
              <Markdown :content="activeEntry.task" />
              <AgentWorkDetails
                :message="activeEntry.message"
                :language="settingStore.language"
                :show-reasoning="settingStore.showReasoning"
                :display-mode="settingStore.agentWorkDisplay"
              />
            </div>
          </section>
          <Markdown v-if="activeEntry.message.content" :content="activeEntry.message.content" />
          <p
            v-if="!activeEntry.children.length && !activeEntry.message.content"
            class="agent-waiting"
          >
            {{ waitingLabel }}
          </p>
        </div>
      </section>
    </template>

    <p v-else class="subagent-empty">{{ emptyLabel }}</p>
  </aside>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ChevronRight, X } from "@lucide/vue";
import AgentWorkDetails from "@/components/chat/AgentWorkDetails.vue";
import Markdown from "@/components/chat/Markdown.vue";
import SubagentIcon from "@/components/chat/SubagentIcon.vue";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import type { ChatMessage, MessageStatus, ToolActivity } from "@/types/chat";

type AgentStatus = "running" | "done" | "error";
type AgentEntry = {
  id: string;
  title: string;
  task: string;
  status: AgentStatus;
  model?: string;
  children: ToolActivity[];
  message: ChatMessage;
};

const props = withDefaults(
  defineProps<{
    activities: ToolActivity[];
    allActivities: ToolActivity[];
    openedEntryIds?: string[];
    selectedEntryId?: string;
    embedded?: boolean;
  }>(),
  { openedEntryIds: () => [], selectedEntryId: "", embedded: false },
);
const emit = defineEmits<{ close: []; closeEntry: [entryId: string] }>();
const settingStore = useSettingStore();
const activeId = ref("");
const tabsRef = ref<HTMLElement | null>(null);
const openTaskDetails = ref(new Set<string>());

const sidebarTitle = computed(() => tr(settingStore.language, "subagent.title"));
const emptyLabel = computed(() => tr(settingStore.language, "subagent.empty"));
const waitingLabel = computed(() => tr(settingStore.language, "subagent.waiting"));
const closeTabLabel = computed(() => tr(settingStore.language, "subagent.closeTab"));
const taskDetailsLabel = computed(() => tr(settingStore.language, "subagent.taskDetails"));
const modelLabel = computed(() => tr(settingStore.language, "subagent.model"));

const agentEntries = computed<AgentEntry[]>(() => {
  const entries: AgentEntry[] = [];
  for (const parent of props.activities) {
    const labels = taskLabels(parent);
    const groups = childGroups(parent.id);
    const models = taskModels(parent);
    const count = Math.max(1, labels.length, groups.length);
    for (let index = 0; index < count; index += 1) {
      const id = `${parent.id}:${index}`;
      const children = groups[index] ?? [];
      const status = normalizeStatus(parent.status);
      const task = labels[index] ?? labels[0] ?? fallbackTask(parent);
      const title = shortTaskTitle(task, entries.length);
      const model = models[index] ?? models[0];
      entries.push({
        id,
        title,
        task,
        status,
        model,
        children,
        message: makeAgentMessage(id, index, parent, children, status),
      });
    }
  }
  const opened = new Set(props.openedEntryIds);
  return entries.filter((entry) => opened.has(entry.id));
});

const activeEntry = computed(
  () => agentEntries.value.find((entry) => entry.id === activeId.value) ?? agentEntries.value[0],
);

watch(
  () => agentEntries.value.map((entry) => entry.id).join("|"),
  () => {
    if (!agentEntries.value.some((entry) => entry.id === activeId.value)) {
      activeId.value = agentEntries.value[0]?.id ?? "";
    }
  },
  { immediate: true },
);

watch(
  () => props.selectedEntryId,
  (entryId) => {
    if (entryId && agentEntries.value.some((entry) => entry.id === entryId)) {
      activeId.value = entryId;
    }
  },
  { immediate: true },
);

function makeAgentMessage(
  id: string,
  taskIndex: number,
  parent: ToolActivity,
  children: ToolActivity[],
  status: AgentStatus,
): ChatMessage {
  const childIds = new Set(children.map((activity) => activity.id));
  const scopedActivities = props.allActivities
    .filter(
      (activity) =>
        childIds.has(activity.id) ||
        children.some((child) => activity.parentActivityId === child.id),
    )
    .map((activity) =>
      childIds.has(activity.id) ? { ...activity, parentActivityId: undefined } : activity,
    );
  return {
    id: `subagent-${id}`,
    sessionId: "subagent",
    role: "assistant",
    content: status === "running" ? "" : completionFor(parent, taskIndex),
    toolActivities: scopedActivities,
    status: messageStatus(status),
    timestamp: 0,
  };
}

function completionFor(parent: ToolActivity, taskIndex: number) {
  const result = parent.result?.trim() ?? "";
  if (parent.toolName !== "run_parallel_subagents" || !result) return result;
  const sections = result.split(/^### Task \d+\s*$/gm);
  return sections[taskIndex + 1]?.trim() ?? "";
}

function taskLabels(activity: ToolActivity) {
  const args = activity.arguments ?? {};
  if (Array.isArray(args.tasks)) {
    return args.tasks
      .map((value) =>
        typeof value === "object" && value != null
          ? String((value as Record<string, unknown>).prompt ?? "").trim()
          : "",
      )
      .filter(Boolean);
  }
  for (const value of [args.description, args.task, args.prompt]) {
    if (typeof value === "string" && value.trim()) return [value.trim()];
  }
  return activity.detail?.trim() ? [activity.detail.trim()] : [];
}

function taskModels(activity: ToolActivity) {
  const args = activity.arguments ?? {};
  if (Array.isArray(args.tasks)) {
    return args.tasks.map((value) =>
      typeof value === "object" && value != null
        ? String((value as Record<string, unknown>).model ?? "").trim() || undefined
        : undefined,
    );
  }
  return typeof args.model === "string" && args.model.trim() ? [args.model.trim()] : [];
}

function fallbackTask(activity: ToolActivity) {
  return activity.title || tr(settingStore.language, "subagent.executeTask");
}

function shortTaskTitle(prompt: string | undefined | null, index: number) {
  const lines = (prompt ?? "")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  const heading = lines.find((line) => /^#{1,6}\s+/.test(line));
  const source = heading ?? lines[0] ?? "";
  const cleaned = source
    .replace(/^#{1,6}\s+/, "")
    .replace(/^(?:\u4efb\u52a1|task|assignment)\s*[:\uff1a-]\s*/i, "")
    .replace(/[`*_~]/g, "")
    .trim();
  const prefix = `${tr(settingStore.language, "subagent.title")} ${index + 1}`;
  const title = cleaned ? `${prefix} \u00b7 ${cleaned}` : prefix;
  return title.length > 72 ? `${title.slice(0, 71)}...` : title;
}

function scrollTabs(event: WheelEvent) {
  const tabs = tabsRef.value;
  if (!tabs || tabs.scrollWidth <= tabs.clientWidth) return;
  event.preventDefault();
  tabs.scrollLeft += Math.abs(event.deltaY) >= Math.abs(event.deltaX) ? event.deltaY : event.deltaX;
}

function isTaskDetailsOpen(entryId: string) {
  return openTaskDetails.value.has(entryId);
}

function toggleTaskDetails(entryId: string) {
  const next = new Set(openTaskDetails.value);
  if (next.has(entryId)) next.delete(entryId);
  else next.add(entryId);
  openTaskDetails.value = next;
}

function childGroups(parentActivityId: string) {
  const groups = new Map<string, ToolActivity[]>();
  for (const activity of props.allActivities) {
    if (activity.parentActivityId !== parentActivityId) continue;
    const key = activity.subagentId ?? "default";
    const group = groups.get(key) ?? [];
    group.push(activity);
    groups.set(key, group);
  }
  return [...groups.values()];
}

function normalizeStatus(status: ToolActivity["status"]): AgentStatus {
  return status === "error" ? "error" : status === "running" ? "running" : "done";
}

function messageStatus(status: AgentStatus): MessageStatus {
  return status === "running" ? "streaming" : status === "error" ? "error" : "done";
}

function statusLabel(status: AgentStatus) {
  if (status === "running") return tr(settingStore.language, "subagent.running");
  if (status === "error") return tr(settingStore.language, "subagent.failed");
  return tr(settingStore.language, "subagent.done");
}
</script>

<style scoped>
.subagent-sidebar {
  flex: none;
  box-sizing: border-box;
  width: 400px;
  min-width: 360px;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding-top: 34px;
  color: var(--peek-text);
  background: transparent;
}
.subagent-sidebar.embedded {
  flex: 1;
  width: 100%;
  min-width: 0;
  padding-top: 0;
}
.subagent-sidebar-header {
  flex: none;
  min-height: 40px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 8px 0 12px;
  border-bottom: 1px solid var(--peek-border);
}
.subagent-sidebar-header strong {
  font-size: 12px;
}
.close-button {
  margin-left: auto;
  width: 27px;
  height: 27px;
  display: inline-grid;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}
.close-button:hover {
  color: var(--peek-text);
  background: var(--peek-hover-bg);
}
.subagent-tabs {
  flex: none;
}
.subagent-tab-shell {
  flex: 0 0 200px;
  width: 200px;
  min-width: 200px;
  max-width: 260px;
}
.subagent-tab {
  flex: 1;
  min-width: 0;
  height: 100%;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 0 4px 0 9px;
  border: 0;
  background: transparent;
  color: inherit;
  cursor: pointer;
}
.subagent-tab > svg {
  flex: none;
  width: 15px;
  height: 15px;
  color: color-mix(in srgb, var(--peek-accent) 82%, var(--peek-text));
}
.subagent-tab span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 10px;
}
.subagent-tab-close {
  position: relative;
  z-index: 1;
  flex: none;
  width: 24px;
  height: 24px;
  display: inline-grid;
  place-items: center;
  margin-right: 4px;
  padding: 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--peek-faint);
  cursor: pointer;
  opacity: 0;
}
.subagent-tab-shell:hover .subagent-tab-close,
.subagent-tab-shell.active .subagent-tab-close {
  opacity: 1;
}
.subagent-tab-close:hover {
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 8%, transparent);
}
.subagent-panel {
  flex: 1;
  min-height: 0;
  overflow: auto;
}
.subagent-panel-header {
  min-width: 0;
  padding: 10px 13px 9px;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-border) 75%, transparent);
}
.subagent-identity {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 7px;
}
.subagent-identity svg {
  flex: none;
  color: var(--peek-accent);
}
.subagent-identity strong {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  color: var(--peek-text);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.agent-status {
  flex: none;
  margin-left: 4px;
  color: var(--peek-muted);
  font-size: 10px;
  white-space: nowrap;
}
.subagent-model-row {
  min-width: 0;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: start;
  gap: 7px;
  margin: 6px 0 0 21px;
  color: var(--peek-faint);
  font-size: 9px;
  line-height: 1.4;
}
.subagent-model-row > span {
  white-space: nowrap;
}
.subagent-model-row > code {
  min-width: 0;
  padding: 0;
  background: transparent;
  color: var(--peek-muted);
  font: inherit;
  font-family: var(--font-mono);
  overflow-wrap: anywhere;
  white-space: normal;
}
.agent-status.error {
  color: var(--destructive);
}
.subagent-work {
  padding: 10px 13px 18px;
}
.subagent-work :deep(.agent-work),
.subagent-work :deep(.markdown-body) {
  width: 100%;
  max-width: none;
  box-sizing: border-box;
}
.subagent-task-details {
  margin: 0 0 9px;
}
.subagent-task-toggle {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 5px;
  min-height: 28px;
  padding: 3px 5px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--peek-muted);
  font: inherit;
  font-size: 10px;
  text-align: left;
  cursor: pointer;
  user-select: none;
}
.subagent-task-toggle:hover {
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 4%, transparent);
}
.subagent-task-toggle:focus-visible {
  outline: 1px solid var(--peek-accent);
  outline-offset: 1px;
}
.subagent-task-toggle svg {
  flex: none;
  transition: transform 140ms ease;
}
.subagent-task-details.open > .subagent-task-toggle svg {
  transform: rotate(90deg);
}
.subagent-task-body {
  margin: 3px 0 8px 5px;
  padding: 7px 8px 3px 10px;
  border-left: 1px solid var(--peek-border);
}
.subagent-task-body > :deep(.markdown-body) {
  margin-bottom: 8px;
  color: var(--peek-muted);
  font-size: 11px;
}
.agent-waiting,
.subagent-empty {
  margin: 0;
  padding: 24px 12px;
  color: var(--peek-muted);
  font-size: 11px;
  text-align: center;
}
.subagent-empty {
  margin: auto 0;
}
@container workspace-sidebar (max-width: 560px) {
  .subagent-tab-shell {
    flex-basis: 168px;
    width: 168px;
    min-width: 168px;
  }
}
</style>
