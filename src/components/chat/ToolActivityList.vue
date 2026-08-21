<template>
  <div
    v-if="enrichedActivities.length"
    class="tool-activity-list"
    :class="{ operations, nested, flat }"
  >
    <template v-for="item in enrichedActivities" :key="item.activity.id">
      <ShellTerminalCard
        v-if="item.activity.kind === 'shell'"
        :activity="item.activity"
        :start-collapsed="cardsCollapsed"
      />

      <TaskListCard v-else-if="item.tasks.length" embedded :tasks="item.tasks" />

      <div v-else-if="item.hunks.length" class="file-diff-stack">
        <FileDiffCard
          v-for="(hunk, index) in item.hunks"
          :key="`${item.activity.id}-${index}`"
          :path="item.filePath"
          :hunk="hunk"
          :kind="item.activity.kind"
          :status="item.activity.status"
          :start-collapsed="cardsCollapsed"
        />
      </div>

      <section
        v-else
        class="tool-activity-card"
        :class="[
          item.activity.kind,
          item.activity.status,
          {
            subagent: isSubagentTool(item.activity),
            'subagent-running': isRunningSubagent(item.activity),
          },
        ]"
      >
        <div class="tool-activity-header">
          <div v-if="!canExpandItem(item.activity)" class="tool-activity-main tool-activity-static">
            <span class="tool-activity-icon" aria-hidden="true">
              <component :is="icon(item.activity)" :size="12" />
            </span>
            <span class="tool-activity-title">{{ activityTitle(item.activity) }}</span>
            <span v-if="isFuzzy(item.activity) && !flat" class="fuzzy-badge">
              {{ tr(settingStore.language, "fuzzyMatch") }}
            </span>
            <span v-if="item.activity.status === 'running'" class="tool-activity-status">
              {{
                tr(
                  settingStore.language,
                  item.activity.toolName === "ask_user"
                    ? "waitingAnswer"
                    : item.activity.preview
                      ? "waitingApproval"
                      : "running",
                )
              }}
            </span>
            <span v-else-if="item.activity.status === 'error'" class="tool-activity-status error">
              {{ tr(settingStore.language, "failed") }}
            </span>
          </div>
          <button
            v-else
            type="button"
            class="tool-activity-main"
            :aria-expanded="isExpanded(item.activity)"
            @click="toggleActivity(item.activity)"
          >
            <ChevronRight
              class="activity-chevron"
              :class="{ open: isExpanded(item.activity) }"
              :size="12"
            />
            <span class="tool-activity-icon" aria-hidden="true">
              <component :is="icon(item.activity)" :size="12" />
            </span>
            <span class="tool-activity-title">{{ activityTitle(item.activity) }}</span>
            <span v-if="isFuzzy(item.activity)" class="fuzzy-badge">
              {{ tr(settingStore.language, "fuzzyMatch") }}
            </span>
            <span v-if="item.activity.status === 'running'" class="tool-activity-status">
              {{
                tr(
                  settingStore.language,
                  item.activity.toolName === "ask_user"
                    ? "waitingAnswer"
                    : item.activity.preview
                      ? "waitingApproval"
                      : "running",
                )
              }}
            </span>
            <span v-else-if="item.activity.status === 'error'" class="tool-activity-status error">
              {{ tr(settingStore.language, "failed") }}
            </span>
          </button>
          <button
            v-if="
              showInspectAction &&
              isSubagentTool(item.activity) &&
              !childAgentRows(item.activity).length
            "
            type="button"
            class="inspect-subagent-button"
            :aria-label="inspectLabel"
            :title="inspectLabel"
            @click.stop="emit('inspectSubagent', item.activity.id)"
          >
            <PanelRightOpen :size="13" />
          </button>
        </div>

        <div v-if="shouldShowErrorBody(item.activity)" class="tool-activity-body">
          <div class="tool-activity-detail">
            <Markdown :content="errorBody(item.activity)" />
          </div>
        </div>

        <div
          v-if="isSubagentTool(item.activity) && childAgentRows(item.activity).length"
          class="child-agent-rows"
        >
          <button
            v-for="agent in childAgentRows(item.activity)"
            :key="agent.id"
            type="button"
            class="child-agent-row"
            :title="agent.prompt"
            @click.stop="emit('inspectSubagent', agent.id)"
          >
            <span class="tool-activity-icon" aria-hidden="true">
              <SubagentIcon :status="agent.status" :size="12" />
            </span>
            <span class="child-agent-title">{{ agent.title }}</span>
            <span v-if="agent.status === 'running'" class="tool-activity-status">
              {{ tr(settingStore.language, "running") }}
            </span>
            <span v-else-if="agent.status === 'error'" class="tool-activity-status error">
              {{ tr(settingStore.language, "failed") }}
            </span>
            <PanelRightOpen :size="13" class="child-agent-inspect" />
          </button>
        </div>

        <div
          v-if="
            isExpanded(item.activity) && (!isSubagentTool(item.activity) || showSubagentDetails)
          "
          class="tool-activity-body"
        >
          <div v-if="item.activity.detail" class="tool-activity-detail">
            <Markdown :content="item.activity.detail" />
          </div>
          <div v-else-if="shouldShowResult(item.activity)" class="tool-activity-detail">
            <Markdown :content="formatResult(item.activity.result!)" />
          </div>

          <ToolActivityList
            v-if="childActivities(item.activity).length"
            class="subagent-activity-list"
            :activities="childActivities(item.activity)"
            :all-activities="activityPool"
            :show-inspect-action="showInspectAction"
            :show-subagent-details="showSubagentDetails"
            nested
            @inspect-subagent="emit('inspectSubagent', $event)"
          />
        </div>
      </section>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, type Component } from "vue";
import {
  ChevronRight,
  FilePenLine,
  FilePlus2,
  FileX2,
  FolderSearch,
  MoveRight,
  Terminal,
  LoaderCircle,
  PanelRightOpen,
  Wrench,
  Workflow,
} from "@lucide/vue";
import Markdown from "@/components/chat/Markdown.vue";
import SubagentIcon from "@/components/chat/SubagentIcon.vue";
import ShellTerminalCard from "@/components/chat/ShellTerminalCard.vue";
import FileDiffCard from "@/components/chat/FileDiffCard.vue";
import TaskListCard from "@/components/chat/TaskListCard.vue";
import type { TaskItem, ToolActivity } from "@/types/chat";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import { SUBAGENT_TOOLS } from "@/services/chat/subagentTools";
import {
  hasExpandableActivityContent,
  shouldShowActivityResult,
} from "@/services/chat/toolActivityDisplay";
import { hunkFromPlainEdit, parseUnifiedDiffHunks, type DiffHunk } from "@/services/chat/toolDiff";

const props = withDefaults(
  defineProps<{
    activities: ToolActivity[];
    allActivities?: ToolActivity[];
    operations?: boolean;
    nested?: boolean;
    flat?: boolean;
    showInspectAction?: boolean;
    showSubagentDetails?: boolean;
    /** When true, shell/diff cards start collapsed (compact display mode). */
    cardsCollapsed?: boolean;
  }>(),
  {
    operations: false,
    nested: false,
    flat: false,
    showInspectAction: true,
    showSubagentDetails: false,
    cardsCollapsed: false,
  },
);
const emit = defineEmits<{ inspectSubagent: [activityId: string] }>();
const settingStore = useSettingStore();
const inspectLabel = computed(() => tr(settingStore.language, "subagent.view"));
const expandedIds = ref(new Set<string>());
const previousStatuses = new Map<string, ToolActivity["status"]>();

const activityPool = computed(() => props.allActivities ?? props.activities);

type ChildAgentRow = {
  id: string;
  title: string;
  prompt: string;
  status: ToolActivity["status"];
};

const FILE_OPERATION_KINDS = new Set(["create", "edit", "delete", "move"]);
const TASK_LIST_TOOLS = new Set(["update_tasks", "todo_write"]);

const enrichedActivities = computed(() =>
  props.activities
    .filter((activity) => {
      // Read/search rows live in process-detail folds. Keep them visible there
      // (`flat`) and under subagents (`nested`); hide them from the open stream.
      if (activity.kind === "read" && !props.nested && !props.flat) return false;
      return !(activity.toolName === "ask_user" && activity.status !== "running");
    })
    .map((activity) => {
      const hunks = collectHunks(activity);
      const filePath = String(activity.preview?.path ?? activity.arguments?.path ?? "");
      return {
        activity,
        hunks,
        tasks: tasksFromActivity(activity),
        filePath: filePath || "file",
      };
    }),
);

function tasksFromActivity(activity: ToolActivity): TaskItem[] {
  if (!TASK_LIST_TOOLS.has(activity.toolName)) return [];
  const raw = activity.arguments?.tasks;
  if (!Array.isArray(raw)) return [];
  return raw.flatMap((value) => {
    if (!value || typeof value !== "object") return [];
    const item = value as Record<string, unknown>;
    const content = String(item.content ?? "").trim();
    if (!content) return [];
    const activeForm =
      typeof item.activeForm === "string"
        ? item.activeForm
        : typeof item.active_form === "string"
          ? item.active_form
          : undefined;
    const level = typeof item.level === "number" ? item.level : undefined;
    return [
      {
        content,
        status: String(item.status ?? "pending"),
        activeForm,
        level,
      },
    ];
  });
}

function collectHunks(activity: ToolActivity): DiffHunk[] {
  if (activity.kind === "shell") return [];
  const args = activity.arguments ?? {};
  const preview = activity.preview;
  if (preview?.unifiedDiff?.trim()) {
    return parseUnifiedDiffHunks(preview.unifiedDiff);
  }
  if (activity.toolName === "replace_many_in_file" && Array.isArray(args.edits)) {
    return args.edits.map((value) => {
      const edit = value as Record<string, unknown>;
      return hunkFromPlainEdit(String(edit.old_string ?? ""), String(edit.new_string ?? ""));
    });
  }
  if (preview && (preview.oldText != null || preview.newText != null)) {
    return [hunkFromPlainEdit(preview.oldText ?? "", preview.newText ?? "")];
  }
  if (activity.toolName === "replace_in_file") {
    return [hunkFromPlainEdit(String(args.old_string ?? ""), String(args.new_string ?? ""))];
  }
  if (activity.toolName === "write_file") {
    return [hunkFromPlainEdit("", String(args.content ?? ""))];
  }
  if (activity.kind === "delete") {
    const deleted = args.old_string ?? args.symbol ?? args.start_anchor ?? activity.detail ?? "";
    return [hunkFromPlainEdit(String(deleted), "")];
  }
  // File ops without structured edits should still fall through to markdown card.
  if (!FILE_OPERATION_KINDS.has(activity.kind) && !preview) return [];
  return [];
}

function activityTitle(activity: ToolActivity) {
  return (
    activity.title?.trim() || activity.toolName?.trim() || tr(settingStore.language, "unknownTool")
  );
}

function shouldShowResult(activity: ToolActivity) {
  return shouldShowActivityResult(activity);
}

function canExpandItem(activity: ToolActivity) {
  if (props.flat) return false;
  return hasExpandableActivityContent(activity, {
    childCount: childActivities(activity).length,
    showSubagentDetails: props.showSubagentDetails,
  });
}

function shouldShowErrorBody(activity: ToolActivity) {
  if (activity.status !== "error") return false;
  if (isExpanded(activity) && (!isSubagentTool(activity) || props.showSubagentDetails)) {
    return false;
  }
  return Boolean(activity.detail?.trim() || activity.result?.trim());
}

function errorBody(activity: ToolActivity) {
  const detail = activity.detail?.trim();
  if (detail) return detail;
  return formatResult(activity.result ?? "");
}

function isFuzzy(activity: ToolActivity) {
  return /fuzzy/i.test(activity.title) || /fuzzy/i.test(activity.result ?? "");
}

function icon(activity: ToolActivity): Component {
  if (isRunningSubagent(activity)) return LoaderCircle;
  if (isSubagentTool(activity)) return Workflow;
  switch (activity.kind) {
    case "shell":
      return Terminal;
    case "create":
      return FilePlus2;
    case "edit":
      return FilePenLine;
    case "delete":
      return FileX2;
    case "move":
      return MoveRight;
    case "read":
      return FolderSearch;
    default:
      return Wrench;
  }
}

function isSubagentTool(activity: ToolActivity) {
  return SUBAGENT_TOOLS.has(activity.toolName);
}

function isRunningSubagent(activity: ToolActivity) {
  return isSubagentTool(activity) && activity.status === "running";
}

function childActivities(activity: ToolActivity) {
  return activityPool.value.filter((candidate) => candidate.parentActivityId === activity.id);
}

function childAgentRows(activity: ToolActivity): ChildAgentRow[] {
  if (!isSubagentTool(activity)) return [];
  const args = activity.arguments ?? {};
  const prompts = Array.isArray(args.tasks)
    ? args.tasks.map((value) => {
        if (typeof value !== "object" || value == null) return "";
        return String((value as Record<string, unknown>).prompt ?? "").trim();
      })
    : [args.prompt, args.task, args.description]
        .filter((value): value is string => typeof value === "string" && Boolean(value.trim()))
        .slice(0, 1)
        .map((value) => value.trim());
  const groups = new Map<string, ToolActivity[]>();
  for (const child of childActivities(activity)) {
    const key = child.subagentId ?? "default";
    const group = groups.get(key) ?? [];
    group.push(child);
    groups.set(key, group);
  }
  const grouped = [...groups.values()];
  const count = Math.max(prompts.length, grouped.length, 1);
  return Array.from({ length: count }, (_, index) => {
    const prompt = prompts[index] ?? prompts[0] ?? activity.title;
    const children = grouped[index] ?? [];
    const status = children.some((child) => child.status === "error")
      ? "error"
      : children.some((child) => child.status === "running")
        ? "running"
        : activity.status;
    return {
      id: `${activity.id}:${index}`,
      title: shortTaskTitle(prompt, index),
      prompt,
      status,
    };
  });
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
    .replace(/^(?:任务|task|assignment)\s*[:：-]\s*/i, "")
    .replace(/[`*_~]/g, "")
    .trim();
  const prefix = tr(settingStore.language, "subagent.numbered", { count: index + 1 });
  const title = cleaned ? `${prefix} · ${cleaned}` : prefix;
  return title.length > 72 ? `${title.slice(0, 71)}...` : title;
}

function isExpanded(activity: ToolActivity) {
  return expandedIds.value.has(activity.id);
}

function setExpanded(activityId: string, expanded: boolean) {
  const next = new Set(expandedIds.value);
  if (expanded) next.add(activityId);
  else next.delete(activityId);
  expandedIds.value = next;
}

function toggleActivity(activity: ToolActivity) {
  if (!canExpandItem(activity)) return;
  if (isSubagentTool(activity) && !props.showSubagentDetails) {
    return;
  }
  setExpanded(activity.id, !isExpanded(activity));
}

function formatResult(result: string) {
  return result.startsWith("```") ? result : `\`\`\`\n${result}\n\`\`\``;
}

function keepExpandedWhenDone(activity: ToolActivity) {
  return (
    activity.kind === "shell" ||
    FILE_OPERATION_KINDS.has(activity.kind) ||
    activity.toolName === "update_tasks" ||
    activity.toolName === "todo_write"
  );
}

watch(
  () => activityPool.value.map((activity) => `${activity.id}:${activity.status}`).join("|"),
  () => {
    for (const activity of activityPool.value) {
      const previous = previousStatuses.get(activity.id);
      if (
        previous === undefined &&
        (activity.status === "running" || activity.status === "error") &&
        canExpandItem(activity)
      ) {
        setExpanded(activity.id, true);
      } else if (previous === "running" && activity.status === "done") {
        if (!keepExpandedWhenDone(activity)) {
          setExpanded(activity.id, false);
        }
      } else if (activity.status === "error" && previous !== "error" && canExpandItem(activity)) {
        setExpanded(activity.id, true);
      }
      previousStatuses.set(activity.id, activity.status);
    }
  },
  { immediate: true },
);
</script>

<style scoped>
.tool-activity-list {
  display: flex;
  flex-direction: column;
  gap: 3px;
  width: 100%;
  margin-bottom: 0;
  box-sizing: border-box;
}
.tool-activity-list.operations {
  gap: 3px;
}
.file-diff-stack {
  display: flex;
  flex-direction: column;
  gap: 0;
  width: 100%;
}
.tool-activity-list.flat {
  gap: 1px;
}
.tool-activity-list.flat .tool-activity-header {
  padding: 2px 2px 2px 0;
  min-height: 0;
}
.tool-activity-list.flat .tool-activity-title {
  white-space: normal;
  text-overflow: unset;
  overflow: visible;
  line-height: 1.45;
}
.tool-activity-list.flat .tool-activity-status {
  font-size: 10px;
}
.tool-activity-list.nested {
  gap: 2px;
  margin: 2px 8px 8px 28px;
  width: calc(100% - 36px);
  padding-left: 8px;
  border-left: 1px solid color-mix(in srgb, var(--peek-border) 82%, transparent);
}
.tool-activity-card {
  width: 100%;
  box-sizing: border-box;
  border: 0;
  border-radius: 6px;
  background: transparent;
  overflow: hidden;
}
.tool-activity-card.running {
  background: color-mix(in srgb, var(--peek-accent) 7%, transparent);
}
.tool-activity-card.subagent {
  margin: 3px 0;
  border: 0;
  background: transparent;
}
.tool-activity-card.subagent-running {
  background: transparent;
  box-shadow: none;
}
.tool-activity-card.subagent > .tool-activity-header {
  min-height: 34px;
  padding: 6px 8px;
  color: var(--peek-text);
  font-weight: 600;
}
.tool-activity-card.subagent > .tool-activity-body > .tool-activity-detail {
  padding: 4px 12px 10px 36px;
  color: color-mix(in srgb, var(--peek-text) 84%, var(--peek-muted));
  line-height: 1.55;
}
.tool-activity-card.subagent-running > .tool-activity-header .tool-activity-icon {
  background: transparent;
}
.tool-activity-card.subagent-running > .tool-activity-header .tool-activity-icon :deep(svg) {
  animation: subagent-tool-spin 900ms linear infinite;
}
.tool-activity-card.error {
  background: color-mix(in srgb, var(--destructive) 8%, transparent);
}
.tool-activity-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  color: var(--peek-muted);
  font-size: 11px;
  line-height: 1.35;
  width: 100%;
  background: transparent;
  border-radius: 6px;
  transition:
    background 120ms ease,
    color 120ms ease;
}
.tool-activity-main {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  user-select: none;
}
.tool-activity-main:not(.tool-activity-static) {
  cursor: pointer;
}
.tool-activity-static {
  cursor: default;
}
.tool-activity-header:hover {
  background: color-mix(in srgb, var(--peek-text) 5%, transparent);
  color: var(--peek-text);
}
.activity-chevron {
  flex: none;
  color: var(--peek-faint);
  transition: transform 150ms ease;
}
.activity-chevron.open {
  transform: rotate(90deg);
}
.tool-activity-main[aria-expanded="true"] {
  color: var(--peek-text);
  border-bottom: 0;
}
.tool-activity-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  width: 16px;
  height: 16px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--peek-accent) 10%, transparent);
  color: var(--peek-accent);
}
.tool-activity-title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.fuzzy-badge {
  flex: none;
  font-size: 9px;
  padding: 0 5px;
  border-radius: 999px;
  background: color-mix(in srgb, #eab308 22%, transparent);
  color: #eab308;
  font-weight: 650;
}
.tool-activity-status {
  flex: none;
  color: var(--peek-muted);
  font-size: 10px;
}
.tool-activity-status.error {
  color: var(--destructive);
}
.inspect-subagent-button {
  flex: none;
  width: 23px;
  height: 23px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: 0;
  border-radius: 4px;
  color: var(--peek-muted);
  background: transparent;
  cursor: pointer;
}
.inspect-subagent-button:hover {
  color: var(--peek-accent);
  background: color-mix(in srgb, var(--peek-accent) 12%, transparent);
}
.child-agent-rows {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin: 0 6px 5px 28px;
  padding-left: 8px;
  border-left: 1px solid color-mix(in srgb, var(--peek-border) 78%, transparent);
}
.child-agent-row {
  width: 100%;
  min-width: 0;
  min-height: 29px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 6px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--peek-muted);
  text-align: left;
  cursor: pointer;
}
.child-agent-row:hover {
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 5%, transparent);
}
.child-agent-title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.child-agent-inspect {
  flex: none;
  color: var(--peek-faint);
}
.child-agent-row:hover .child-agent-inspect {
  color: var(--peek-accent);
}
.tool-activity-detail {
  padding: 2px 6px 8px 28px;
  font-size: 11px;
  color: var(--peek-muted);
}
.tool-activity-detail :deep(pre) {
  margin: 0;
  max-height: var(--agent-card-max-height, 240px);
  overflow: auto;
  border-radius: 6px;
}
.tool-activity-card.create .tool-activity-icon {
  background: color-mix(in srgb, #22c55e 15%, transparent);
  color: #22c55e;
}
.tool-activity-card.edit .tool-activity-icon {
  background: color-mix(in srgb, #eab308 15%, transparent);
  color: #eab308;
}
.tool-activity-card.delete .tool-activity-icon {
  background: color-mix(in srgb, var(--destructive) 15%, transparent);
  color: var(--destructive);
}
@keyframes subagent-tool-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
