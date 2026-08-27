<template>
  <section class="shell-terminal-card" :class="[status, { collapsed: !expanded }]">
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
      <span class="shell-terminal-dot" :class="status" aria-hidden="true"></span>
      <span class="shell-terminal-prompt" aria-hidden="true">&gt;_</span>
      <span class="shell-terminal-title">{{ title }}</span>
      <span v-if="status === 'running'" class="shell-terminal-status">{{ runningLabel }}</span>
      <span v-else-if="status === 'error'" class="shell-terminal-status error">
        {{ failedLabel }}
      </span>
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
import { ChevronRight } from "@lucide/vue";
import type { ToolActivity } from "@/types/chat";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import { parseAnsi, type AnsiSpan } from "@/services/chat/ansi";
import { activityMatchesQuery } from "@/services/chat/conversationFind";
import { useExpandForFind } from "@/composables/chat/useConversationFind";

const props = withDefaults(
  defineProps<{
    activity: ToolActivity;
    startCollapsed?: boolean;
  }>(),
  {
    startCollapsed: false,
  },
);

const settingStore = useSettingStore();
const expanded = ref(!props.startCollapsed || props.activity.status === "running");
const runningLabel = computed(() => tr(settingStore.language, "running"));
const failedLabel = computed(() => tr(settingStore.language, "failed"));
const waitingLabel = computed(() => (settingStore.language === "zh-CN" ? "正在运行…" : "Running…"));

const status = computed(() => props.activity.status);

watch(
  () => props.activity.status,
  (next, prev) => {
    if (next === "running") expanded.value = true;
    else if (props.startCollapsed && prev === "running") {
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

const title = computed(() => {
  const args = props.activity.arguments ?? {};
  const description = typeof args.description === "string" ? args.description.trim() : "";
  if (description) return description;
  const raw = props.activity.title
    .replace(/^执行命令[：:]\s*/u, "")
    .replace(/^运行命令[：:]\s*/u, "")
    .replace(/^Run(?:ning)?(?:\s+command)?[：:]\s*/i, "")
    .trim();
  return raw || props.activity.title;
});

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
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  min-height: 34px;
  margin: 0;
  padding: 7px 12px;
  border: 0;
  border-bottom: 1px solid var(--term-border);
  background: var(--term-header-bg);
  color: var(--term-fg);
  font: inherit;
  font-size: 12px;
  line-height: 1.35;
  text-align: left;
  cursor: pointer;
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
.shell-terminal-dot {
  flex: none;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--term-green);
}
.shell-terminal-dot.running {
  background: var(--peek-accent);
  animation: shell-terminal-pulse 1.2s ease-in-out infinite;
}
.shell-terminal-dot.error {
  background: var(--term-red);
}
@keyframes shell-terminal-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.35;
  }
}
.shell-terminal-prompt {
  flex: none;
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 700;
  color: var(--term-muted);
  letter-spacing: -0.04em;
}
.shell-terminal-title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 550;
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
</style>
