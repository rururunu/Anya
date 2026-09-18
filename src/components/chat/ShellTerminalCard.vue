<template>
  <section
    class="shell-terminal-card"
    :class="[status, { collapsed: !expanded, ok: isOk, fail: isFail }]"
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
      <Terminal class="shell-terminal-leading" :size="13" :stroke-width="1.75" aria-hidden="true" />
      <span class="shell-terminal-variant">{{ variantLabel }}</span>
      <span v-if="summaryLine" class="shell-terminal-summary">{{ summaryLine }}</span>
      <span v-if="durationLabel" class="shell-terminal-meta">{{ durationLabel }}</span>
      <span v-if="status === 'running'" class="shell-terminal-status running" aria-hidden="true" />
      <span v-else-if="isFail" class="shell-terminal-status fail">{{ exitLabel || "err" }}</span>
      <span v-else-if="isOk && exitLabel" class="shell-terminal-status ok">{{ exitLabel }}</span>
    </button>

    <div v-if="expanded" class="shell-terminal-screen">
      <div v-if="command" class="shell-terminal-cmdline">
        <span class="shell-terminal-ps1" aria-hidden="true">$</span>
        <span class="shell-terminal-cmd">{{ command }}</span>
      </div>

      <pre v-if="stdoutSpans.length" class="shell-terminal-body peek-scrollbar"><code><span
        v-for="(span, index) in stdoutSpans"
        :key="`out-${index}`"
        :style="spanStyle(span)"
      >{{ span.text }}</span></code></pre>

      <pre
        v-if="stderrText"
        class="shell-terminal-body stderr peek-scrollbar"
      ><code>{{ stderrText }}</code></pre>

      <pre
        v-else-if="!parsed.structured && outputSpans.length"
        class="shell-terminal-body peek-scrollbar"
      ><code><span
        v-for="(span, index) in outputSpans"
        :key="index"
        :style="spanStyle(span)"
      >{{ span.text }}</span></code></pre>

      <pre
        v-else-if="status === 'running' && !stdoutSpans.length && !stderrText"
        class="shell-terminal-body muted peek-scrollbar"
      ><code>{{ waitingLabel }}<span class="shell-terminal-cursor" aria-hidden="true" /></code></pre>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch, type CSSProperties } from "vue";
import { ChevronRight, Terminal } from "@lucide/vue";
import type { ToolActivity } from "@/types/chat";
import { useSettingStore } from "@/stores/setting";
import { parseAnsi, type AnsiSpan } from "@/services/chat/ansi";
import { parseShellResult } from "@/services/chat/shellResult";
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

const waitingLabel = computed(() => (settingStore.language === "zh-CN" ? "运行中…" : "Running…"));
const command = computed(() => String(props.activity.arguments?.command ?? "").trim());
const rawOutput = computed(() =>
  (props.activity.result ?? extractOutputFromDetail(props.activity.detail)).trim(),
);
const parsed = computed(() => parseShellResult(rawOutput.value));

const stdoutSpans = computed<AnsiSpan[]>(() => {
  if (!parsed.value.stdout) return [];
  return parseAnsi(parsed.value.stdout);
});

const stderrText = computed(() => parsed.value.stderr.trimEnd());

const outputSpans = computed<AnsiSpan[]>(() => {
  if (!rawOutput.value) return [];
  return parseAnsi(rawOutput.value);
});

const isOk = computed(() => {
  if (status.value === "running" || status.value === "error") return false;
  if (parsed.value.exitCode != null) return parsed.value.exitCode === 0;
  return status.value === "done";
});

const isFail = computed(() => {
  if (status.value === "error") return true;
  if (parsed.value.exitCode != null) return parsed.value.exitCode !== 0;
  return false;
});

const exitLabel = computed(() => {
  if (parsed.value.exitCode == null) return "";
  return `${parsed.value.exitCode}`;
});

const durationLabel = computed(() => parsed.value.duration ?? "");

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
.shell-terminal-card {
  --term-bg: #111418;
  --term-header: #161a1f;
  --term-border: color-mix(in srgb, #fff 8%, transparent);
  --term-fg: #d7dde5;
  --term-muted: #8b939e;
  --term-green: #5ecf7a;
  --term-red: #ff6b6b;
  width: 100%;
  box-sizing: border-box;
  margin: 4px 0 8px;
  border: 1px solid var(--term-border);
  border-radius: 10px;
  background: var(--term-bg);
  overflow: hidden;
}
.shell-terminal-card.running {
  border-color: color-mix(in srgb, var(--peek-accent) 35%, var(--term-border));
}
.shell-terminal-card.fail {
  border-color: color-mix(in srgb, var(--term-red) 28%, var(--term-border));
}
.shell-terminal-header {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  min-height: 30px;
  margin: 0;
  padding: 5px 10px;
  border: 0;
  border-bottom: 1px solid var(--term-border);
  background: var(--term-header);
  color: var(--term-fg);
  font: inherit;
  font-size: 12px;
  line-height: 18px;
  text-align: left;
  cursor: pointer;
}
.shell-terminal-card.collapsed .shell-terminal-header {
  border-bottom: 0;
}
.shell-terminal-chevron {
  flex: none;
  color: var(--term-muted);
  transition: transform 120ms ease;
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
  font-weight: 600;
  color: var(--term-fg);
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
.shell-terminal-meta {
  flex: none;
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--term-muted);
}
.shell-terminal-status {
  flex: none;
  font-family: var(--font-mono);
  font-size: 10px;
  font-weight: 650;
}
.shell-terminal-status.running {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--peek-accent);
  animation: shell-dot 1.2s ease-out infinite;
}
.shell-terminal-status.ok {
  color: var(--term-green);
}
.shell-terminal-status.fail {
  color: var(--term-red);
}
.shell-terminal-screen {
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.5;
  font-variant-ligatures: none;
}
.shell-terminal-cmdline {
  display: flex;
  gap: 8px;
  padding: 8px 12px 2px;
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
}
.shell-terminal-body {
  margin: 0;
  max-height: var(--agent-card-max-height, 220px);
  overflow: auto;
  padding: 4px 12px 10px;
  color: var(--term-fg);
  font: inherit;
  white-space: pre-wrap;
  word-break: break-word;
}
.shell-terminal-body.stderr {
  color: color-mix(in srgb, var(--term-red) 70%, var(--term-fg));
}
.shell-terminal-body.muted {
  color: var(--term-muted);
}
.shell-terminal-body code {
  font: inherit;
  color: inherit;
  background: transparent;
}
.shell-terminal-cursor {
  display: inline-block;
  width: 6px;
  height: 1em;
  margin-left: 2px;
  vertical-align: text-bottom;
  background: color-mix(in srgb, var(--peek-accent) 70%, #fff);
  animation: shell-blink 1s steps(1) infinite;
}
@keyframes shell-dot {
  0%,
  100% {
    opacity: 0.45;
  }
  50% {
    opacity: 1;
  }
}
@keyframes shell-blink {
  50% {
    opacity: 0;
  }
}
@media (prefers-reduced-motion: reduce) {
  .shell-terminal-status.running,
  .shell-terminal-cursor {
    animation: none;
  }
}
</style>
