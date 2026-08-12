<template>
  <div
    v-if="segments.length"
    class="agent-work"
    :class="{ 'has-running-subagent': hasRunningSubagent }"
  >
    <!-- Live turn: keep chronological interleaving for follow-along. -->
    <template v-if="streaming">
      <AgentWorkSegment
        v-for="(segment, segmentIndex) in segments"
        :key="segment.id"
        :segment="segment"
        :streaming="streaming"
        :follow="streaming && segment.id === lastSegmentId"
        :language="language"
        :show-reasoning-summary="isFirstReasoningSegment(segmentIndex)"
        :show-narration-summary="true"
        :all-activities="visibleActivities"
        :collapsible="segment.type === 'process' && processSegmentCollapsible(segment)"
        :process-open="segment.type === 'process' && isProcessOpen(segment.id)"
        :headline="segment.type === 'process' ? processHeadline(segment) : ''"
        :cards-collapsed="displayMode === 'compact'"
        @inspect-subagent="emit('inspectSubagent', $event)"
        @preview-image="emit('previewImage', $event)"
        @toggle-process="toggleProcess"
      />
    </template>

    <!-- Completed: fold thinking + tools into one collapsed block; keep final reply open. -->
    <template v-else>
      <details
        v-if="preambleSegments.length"
        class="agent-work-fold"
        :open="foldOpen"
        @toggle="handleFoldToggle"
      >
        <summary class="agent-work-fold-summary" :class="{ pinned: foldOpen }">
          <ChevronRight class="agent-work-fold-chevron" :class="{ open: foldOpen }" :size="12" />
          <span>{{ foldLabel }}</span>
          <span v-if="foldMeta" class="agent-work-fold-meta">{{ foldMeta }}</span>
        </summary>
        <div class="agent-work-fold-body">
          <AgentWorkSegment
            v-for="segment in preambleSegments"
            :key="segment.id"
            :segment="segment"
            :streaming="false"
            :follow="false"
            :language="language"
            :show-reasoning-summary="false"
            :show-narration-summary="false"
            :all-activities="visibleActivities"
            :collapsible="segment.type === 'process' && processSegmentCollapsible(segment)"
            :process-open="segment.type === 'process' && isProcessOpen(segment.id)"
            :headline="segment.type === 'process' ? processHeadline(segment) : ''"
            :cards-collapsed="displayMode === 'compact'"
            @inspect-subagent="emit('inspectSubagent', $event)"
            @preview-image="emit('previewImage', $event)"
            @toggle-process="toggleProcess"
          />
        </div>
      </details>

      <AgentWorkSegment
        v-for="segment in replySegments"
        :key="segment.id"
        :segment="segment"
        :streaming="false"
        :follow="false"
        :language="language"
        :show-reasoning-summary="true"
        :show-narration-summary="true"
        :all-activities="visibleActivities"
        :collapsible="false"
        :process-open="false"
        headline=""
        :cards-collapsed="displayMode === 'compact'"
        @inspect-subagent="emit('inspectSubagent', $event)"
        @preview-image="emit('previewImage', $event)"
        @toggle-process="toggleProcess"
      />
    </template>
  </div>
</template>

<script setup lang="ts">
import { ChevronRight } from "@lucide/vue";
import { computed, reactive, ref, watch } from "vue";
import AgentWorkSegment from "@/components/chat/AgentWorkSegment.vue";
import type { ChatMessage, ToolActivity } from "@/types/chat";
import type { AgentWorkDisplay, AppLanguage } from "@/types/setting";
import { SUBAGENT_TOOLS } from "@/services/chat/subagentTools";
import {
  isProcessSegmentCollapsible,
  summarizeProcessActivities,
} from "@/services/chat/toolActivityDisplay";
import { tr } from "@/services/i18n";

const props = withDefaults(
  defineProps<{
    message: ChatMessage;
    language?: AppLanguage;
    showReasoning?: boolean;
    /** detailed = shell/diff inline; compact = fold into process details. */
    displayMode?: AgentWorkDisplay;
    /** Content is a special-cased marker (e.g. "configure a provider") rendered
     * elsewhere, so skip showing it as regular inline text here. */
    suppressContent?: boolean;
  }>(),
  {
    displayMode: "detailed",
  },
);
const emit = defineEmits<{
  inspectSubagent: [activityId: string];
  previewImage: [source: string];
}>();

type TimelineSegment =
  | { type: "reasoning"; id: string; content: string }
  | { type: "narration"; id: string; content: string }
  | { type: "content"; id: string; content: string }
  | { type: "inline"; id: string; activities: ToolActivity[]; operations: boolean }
  | { type: "process"; id: string; activities: ToolActivity[]; operations: boolean };

const SHOWCASE_KINDS = new Set(["shell", "create", "edit", "delete", "move"]);
const TASK_LIST_TOOLS = new Set(["update_tasks", "todo_write"]);

const processOpen = reactive(new Map<string, boolean>());
const userToggledProcess = reactive(new Set<string>());

const foldOpen = ref(false);
const foldPinned = ref(false);

const streaming = computed(
  () => props.message.status === "pending" || props.message.status === "streaming",
);
const waitingForAskUser = computed(
  () =>
    props.message.toolActivities?.some(
      (activity) => activity.toolName === "ask_user" && activity.status === "running",
    ) ?? false,
);
const visibleActivities = computed(() =>
  (props.message.toolActivities ?? []).filter(
    (activity) => !(activity.toolName === "ask_user" && activity.status !== "running"),
  ),
);
const activityById = computed(
  () =>
    new Map(
      visibleActivities.value
        .filter((activity) => !activity.parentActivityId)
        .map((activity) => [activity.id, activity]),
    ),
);
const topLevelActivities = computed(() =>
  visibleActivities.value.filter((activity) => !activity.parentActivityId),
);

const hasRunningSubagent = computed(() =>
  topLevelActivities.value.some(
    (activity) => activity.status === "running" && SUBAGENT_TOOLS.has(activity.toolName),
  ),
);

/** Task lists + (in detailed mode) shell/file edits stay in the open stream. */
function isInlineActivity(activity: ToolActivity): boolean {
  if (TASK_LIST_TOOLS.has(activity.toolName)) return true;
  if (props.displayMode === "compact") return false;
  return SHOWCASE_KINDS.has(activity.kind);
}

function isOperationsActivity(activity: ToolActivity): boolean {
  return SHOWCASE_KINDS.has(activity.kind) && activity.kind !== "shell";
}

function segmentKind(activity: ToolActivity): "inline" | "process" {
  return isInlineActivity(activity) ? "inline" : "process";
}

function pushActivity(segments: TimelineSegment[], activity: ToolActivity) {
  const kind = segmentKind(activity);
  const operations = isOperationsActivity(activity);
  const last = segments[segments.length - 1];
  if (last && last.type === kind && last.operations === operations) {
    last.activities.push(activity);
    return;
  }
  const base = {
    id: `${kind}-${activity.id}`,
    activities: [activity],
    operations,
  };
  segments.push(kind === "inline" ? { type: "inline", ...base } : { type: "process", ...base });
}

type TextSegment = Extract<TimelineSegment, { type: "reasoning" | "content" }>;

function isTextSegment(segment: TimelineSegment): segment is TextSegment {
  return segment.type === "reasoning" || segment.type === "content";
}

/**
 * Append any part of `finalText` that isn't already covered by the matching
 * segments in `out`. Keeps the reply visible even when the timeline is
 * missing, partial, or stale without mutating reactive timeline items.
 */
function reconcileTrailingText(
  out: TimelineSegment[],
  kind: "reasoning" | "content",
  finalText: string | undefined,
) {
  const finalValue = finalText ?? "";
  if (!finalValue) return;
  let accumulated = "";
  for (const segment of out) {
    if (isTextSegment(segment) && segment.type === kind) accumulated += segment.content;
  }
  if (finalValue === accumulated) return;

  if (!accumulated || finalValue.startsWith(accumulated)) {
    const missing = finalValue.slice(accumulated.length);
    if (!missing) return;
    const last = out[out.length - 1];
    if (last && isTextSegment(last) && last.type === kind) {
      last.content += missing;
    } else {
      out.push({
        type: kind,
        id: `${kind}-final-${out.length}`,
        content: missing,
      } as TimelineSegment);
    }
    return;
  }

  if (accumulated.includes(finalValue)) return;
  const notice = finalValue.trim();
  if (!notice) return;
  if (accumulated.includes(notice)) return;
  out.push({
    type: kind,
    id: `${kind}-final-${out.length}`,
    content: notice.startsWith("已停止") || notice.startsWith("Stopped") ? `\n\n${notice}` : notice,
  } as TimelineSegment);
}

/**
 * Once the turn is finished, fold every interleaved reasoning chunk into a
 * single entry so the completed fold stays short.
 */
function coalesceCompletedReasoning(out: TimelineSegment[]): TimelineSegment[] {
  if (streaming.value) return out;
  const fromMessage = props.message.reasoning?.trim() ?? "";
  const fromSegments = out
    .filter((segment): segment is Extract<TimelineSegment, { type: "reasoning" }> => {
      return segment.type === "reasoning";
    })
    .map((segment) => segment.content)
    .join("");
  const combined = pickFullText(fromMessage, fromSegments);
  const withoutReasoning = out.filter((segment) => segment.type !== "reasoning");
  if (!combined || props.showReasoning === false) return withoutReasoning;

  const firstIdx = out.findIndex((segment) => segment.type === "reasoning");
  const block: TimelineSegment = {
    type: "reasoning",
    id: `${props.message.id}-reasoning-completed`,
    content: combined,
  };
  if (firstIdx <= 0) return [block, ...withoutReasoning];

  const before = out.slice(0, firstIdx).filter((segment) => segment.type !== "reasoning");
  const after = out.slice(firstIdx).filter((segment) => segment.type !== "reasoning");
  return [...before, block, ...after];
}

function pickFullText(fromMessage: string, fromSegments: string): string {
  if (!fromMessage) return fromSegments;
  if (!fromSegments) return fromMessage;
  if (fromMessage === fromSegments) return fromMessage;
  if (fromMessage.startsWith(fromSegments)) return fromMessage;
  if (fromSegments.startsWith(fromMessage)) return fromSegments;
  return fromSegments.length >= fromMessage.length ? fromSegments : fromMessage;
}

/**
 * After tools finish, fold mid-turn narration (content before the last tool)
 * into a collapsed narration block. Only the final reply after the last tool
 * stays outside the completed fold.
 */
function coalesceCompletedNarration(out: TimelineSegment[]): TimelineSegment[] {
  if (streaming.value) return out;
  const hasTools = out.some((segment) => segment.type === "process" || segment.type === "inline");
  if (!hasTools) return out;

  let lastToolIdx = -1;
  for (let i = 0; i < out.length; i++) {
    if (out[i].type === "process" || out[i].type === "inline") lastToolIdx = i;
  }
  if (lastToolIdx < 0) return out;

  const intermediate: string[] = [];
  const kept: TimelineSegment[] = [];
  let insertAt = 0;

  for (let i = 0; i < out.length; i++) {
    const segment = out[i];
    if (segment.type === "content" && i < lastToolIdx) {
      if (segment.content.trim()) intermediate.push(segment.content.trim());
      if (intermediate.length === 1) insertAt = kept.length;
      continue;
    }
    kept.push(segment);
  }

  const joined = intermediate.join("\n\n").trim();
  const hasVisibleReply = kept.some((segment) => segment.type === "content");

  let visibleTail = "";
  let folded = joined;
  if (!hasVisibleReply && intermediate.length > 0) {
    visibleTail = intermediate[intermediate.length - 1] ?? "";
    folded = intermediate.slice(0, -1).join("\n\n").trim();
  }

  if (folded) {
    if (kept[0]?.type === "reasoning") {
      insertAt = Math.max(insertAt, 1);
    }
    kept.splice(insertAt, 0, {
      type: "narration",
      id: `${props.message.id}-narration-completed`,
      content: folded,
    });
  }

  if (visibleTail) {
    kept.push({
      type: "content",
      id: `${props.message.id}-narration-tail`,
      content: visibleTail,
    });
  }

  return kept;
}

/** Single chronological stream with process-detail chunks interleaved. */
const segments = computed<TimelineSegment[]>(() => {
  let out: TimelineSegment[] = [];
  const timeline = props.message.workTimeline ?? [];
  const seen = new Set<string>();

  for (const item of timeline) {
    if (item.type === "content" && props.suppressContent) continue;
    if (item.type === "reasoning" || item.type === "content") {
      if (item.content.trim()) {
        out.push({ ...item });
      }
      continue;
    }
    const activity = activityById.value.get(item.toolActivityId);
    if (!activity) continue;
    seen.add(activity.id);
    pushActivity(out, activity);
  }

  reconcileTrailingText(out, "reasoning", props.message.reasoning);
  if (!props.suppressContent) {
    reconcileTrailingText(out, "content", props.message.content);
  }

  if (props.showReasoning === false) {
    out = out.filter((segment) => segment.type !== "reasoning");
  }

  for (const activity of topLevelActivities.value) {
    if (seen.has(activity.id)) continue;
    pushActivity(out, activity);
  }
  return coalesceCompletedNarration(coalesceCompletedReasoning(out));
});

const preambleSegments = computed(() =>
  segments.value.filter((segment) => segment.type !== "content"),
);
const replySegments = computed(() =>
  segments.value.filter((segment) => segment.type === "content"),
);

const foldReasoningText = computed(() => {
  const fromSegments = preambleSegments.value
    .filter((segment): segment is Extract<TimelineSegment, { type: "reasoning" }> => {
      return segment.type === "reasoning";
    })
    .map((segment) => segment.content)
    .join("");
  return pickFullText(props.message.reasoning?.trim() ?? "", fromSegments);
});

const foldLabel = computed(() => {
  const language = props.language ?? "zh-CN";
  return tr(language, "worked");
});

const foldMeta = computed(() => {
  const language = props.language ?? "zh-CN";
  const chars = foldReasoningText.value.length;
  if (chars > 0) {
    return tr(language, "chars", { count: chars.toLocaleString() });
  }
  const toolCount = preambleSegments.value.reduce((count, segment) => {
    if (segment.type === "process" || segment.type === "inline") {
      return count + segment.activities.length;
    }
    return count;
  }, 0);
  if (toolCount > 0) {
    return tr(language, "toolCount", { count: String(toolCount) });
  }
  return "";
});

const lastSegmentId = computed(() => segments.value[segments.value.length - 1]?.id);

function isFirstReasoningSegment(index: number) {
  for (let i = 0; i < index; i++) {
    if (segments.value[i]?.type === "reasoning") return false;
  }
  return true;
}

function processSegmentCollapsible(segment: Extract<TimelineSegment, { type: "process" }>) {
  return isProcessSegmentCollapsible(segment.activities, visibleActivities.value);
}

function processHeadline(segment: Extract<TimelineSegment, { type: "process" }>) {
  const language = props.language ?? "zh-CN";
  return summarizeProcessActivities(segment.activities, language);
}

function isProcessOpen(id: string) {
  return processOpen.get(id) ?? false;
}

function toggleProcess(id: string) {
  userToggledProcess.add(id);
  processOpen.set(id, !isProcessOpen(id));
}

function handleFoldToggle(event: Event) {
  const target = event.currentTarget as HTMLDetailsElement | null;
  if (!target) return;
  foldOpen.value = target.open;
  foldPinned.value = target.open;
}

/** Collapsed by default; only expand while showcase work is actively running. */
function shouldAutoExpandProcess(segment: Extract<TimelineSegment, { type: "process" }>) {
  if (waitingForAskUser.value) return true;
  return segment.activities.some(
    (activity) => activity.status === "running" && SHOWCASE_KINDS.has(activity.kind),
  );
}

watch(
  () => streaming.value,
  (live) => {
    if (live) {
      foldPinned.value = false;
      foldOpen.value = false;
      return;
    }
    // Always collapse the completed fold unless the user expands it afterward.
    if (!foldPinned.value) {
      foldOpen.value = false;
    }
  },
  { immediate: true },
);

watch(
  () =>
    [
      props.message.status,
      props.message.askUserAnswer?.length ?? 0,
      streaming.value,
      segments.value
        .map((segment) =>
          segment.type === "process"
            ? `${segment.id}:${segment.activities.map((a) => a.status).join(",")}`
            : segment.id,
        )
        .join("|"),
    ] as const,
  () => {
    for (const segment of segments.value) {
      if (segment.type !== "process") continue;
      if (userToggledProcess.has(segment.id)) continue;
      processOpen.set(segment.id, shouldAutoExpandProcess(segment));
    }
  },
  { immediate: true },
);
</script>

<style scoped>
.agent-work {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
  margin-bottom: 8px;
  box-sizing: border-box;
}

.agent-work :deep(.shell-terminal-card),
.agent-work :deep(.file-diff-card),
.agent-work :deep(.task-list-card.embedded) {
  margin-left: 0;
  margin-right: 0;
}

.agent-work-fold {
  width: 100%;
  margin: 0;
  border: 0;
  background: transparent;
}

.agent-work-fold-summary {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  padding: 3px 2px;
  font-family: var(--peek-font-sans);
  font-size: 11px;
  font-weight: 550;
  color: var(--peek-muted);
  list-style: none;
  user-select: none;
}

/* While expanded, keep the collapse control reachable no matter how long the
   folded reasoning/tool timeline is — pin it to the top of the scroll
   viewport instead of letting it scroll away with the rest of the body. */
.agent-work-fold-summary.pinned {
  position: sticky;
  top: 0;
  z-index: 3;
  background: var(--peek-bg);
  margin: 0 -2px;
  padding: 3px 2px;
  border-radius: 6px;
}

.agent-work-fold-summary::-webkit-details-marker {
  display: none;
}

.agent-work-fold-chevron {
  flex: none;
  color: var(--peek-faint);
  transition: transform 160ms ease;
}

.agent-work-fold-chevron.open {
  transform: rotate(90deg);
}

.agent-work-fold-meta {
  margin-left: auto;
  font-weight: 500;
  font-size: 11px;
  color: var(--peek-faint);
}

.agent-work-fold-body {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 2px 0 4px;
}

.agent-work.has-running-subagent .agent-work-fold-summary {
  color: var(--peek-text);
}
</style>
