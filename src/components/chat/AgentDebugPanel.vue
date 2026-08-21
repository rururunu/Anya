<template>
  <aside
    class="agent-debug-panel"
    :class="{ embedded }"
    :aria-label="tr(settingStore.language, 'runtime.title')"
  >
    <header class="debug-header">
      <div class="debug-heading">
        <Bug :size="14" />
        <strong>{{ tr(settingStore.language, "runtime.title") }}</strong>
        <span v-if="runList.length" class="run-count">{{ runList.length }}</span>
      </div>
      <div class="debug-actions">
        <button
          class="icon-btn"
          type="button"
          :disabled="!selectedRun"
          :title="tr(settingStore.language, 'runtime.copyRun')"
          @click="copyValue('run', stringify(selectedRun))"
        >
          <Check v-if="isCopied('run')" :size="13" />
          <Copy v-else :size="13" />
        </button>
        <button
          class="icon-btn"
          type="button"
          :title="tr(settingStore.language, 'runtime.clear')"
          @click="clear"
        >
          <Trash2 :size="13" />
        </button>
        <button
          v-if="!embedded"
          class="icon-btn"
          type="button"
          :title="tr(settingStore.language, 'runtime.close')"
          @click="emit('close')"
        >
          <X :size="14" />
        </button>
      </div>
    </header>

    <div v-if="runList.length" class="run-bar">
      <select v-model="selectedRunId" :aria-label="tr(settingStore.language, 'runtime.run')">
        <option v-for="run in runList" :key="run.id" :value="run.id">
          {{ shortId(run.id) }} · {{ stateLabel(run.state) }}
        </option>
      </select>
      <span class="state-pill" :data-state="selectedRun?.state">
        {{ stateLabel(selectedRun?.state) }}
      </span>
    </div>

    <nav class="debug-tabs" :aria-label="tr(settingStore.language, 'runtime.views')">
      <button
        v-for="item in tabs"
        :key="item.id"
        type="button"
        :class="{ active: tab === item.id }"
        @click="tab = item.id"
      >
        {{ item.label }}
        <span v-if="item.id === 'events'">{{ selectedRun?.events.length ?? 0 }}</span>
        <span v-if="item.id === 'tools'">{{ selectedRun?.tools.length ?? 0 }}</span>
        <span v-if="item.id === 'subagents'" :class="{ 'active-count': activeSubagentCount > 0 }">
          {{ activeSubagentCount || selectedRun?.subagents.length || 0 }}
        </span>
        <span v-if="item.id === 'tokens' && selectedRun?.tokenUsage">
          {{ formatTokens(selectedRun.tokenUsage.totalTokens) }}
        </span>
      </button>
    </nav>

    <div v-if="!selectedRun && tab !== 'logs'" class="debug-empty">
      {{ tr(settingStore.language, "runtime.waiting") }}
    </div>

    <div v-else-if="tab === 'logs'" class="debug-scroll log-list peek-scrollbar">
      <article
        v-for="(entry, index) in frontendLogs"
        :key="`${entry.ts}-${index}`"
        class="event-row"
      >
        <span class="event-sequence" :data-level="entry.level">{{ entry.level }}</span>
        <div>
          <div class="event-title">
            <strong>[{{ entry.scope }}] {{ entry.message }}</strong>
            <time>{{ formatTime(entry.ts) }}</time>
          </div>
          <pre v-if="entry.detail !== undefined">{{ stringify(entry.detail) }}</pre>
        </div>
      </article>
      <p v-if="frontendLogs.length === 0" class="inline-empty">
        {{ tr(settingStore.language, "runtime.noLogs") }}
      </p>
    </div>

    <div
      v-else-if="tab === 'events'"
      ref="eventListRef"
      class="debug-scroll event-list peek-scrollbar"
    >
      <article v-for="record in selectedRun.events" :key="record.sequence" class="event-row">
        <span class="event-sequence">{{ record.sequence }}</span>
        <div>
          <div class="event-title">
            <strong>{{ eventName(record.event) }}</strong>
            <div class="event-actions">
              <time>{{ formatTime(record.timestampMs) }}</time>
              <button
                type="button"
                class="copy-btn"
                :title="tr(settingStore.language, 'runtime.copyEvent')"
                @click="copyValue(`event-${record.sequence}`, stringify(record))"
              >
                <Check v-if="isCopied(`event-${record.sequence}`)" :size="11" />
                <Copy v-else :size="11" />
              </button>
            </div>
          </div>
          <pre v-if="eventDetail(record.event)">{{ eventDetail(record.event) }}</pre>
        </div>
      </article>
    </div>

    <div v-else-if="tab === 'tools'" class="debug-scroll tool-list peek-scrollbar">
      <article v-for="tool in selectedRun.tools" :key="tool.callId" class="tool-card">
        <header>
          <TerminalSquare :size="13" />
          <strong>{{ tool.tool }}</strong>
          <span
            :class="[
              'tool-status',
              tool.success === false ? 'failed' : tool.result ? 'done' : 'running',
            ]"
          >
            {{ toolStatusLabel(tool) }}
          </span>
        </header>
        <p v-if="tool.description">{{ tool.description }}</p>
        <details open>
          <summary>{{ tr(settingStore.language, "runtime.arguments") }}</summary>
          <div class="payload-block">
            <button
              type="button"
              class="copy-btn"
              :title="tr(settingStore.language, 'runtime.copyArguments')"
              @click="copyValue(`args-${tool.callId}`, stringify(tool.arguments))"
            >
              <Check v-if="isCopied(`args-${tool.callId}`)" :size="11" />
              <Copy v-else :size="11" />
            </button>
            <pre>{{ stringify(tool.arguments) }}</pre>
          </div>
        </details>
        <details v-if="tool.result">
          <summary>{{ tr(settingStore.language, "runtime.result") }}</summary>
          <div class="payload-block">
            <button
              type="button"
              class="copy-btn"
              :title="tr(settingStore.language, 'runtime.copyResult')"
              @click="copyValue(`result-${tool.callId}`, tool.result)"
            >
              <Check v-if="isCopied(`result-${tool.callId}`)" :size="11" />
              <Copy v-else :size="11" />
            </button>
            <pre>{{ tool.result }}</pre>
          </div>
        </details>
      </article>
      <p v-if="!selectedRun.tools.length" class="inline-empty">
        {{ tr(settingStore.language, "runtime.noTools") }}
      </p>
    </div>

    <div v-else-if="tab === 'subagents'" class="debug-scroll subagent-list peek-scrollbar">
      <article
        v-for="agent in selectedRun.subagents"
        :key="agent.id"
        class="subagent-card"
        :class="{ running: agent.status === 'running' }"
        :style="{ marginLeft: `${Math.max(0, agent.depth - 1) * 12}px` }"
      >
        <button type="button" class="subagent-header" @click="toggleSubagent(agent)">
          <ChevronRight
            :size="12"
            class="subagent-chevron"
            :class="{ open: isSubagentExpanded(agent) }"
          />
          <SubagentIcon :status="agent.status" :size="13" />
          <strong>{{ shortId(agent.id) }}</strong>
          <span v-if="agent.readOnly" class="readonly-badge">
            {{ tr(settingStore.language, "runtime.readOnly") }}
          </span>
          <span :class="['tool-status', agent.status]">{{ runtimeStatusLabel(agent.status) }}</span>
        </button>
        <div v-if="isSubagentExpanded(agent)" class="subagent-body">
          <div class="subagent-description-row">
            <p class="subagent-description">
              {{ agent.description || tr(settingStore.language, "runtime.task") }}
            </p>
            <button
              type="button"
              class="copy-btn"
              :title="tr(settingStore.language, 'runtime.copyTask')"
              @click="
                copyValue(
                  `subagent-task-${agent.id}`,
                  agent.description || tr(settingStore.language, 'runtime.task'),
                )
              "
            >
              <Check v-if="isCopied(`subagent-task-${agent.id}`)" :size="11" />
              <Copy v-else :size="11" />
            </button>
          </div>
          <div class="subagent-meta">
            <span>{{ tr(settingStore.language, "runtime.depth", { depth: agent.depth }) }}</span>
            <time>{{ formatTime(agent.startedAt) }}</time>
          </div>
          <ol v-if="agent.progress.length" class="subagent-progress">
            <li
              v-for="(progress, index) in agent.progress"
              :key="`${progress.timestampMs}-${index}`"
            >
              <span>{{ progress.kind }}</span>
              <strong>{{ progress.content }}</strong>
            </li>
          </ol>
          <div v-if="agent.tools.length" class="subagent-tools">
            <div v-for="tool in agent.tools" :key="tool.callId" class="subagent-tool-row">
              <TerminalSquare :size="11" />
              <span>{{ tool.tool }}</span>
              <small :class="tool.success === false ? 'failed' : tool.result ? 'done' : 'running'">
                {{ toolStatusLabel(tool) }}
              </small>
            </div>
          </div>
          <details v-if="agent.summary">
            <summary>{{ tr(settingStore.language, "runtime.finalResult") }}</summary>
            <div class="payload-block">
              <button
                type="button"
                class="copy-btn"
                :title="tr(settingStore.language, 'runtime.copyFinalResult')"
                @click="copyValue(`subagent-result-${agent.id}`, agent.summary)"
              >
                <Check v-if="isCopied(`subagent-result-${agent.id}`)" :size="11" />
                <Copy v-else :size="11" />
              </button>
              <pre>{{ agent.summary }}</pre>
            </div>
          </details>
        </div>
      </article>
      <p v-if="!selectedRun.subagents.length" class="inline-empty">
        {{ tr(settingStore.language, "runtime.noSubagents") }}
      </p>
    </div>

    <div v-else-if="tab === 'tokens'" class="debug-scroll token-view peek-scrollbar">
      <template v-if="selectedRun.tokenUsage">
        <div class="token-summary">
          <div>
            <span>{{ tr(settingStore.language, "runtime.model") }}</span>
            <strong>{{ selectedRun.model || "unknown" }}</strong>
          </div>
          <div>
            <span>{{ tr(settingStore.language, "runtime.accuracy") }}</span>
            <strong :class="['accuracy-value', selectedRun.tokenUsage.accuracy]">
              {{ accuracyLabel(selectedRun.tokenUsage.accuracy) }}
            </strong>
          </div>
          <div class="token-total">
            <span>{{ tr(settingStore.language, "runtime.totalTokens") }}</span>
            <strong>{{ formatTokens(selectedRun.tokenUsage.totalTokens) }}</strong>
          </div>
        </div>
        <dl class="token-breakdown">
          <template v-for="item in tokenBreakdown(selectedRun.tokenUsage)" :key="item.key">
            <dt>{{ item.label }}</dt>
            <dd>{{ formatTokens(item.value) }}</dd>
          </template>
        </dl>
        <p v-if="selectedRun.tokenUsage.source" class="token-source">
          {{ selectedRun.tokenUsage.source }}
        </p>
      </template>
      <p v-else class="inline-empty">{{ tr(settingStore.language, "runtime.noTokens") }}</p>
    </div>

    <div v-else class="debug-scroll context-view peek-scrollbar">
      <div v-if="selectedRun.context" class="payload-block context-payload">
        <button
          type="button"
          class="copy-btn"
          :title="tr(settingStore.language, 'runtime.copyContext')"
          @click="copyValue('context', stringify(selectedRun.context))"
        >
          <Check v-if="isCopied('context')" :size="11" />
          <Copy v-else :size="11" />
        </button>
        <pre>{{ stringify(selectedRun.context) }}</pre>
      </div>
      <p v-else class="inline-empty">{{ tr(settingStore.language, "runtime.noContext") }}</p>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref } from "vue";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Bug, Check, ChevronRight, Copy, TerminalSquare, Trash2, X } from "@lucide/vue";
import SubagentIcon from "@/components/chat/SubagentIcon.vue";
import { listenAgentDebugEvent } from "@/services/ipc/events";
import { getAgentDebugSnapshot } from "@/services/ipc/commands";
import { copyText } from "@/services/clipboard";
import { tr } from "@/services/i18n";
import { getRecentLogs, type LogEntry } from "@/services/logger";
import { useSettingStore } from "@/stores/setting";
import type {
  AgentDebugEvent,
  AgentEvent,
  AgentEventRecord,
  AgentState,
  CapturedContext,
  TokenAccuracy,
  TokenUsage,
} from "@/types/chat";

type DebugTab = "events" | "tools" | "subagents" | "tokens" | "context" | "logs";
interface DebugToolCall {
  callId: string;
  tool: string;
  description: string;
  arguments: Record<string, unknown>;
  result?: string;
  success?: boolean;
}
interface DebugRun {
  id: string;
  state: AgentState;
  events: AgentEventRecord[];
  tools: DebugToolCall[];
  subagents: DebugSubagent[];
  context?: CapturedContext;
  model?: string;
  tokenUsage?: TokenUsage;
}
interface DebugProgress {
  kind: string;
  content: string;
  timestampMs: number;
}
interface DebugSubagent {
  id: string;
  parentSubagentId?: string;
  description: string;
  readOnly: boolean;
  depth: number;
  status: "running" | "done" | "failed";
  startedAt: number;
  finishedAt?: number;
  progress: DebugProgress[];
  tools: DebugToolCall[];
  summary?: string;
}

const emit = defineEmits<{ close: [] }>();
const props = withDefaults(defineProps<{ embedded?: boolean }>(), { embedded: false });
const embedded = computed(() => props.embedded);
const settingStore = useSettingStore();
const runs = reactive<Record<string, DebugRun>>({});
const selectedRunId = ref("");
const tab = ref<DebugTab>("events");
const expandedSubagents = reactive(new Set<string>());
const eventListRef = ref<HTMLElement | null>(null);
const copiedKey = ref("");
const frontendLogs = ref<LogEntry[]>([]);
let logsPollTimer: ReturnType<typeof setInterval> | undefined;
const tabs = computed<Array<{ id: DebugTab; label: string }>>(() => [
  { id: "events", label: tr(settingStore.language, "runtime.events") },
  { id: "tools", label: tr(settingStore.language, "runtime.tools") },
  { id: "subagents", label: tr(settingStore.language, "runtime.subagents") },
  { id: "tokens", label: tr(settingStore.language, "runtime.tokens") },
  { id: "context", label: tr(settingStore.language, "runtime.context") },
  { id: "logs", label: tr(settingStore.language, "runtime.logs") },
]);
let unlisten: UnlistenFn | undefined;
let unlistenFocus: UnlistenFn | undefined;
let copyResetTimer: ReturnType<typeof setTimeout> | undefined;

const runList = computed(() => Object.values(runs).reverse());
const selectedRun = computed(() => runs[selectedRunId.value]);
const activeSubagentCount = computed(
  () => selectedRun.value?.subagents.filter((agent) => agent.status === "running").length ?? 0,
);

function ensureRun(runId: string, state: AgentState = "created") {
  runs[runId] ??= { id: runId, state, events: [], tools: [], subagents: [] };
  selectedRunId.value = runId;
  return runs[runId];
}

function receive(payload: AgentDebugEvent) {
  try {
    if (payload.type === "runCreated") {
      ensureRun(payload.data.runId, payload.data.state);
      return;
    }
    if (payload.type === "contextSnapshot") {
      ensureRun(payload.data.runId).context = payload.data.context;
      return;
    }
    if (payload.type === "tokenUsage") {
      const run = ensureRun(payload.data.runId);
      run.model = payload.data.model;
      run.tokenUsage = payload.data.usage;
      return;
    }
    if (payload.type === "toolCall") {
      const run = ensureRun(payload.data.runId);
      const existing = run.tools.find((tool) => tool.callId === payload.data.callId);
      if (!existing) run.tools.push({ ...payload.data });
      return;
    }
    if (payload.type === "subagentStarted") {
      const run = ensureRun(payload.data.runId);
      if (!run.subagents.some((agent) => agent.id === payload.data.subagentId)) {
        run.subagents.push({
          id: payload.data.subagentId,
          parentSubagentId: payload.data.parentSubagentId,
          description: payload.data.description,
          readOnly: payload.data.readOnly,
          depth: payload.data.depth,
          status: "running",
          startedAt: payload.data.timestampMs,
          progress: [],
          tools: [],
        });
      }
      return;
    }
    if (payload.type === "subagentProgress") {
      const agent = findSubagent(payload.data.runId, payload.data.subagentId);
      if (agent) {
        const progress = {
          kind: payload.data.kind,
          content: payload.data.content,
          timestampMs: payload.data.timestampMs,
        };
        if (
          !agent.progress.some(
            (item) =>
              item.timestampMs === progress.timestampMs &&
              item.kind === progress.kind &&
              item.content === progress.content,
          )
        ) {
          agent.progress.push(progress);
        }
        if (agent.progress.length > 100) agent.progress.splice(0, agent.progress.length - 100);
      }
      return;
    }
    if (payload.type === "subagentToolCall") {
      const agent = findSubagent(payload.data.runId, payload.data.subagentId);
      if (agent && !agent.tools.some((tool) => tool.callId === payload.data.callId)) {
        agent.tools.push({ ...payload.data });
      }
      return;
    }
    if (payload.type === "subagentToolResult") {
      const agent = findSubagent(payload.data.runId, payload.data.subagentId);
      const tool = agent?.tools.find((item) => item.callId === payload.data.callId);
      if (tool) {
        tool.result = payload.data.result;
        tool.success = payload.data.success;
      }
      return;
    }
    if (payload.type === "subagentFinished") {
      const agent = findSubagent(payload.data.runId, payload.data.subagentId);
      if (agent) {
        agent.status = payload.data.success ? "done" : "failed";
        agent.summary = payload.data.summary;
        agent.finishedAt = payload.data.timestampMs;
        expandedSubagents.delete(agent.id);
      }
      return;
    }
    const { record } = payload.data;
    const run = ensureRun(record.runId);
    if (!run.events.some((item) => item.sequence === record.sequence)) run.events.push(record);
    if (run.events.length > 500) run.events.splice(0, run.events.length - 500);
    if (record.event.type === "stateChanged") run.state = record.event.data.to;
    if (record.event.type === "toolResult") {
      const result = record.event.data;
      const tool = run.tools.find((item) => item.callId === result.callId);
      if (tool) {
        tool.result = result.result;
        tool.success = result.success;
      }
    }
    void nextTick(() => eventListRef.value?.scrollTo({ top: eventListRef.value.scrollHeight }));
  } catch (error) {
    console.warn("Agent debug event ignored:", error);
  }
}

function findSubagent(runId: string, subagentId: string) {
  return ensureRun(runId).subagents.find((agent) => agent.id === subagentId);
}

function isSubagentExpanded(agent: DebugSubagent) {
  return expandedSubagents.has(agent.id);
}

function toggleSubagent(agent: DebugSubagent) {
  if (expandedSubagents.has(agent.id)) expandedSubagents.delete(agent.id);
  else expandedSubagents.add(agent.id);
}

function eventName(event: AgentEvent) {
  return event.type.replace(/([A-Z])/g, " $1").replace(/^./, (value) => value.toUpperCase());
}
function eventDetail(event: AgentEvent) {
  if (event.type === "stateChanged") return `${event.data.from} → ${event.data.to}`;
  if (event.type === "toolCalled") return event.data.tool;
  if (event.type === "toolResult")
    return `${event.data.tool} · ${event.data.success ? "success" : "failed"}`;
  if (event.type === "error") return event.data.message;
  if (event.type === "fileChanged") return event.data.path;
  return "";
}
function stateLabel(state?: AgentState) {
  return state?.replace(/([A-Z])/g, " $1") ?? "unknown";
}
function runtimeStatusLabel(status: "running" | "done" | "failed") {
  return tr(settingStore.language, `runtime.${status}`);
}
function toolStatusLabel(tool: DebugToolCall) {
  return runtimeStatusLabel(tool.success === false ? "failed" : tool.result ? "done" : "running");
}
function formatTime(timestamp: number) {
  return new Date(timestamp).toLocaleTimeString([], { hour12: false });
}
function shortId(id: string) {
  return id.slice(0, 8);
}
function stringify(value: unknown) {
  return JSON.stringify(value, null, 2);
}
function formatTokens(value: number) {
  return new Intl.NumberFormat(settingStore.language).format(value);
}
function accuracyLabel(accuracy: TokenAccuracy) {
  return tr(settingStore.language, `runtime.accuracy.${accuracy}`);
}
function tokenBreakdown(usage: TokenUsage) {
  const items = [
    {
      key: "input",
      label: tr(settingStore.language, "runtime.inputTokens"),
      value: usage.inputTokens,
    },
    {
      key: "output",
      label: tr(settingStore.language, "runtime.outputTokens"),
      value: usage.outputTokens,
    },
    {
      key: "system",
      label: tr(settingStore.language, "runtime.systemTokens"),
      value: usage.systemTokens,
    },
    {
      key: "context",
      label: tr(settingStore.language, "runtime.contextTokens"),
      value: usage.contextTokens,
    },
    {
      key: "toolCall",
      label: tr(settingStore.language, "runtime.toolCallTokens"),
      value: usage.toolCallTokens,
    },
    {
      key: "toolResult",
      label: tr(settingStore.language, "runtime.toolResultTokens"),
      value: usage.toolResultTokens,
    },
    {
      key: "memory",
      label: tr(settingStore.language, "runtime.memoryTokens"),
      value: usage.memoryTokens,
    },
  ];
  if ((usage.cacheReadTokens ?? 0) > 0) {
    items.push({
      key: "cacheRead",
      label: tr(settingStore.language, "usage.cacheRead"),
      value: usage.cacheReadTokens ?? 0,
    });
  }
  if ((usage.reasoningTokens ?? 0) > 0) {
    items.push({
      key: "reasoning",
      label: tr(settingStore.language, "usage.reasoning"),
      value: usage.reasoningTokens ?? 0,
    });
  }
  return items;
}
function isCopied(key: string) {
  return copiedKey.value === key;
}
async function copyValue(key: string, value: string | undefined) {
  if (!value) return;
  try {
    await copyText(value);
    copiedKey.value = key;
    if (copyResetTimer) globalThis.clearTimeout(copyResetTimer);
    copyResetTimer = globalThis.setTimeout(() => {
      if (copiedKey.value === key) copiedKey.value = "";
    }, 1400);
  } catch (error) {
    console.warn("Failed to copy Agent Runtime data:", error);
  }
}
function clear() {
  for (const key of Object.keys(runs)) delete runs[key];
  selectedRunId.value = "";
  expandedSubagents.clear();
}

async function hydrateSnapshot() {
  try {
    const snapshot = await getAgentDebugSnapshot();
    snapshot.forEach(receive);
  } catch (error) {
    console.warn("Agent debug snapshot unavailable:", error);
  }
}

onMounted(async () => {
  frontendLogs.value = getRecentLogs();
  logsPollTimer = globalThis.setInterval(() => {
    frontendLogs.value = getRecentLogs();
  }, 1000);
  try {
    unlisten = await listenAgentDebugEvent(receive);
    await hydrateSnapshot();
    unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) void hydrateSnapshot();
    });
  } catch (error) {
    console.warn("Agent debug listener unavailable:", error);
  }
});
onUnmounted(() => {
  unlisten?.();
  unlistenFocus?.();
  if (copyResetTimer) globalThis.clearTimeout(copyResetTimer);
  if (logsPollTimer) globalThis.clearInterval(logsPollTimer);
});
</script>

<style scoped>
.agent-debug-panel {
  position: absolute;
  z-index: 12;
  inset: 33px 0 0 auto;
  width: min(620px, 52%);
  min-width: 380px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-list-bg) 97%, transparent);
  border-left: 1px solid var(--peek-border);
  box-shadow: -12px 0 26px rgba(0, 0, 0, 0.18);
}
.agent-debug-panel.embedded {
  position: relative;
  z-index: auto;
  inset: auto;
  flex: 1;
  width: 100%;
  min-width: 0;
  height: 100%;
  border: 0;
  background: transparent;
  box-shadow: none;
}
.debug-header,
.run-bar,
.debug-actions,
.debug-heading,
.tool-card header {
  display: flex;
  align-items: center;
}
.debug-header {
  min-height: 38px;
  justify-content: space-between;
  padding: 0 10px 0 12px;
  border-bottom: 1px solid var(--peek-border);
}
.debug-heading,
.tool-card header {
  gap: 7px;
}
.debug-heading strong {
  font-size: 12px;
}
.run-count {
  min-width: 18px;
  height: 18px;
  display: inline-grid;
  place-items: center;
  padding: 0 5px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--peek-text) 7%, transparent);
  color: var(--peek-muted);
  font: 9px/1 var(--font-mono);
}
.debug-actions {
  gap: 3px;
}
.icon-btn {
  width: 26px;
  height: 26px;
  display: grid;
  place-items: center;
  border: 0;
  border-radius: 4px;
  color: var(--peek-muted);
  background: transparent;
  cursor: pointer;
}
.icon-btn:hover {
  color: var(--peek-text);
  background: var(--peek-hover);
}
.icon-btn:disabled {
  opacity: 0.35;
  cursor: default;
}
.run-bar {
  gap: 8px;
  padding: 8px 10px;
  border-bottom: 1px solid var(--peek-border);
}
.run-bar select {
  min-width: 0;
  flex: 1;
  height: 28px;
  color: var(--peek-text);
  background: var(--peek-input-bg);
  border: 1px solid var(--peek-border);
  border-radius: 4px;
  padding: 0 7px;
}
.state-pill,
.tool-status {
  font-size: 10px;
  text-transform: uppercase;
  color: var(--peek-muted);
}
.state-pill[data-state="completed"],
.tool-status.done {
  color: #58b887;
}
.state-pill[data-state="failed"],
.tool-status.failed {
  color: #e06c75;
}
.debug-tabs {
  display: flex;
  height: 32px;
  padding: 3px 8px 0;
  gap: 2px;
  border-bottom: 1px solid var(--peek-border);
}
.debug-tabs button {
  border: 0;
  border-bottom: 2px solid transparent;
  color: var(--peek-muted);
  background: transparent;
  padding: 0 9px;
  font-size: 11px;
  cursor: pointer;
}
.debug-tabs button.active {
  color: var(--peek-text);
  border-bottom-color: var(--peek-accent);
}
.debug-tabs span {
  margin-left: 4px;
  opacity: 0.7;
}
.debug-scroll {
  min-height: 0;
  flex: 1;
  overflow: auto;
  padding: 10px;
}
.event-list {
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.event-row {
  display: grid;
  grid-template-columns: 25px minmax(0, 1fr);
  gap: 6px;
  padding: 7px;
  border-radius: 5px;
}
.event-row:hover {
  background: color-mix(in srgb, var(--peek-text) 4%, transparent);
}
.event-sequence {
  font:
    10px ui-monospace,
    monospace;
  color: var(--peek-muted);
}
.event-title {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  font-size: 11px;
}
.event-actions {
  flex: none;
  display: flex;
  align-items: center;
  gap: 4px;
}
.event-title time {
  color: var(--peek-muted);
  font:
    10px ui-monospace,
    monospace;
}
pre {
  margin: 5px 0 0;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  color: var(--peek-muted);
  font:
    10px/1.5 ui-monospace,
    monospace;
}
.tool-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.tool-card {
  padding: 9px;
  border: 1px solid var(--peek-border);
  border-radius: 6px;
  background: color-mix(in srgb, var(--peek-input-bg) 78%, transparent);
}
.tool-card header strong {
  flex: 1;
  font-size: 11px;
}
.tool-card p,
summary {
  color: var(--peek-muted);
  font-size: 10px;
}
details {
  margin-top: 7px;
}
summary {
  cursor: pointer;
}
.payload-block {
  position: relative;
  min-width: 0;
  margin-top: 5px;
  padding: 8px 30px 8px 9px;
  border-radius: 5px;
  background: color-mix(in srgb, var(--peek-text) 4%, transparent);
}
.payload-block pre {
  margin: 0;
}
.copy-btn {
  flex: none;
  width: 23px;
  height: 23px;
  display: inline-grid;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--peek-faint);
  cursor: pointer;
}
.copy-btn:hover {
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 7%, transparent);
}
.payload-block > .copy-btn {
  position: absolute;
  top: 4px;
  right: 4px;
}
.debug-empty,
.inline-empty {
  color: var(--peek-muted);
  font-size: 11px;
  text-align: center;
  padding: 28px 12px;
}
.context-payload {
  margin-top: 0;
}
.context-payload > pre {
  color: var(--peek-text);
}
.token-view {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.token-summary {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 12px 20px;
}
.token-summary > div {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.token-summary span,
.token-source {
  color: var(--peek-muted);
  font-size: 10px;
}
.token-summary strong {
  overflow: hidden;
  font: 11px/1.4 var(--font-mono);
  text-overflow: ellipsis;
  white-space: nowrap;
}
.token-summary .token-total {
  grid-column: 1 / -1;
  padding-top: 12px;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 55%, transparent);
}
.token-total strong {
  font-size: 24px;
  font-weight: 500;
}
.accuracy-value.exact {
  color: #58b887;
}
.accuracy-value.mixed {
  color: var(--peek-accent);
}
.accuracy-value.estimated {
  color: var(--peek-muted);
}
.token-breakdown {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0;
  margin: 0;
}
.token-breakdown dt,
.token-breakdown dd {
  min-height: 30px;
  display: flex;
  align-items: center;
  margin: 0;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-border) 45%, transparent);
  font-size: 11px;
}
.token-breakdown dt {
  color: var(--peek-muted);
}
.token-breakdown dd {
  justify-content: flex-end;
  color: var(--peek-text);
  font-family: var(--font-mono);
  font-variant-numeric: tabular-nums;
}
.token-source {
  margin: -8px 0 0;
  font-family: var(--font-mono);
  overflow-wrap: anywhere;
}
.subagent-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.subagent-card {
  border: 1px solid var(--peek-border);
  border-radius: 6px;
  background: color-mix(in srgb, var(--peek-input-bg) 78%, transparent);
  overflow: hidden;
  transition:
    border-color 140ms ease,
    background 140ms ease,
    box-shadow 140ms ease;
}
.subagent-card.running {
  border-color: color-mix(in srgb, var(--peek-accent) 42%, var(--peek-border));
}
.subagent-header,
.subagent-meta,
.subagent-tool-row {
  display: flex;
  align-items: center;
  gap: 7px;
}
.subagent-header {
  width: 100%;
  min-height: 34px;
  padding: 0 9px;
  border: 0;
  color: var(--peek-text);
  background: transparent;
  text-align: left;
  cursor: pointer;
}
.subagent-header strong {
  flex: 1;
  font-size: 11px;
}
.subagent-body {
  padding: 0 9px 9px 28px;
}
.subagent-chevron {
  color: var(--peek-muted);
  transition: transform 120ms ease;
}
.subagent-chevron.open {
  transform: rotate(90deg);
}
.tool-status.running,
.active-count {
  color: var(--peek-accent);
}
.active-count {
  font-weight: 700;
}
.readonly-badge {
  color: var(--peek-muted);
  font-size: 8px;
  letter-spacing: 0.04em;
}
.subagent-description {
  margin: 7px 0 5px;
  color: var(--peek-text);
  font-size: 11px;
  line-height: 1.45;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.subagent-description-row {
  display: flex;
  align-items: flex-start;
  gap: 6px;
}
.subagent-description-row .subagent-description {
  flex: 1;
  min-width: 0;
}
.subagent-description-row .copy-btn {
  margin-top: 4px;
}
.subagent-meta {
  color: var(--peek-muted);
  font:
    9px ui-monospace,
    monospace;
}
.subagent-meta time {
  margin-left: auto;
}
.subagent-progress {
  margin: 8px 0 0;
  padding: 0;
  list-style: none;
}
.subagent-progress li {
  display: grid;
  grid-template-columns: 72px minmax(0, 1fr);
  gap: 6px;
  padding: 4px 0;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 55%, transparent);
  font-size: 10px;
}
.subagent-progress span {
  color: var(--peek-muted);
}
.subagent-progress strong {
  min-width: 0;
  overflow-wrap: anywhere;
  font-weight: 500;
}
.subagent-tools {
  margin-top: 7px;
  border-top: 1px solid var(--peek-border);
}
.subagent-tool-row {
  min-height: 27px;
  font-size: 10px;
}
.subagent-tool-row span {
  flex: 1;
}
.subagent-tool-row small {
  color: var(--peek-muted);
}
.subagent-tool-row small.done {
  color: #58b887;
}
.subagent-tool-row small.failed {
  color: #e06c75;
}
</style>
