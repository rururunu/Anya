<template>
  <section
    class="shell-terminal-card"
    :class="[status, { collapsed: !expanded }]"
    :data-state="status === 'running' ? 'running' : status"
  >
    <button
      type="button"
      class="shell-terminal-header"
      :aria-expanded="expanded"
      @click="expanded = !expanded"
    >
      <ChevronRight
        class="shell-terminal-chevron"
        :class="{ open: expanded }"
        :size="12"
        aria-hidden="true"
      />
      <Terminal class="shell-terminal-leading" :size="14" :stroke-width="1.75" aria-hidden="true" />
      <span class="shell-terminal-variant">{{ variantLabel }}</span>
      <span v-if="summaryLine" class="shell-terminal-separator" aria-hidden="true" />
      <span v-if="summaryLine" class="shell-terminal-summary">{{ summaryLine }}</span>
      <span v-if="status === 'error'" class="shell-terminal-status error">{{ failedLabel }}</span>
      <span v-else-if="exitBadge" class="shell-terminal-exit-badge">{{ exitBadge }}</span>
    </button>
    <div v-if="expanded" class="shell-terminal-screen">
      <div v-if="command" class="shell-terminal-cmdline">
        <span class="shell-terminal-ps1" aria-hidden="true">$</span>
        <span class="shell-terminal-cmd">{{ command }}</span>
      </div>
      <pre v-if="outputSpans.length" class="shell-terminal-body peek-scrollbar"><code><span
        v-for="(span, index) in outputSpans"
        :key="index"
        :style="spanStyle(span)"
      >{{ span.text }}</span></code></pre>
      <pre
        v-else-if="status === 'running'"
        class="shell-terminal-body muted peek-scrollbar"
      ><code>{{ waitingLabel }}</code></pre>
      <div v-if="exitBadge" class="shell-terminal-exit">{{ exitBadge }}</div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch, type CSSProperties } from "vue";
import { ChevronRight, Terminal } from "@lucide/vue";
import type { ToolActivity } from "@/types/chat";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import { parseAnsi, type AnsiSpan } from "@/services/chat/ansi";
import { toolVariantLabel, activitySummaryLine } from "@/services/chat/toolActivityDisplay";
import { activityMatchesQuery } from "@/services/chat/conversationFind";
import { useExpandForFind } from "@/composables/chat/useConversationFind";

const props = withDefaults(
  defineProps<{
    activity: ToolActivity;
    startCollapsed?: boolean;
  }>(),
  {
    startCollapsed: true,
  },
);

const settingStore = useSettingStore();
const expanded = ref(!props.startCollapsed);
const failedLabel = computed(() => tr(settingStore.language, "failed"));
const variantLabel = computed(() => toolVariantLabel(props.activity, settingStore.language));
const summaryLine = computed(() => {
  const command = String(props.activity.arguments?.command ?? "").trim();
  if (command) return command;
  return activitySummaryLine(props.activity);
});

const status = computed(() => props.activity.status);

watch(
  () => props.activity.status,
  (next, prev) => {
    if (props.startCollapsed && prev === "running" && next !== "running") {
      expanded.value = false;
    }
  },
);

useExpandForFind(
  (query) => activityMatchesQuery(props.activity, query),
  () => {
    expanded.value = true;
  },
);

const waitingLabel = computed(() => (settingStore.language === "zh-CN" ? "正在运行…" : "Running…"));

const command = computed(() => String(props.activity.arguments?.command ?? "").trim());

const output = computed(() =>
  (props.activity.result ?? extractOutputFromDetail(props.activity.detail)).trim(),
);

const outputSpans = computed<AnsiSpan[]>(() => {
  if (!output.value) return [];
  return parseAnsi(output.value);
});

const exitBadge = computed(() => {
  const match = output.value.match(/exit code:?\s*(-?\d+)/i);
  if (match && match[1] !== "0") return `exit ${match[1]}`;
  if (status.value === "error" && !match) return "exit 1";
  return "";
});

function spanStyle(span: AnsiSpan): CSSProperties | undefined {
  const style: CSSProperties = {};
  if (span.color) style.color = span.color;
  if (span.background) style.backgroundColor = span.background;
  if (span.bold) style.fontWeight = 700;
  if (span.dim) style.opacity = 0.65;
  if (span.italic) style.fontStyle = "italic";
  if (span.underline) style.textDecoration = "underline";
  return Object.keys(style).length ? style : undefined;
}

function extractOutputFromDetail(detail?: string | null): string {
  if (!detail) return "";
  const outputMatch = detail.match(/\*\*输出[：:]\*\*\s*```[^\n]*\n([\s\S]*?)```/);
  if (outputMatch?.[1]) return outputMatch[1].trimEnd();
  const fence = detail.match(/```(?:powershell|bash|shell|ps1)?\n([\s\S]*?)```/);
  return fence?.[1]?.trimEnd() ?? detail;
}
</script>

<style scoped>
/* 真实终端观感：不随主题变浅，始终深色屏幕底。 */
.shell-terminal-card {
  --term-bg: #0d1117;
  --term-header-bg: #161b22;
  --term-border: #2b3138;
  --term-fg: #c9d1d9;
  --term-muted: #8b949e;
  --term-green: #3fb950;
  --term-red: #f85149;
  width: 100%;
  box-sizing: border-box;
  margin: 4px 0 8px;
  border: 1px solid var(--term-border);
  border-radius: 10px;
  background: var(--term-bg);
  overflow: hidden;
}
.shell-terminal-card.running {
  border-color: color-mix(in srgb, var(--peek-accent) 45%, var(--term-border));
}
.shell-terminal-card.error {
  border-color: color-mix(in srgb, var(--term-red) 45%, var(--term-border));
}
.shell-terminal-header {
  position: relative;
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  min-height: 24px;
  margin: 0;
  padding: 4px 10px;
  border: 0;
  border-bottom: 1px solid var(--term-border);
  background: var(--term-header-bg);
  color: var(--term-fg);
  font: inherit;
  font-size: 12px;
  line-height: 24px;
  text-align: left;
  cursor: pointer;
  overflow: hidden;
}
.shell-terminal-card[data-state="running"] .shell-terminal-header::after {
  content: "";
  position: absolute;
  inset-block: 0;
  left: 0;
  width: 300px;
  background: linear-gradient(
    90deg,
    transparent 0%,
    color-mix(in srgb, var(--term-fg) 12%, transparent) 55%,
    transparent 100%
  );
  animation: shell-terminal-sweep 2.6s ease-out infinite;
  pointer-events: none;
}
@keyframes shell-terminal-sweep {
  0% {
    left: -300px;
  }
  90%,
  100% {
    left: 100%;
  }
}
.shell-terminal-card.collapsed .shell-terminal-header {
  border-bottom: 0;
}
.shell-terminal-chevron {
  flex: none;
  color: var(--term-muted);
  transition: transform 140ms ease;
}
.shell-terminal-chevron.open {
  transform: rotate(90deg);
}
.shell-terminal-leading {
  flex: none;
  color: var(--term-muted);
}
.shell-terminal-variant {
  flex: none;
  font-weight: 550;
  color: var(--term-fg);
}
.shell-terminal-separator {
  flex: none;
  width: 2px;
  height: 2px;
  margin: 0 2px;
  border-radius: 50%;
  background: var(--term-muted);
}
.shell-terminal-summary {
  min-width: 0;
  flex: 1 1 auto;
  overflow: hidden;
  color: var(--term-muted);
  font-family: var(--font-mono);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.shell-terminal-exit-badge {
  flex: none;
  font-family: var(--font-mono);
  font-size: 10px;
  font-weight: 650;
  color: var(--term-red);
}
.shell-terminal-status {
  flex: none;
  font-size: 10px;
  color: var(--term-muted);
}
.shell-terminal-status.error {
  color: var(--term-red);
}
.shell-terminal-screen {
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.6;
  font-variant-ligatures: none;
  font-variant-numeric: tabular-nums;
}
.shell-terminal-cmdline {
  display: flex;
  gap: 8px;
  padding: 9px 12px 0;
  color: var(--term-fg);
  white-space: pre-wrap;
  word-break: break-word;
}
.shell-terminal-ps1 {
  flex: none;
  font-weight: 700;
  color: var(--term-green);
}
.shell-terminal-cmd {
  min-width: 0;
  font-weight: 550;
}
.shell-terminal-body {
  margin: 0;
  max-height: var(--agent-card-max-height, 240px);
  overflow: auto;
  padding: 8px 12px 12px;
  color: var(--term-fg);
  font: inherit;
  white-space: pre-wrap;
  word-break: break-word;
}
.shell-terminal-body.muted {
  color: var(--term-muted);
}
.shell-terminal-body code {
  font: inherit;
  color: inherit;
  background: transparent;
}
.shell-terminal-exit {
  padding: 4px 12px 8px;
  font-size: 11px;
  font-weight: 700;
  color: var(--term-red);
}

@media (prefers-reduced-motion: reduce) {
  .shell-terminal-card[data-state="running"] .shell-terminal-header::after {
    animation: none;
  }
}
</style>
