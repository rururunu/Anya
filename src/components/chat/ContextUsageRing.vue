<template>
  <PopoverRoot>
    <PopoverTrigger as-child>
      <button
        type="button"
        class="context-usage-ring"
        :class="tone"
        data-tauri-drag-region="false"
        :aria-label="heading"
        :aria-valuemin="0"
        :aria-valuemax="100"
        :aria-valuenow="percent"
      >
        <svg
          class="context-usage-ring-svg"
          :width="size"
          :height="size"
          :viewBox="`0 0 ${size} ${size}`"
          aria-hidden="true"
        >
          <circle
            class="track"
            :cx="center"
            :cy="center"
            :r="radius"
            fill="none"
            :stroke-width="stroke"
          />
          <circle
            class="progress"
            :cx="center"
            :cy="center"
            :r="radius"
            fill="none"
            :stroke-width="stroke"
            :stroke-dasharray="circumference"
            :stroke-dashoffset="dashOffset"
            stroke-linecap="round"
            :transform="`rotate(-90 ${center} ${center})`"
          />
        </svg>
      </button>
    </PopoverTrigger>
    <PopoverPortal>
      <PopoverContent
        class="context-usage-card"
        side="top"
        align="end"
        :side-offset="8"
        :collision-padding="12"
        data-tauri-drag-region="false"
      >
        <header class="context-usage-card-header">
          <span>{{ heading }}</span>
          <span class="context-usage-card-total">~{{ usedLabel }} / {{ totalLabel }}</span>
        </header>
        <div class="context-usage-bar" aria-hidden="true">
          <span
            v-for="segment in segments"
            :key="segment.key"
            class="context-usage-bar-seg"
            :class="segment.key"
            :style="{ width: `${segment.barPercent}%` }"
          />
        </div>
        <ul class="context-usage-list">
          <li v-for="segment in segments" :key="segment.key">
            <span class="context-usage-swatch" :class="segment.key" />
            <span>{{ segment.label }}</span>
            <span class="context-usage-count">~{{ segment.countLabel }}</span>
          </li>
        </ul>
        <PopoverArrow class="context-usage-arrow" />
      </PopoverContent>
    </PopoverPortal>
  </PopoverRoot>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { PopoverArrow, PopoverContent, PopoverPortal, PopoverRoot, PopoverTrigger } from "reka-ui";
import { storeToRefs } from "pinia";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import { formatTokenCount } from "@/services/chat/tokenEstimate";
import type { ContextUsageSnapshot } from "@/types/chat";

const props = withDefaults(
  defineProps<{
    usage: ContextUsageSnapshot;
    size?: number;
  }>(),
  {
    size: 18,
  },
);

const settingStore = useSettingStore();
const { language } = storeToRefs(settingStore);

const stroke = 2;
const center = computed(() => props.size / 2);
const radius = computed(() => (props.size - stroke) / 2);
const circumference = computed(() => 2 * Math.PI * radius.value);
const clampedRatio = computed(() => Math.max(0, Math.min(props.usage.usageRatio, 1)));
const dashOffset = computed(() => circumference.value * (1 - clampedRatio.value));
const percent = computed(() => Math.round(clampedRatio.value * 100));
const heading = computed(() =>
  tr(language.value, "context.usedHeading", { percent: percent.value }),
);
const usedLabel = computed(() => formatTokenCount(props.usage.estimatedTokens, language.value));
const totalLabel = computed(() =>
  formatTokenCount(props.usage.contextWindowTokens, language.value),
);

const segments = computed(() => {
  const windowTokens = Math.max(1, props.usage.contextWindowTokens);
  const items = [
    {
      key: "system",
      label: tr(language.value, "context.breakdownSystem"),
      tokens: Math.max(0, props.usage.systemPromptTokens ?? 0),
    },
    {
      key: "tools",
      label: tr(language.value, "context.breakdownTools"),
      tokens: Math.max(0, props.usage.toolsTokens ?? 0),
    },
    {
      key: "environment",
      label: tr(language.value, "context.breakdownEnvironment"),
      tokens: Math.max(0, props.usage.environmentTokens ?? 0),
    },
    {
      key: "rules",
      label: tr(language.value, "context.breakdownRules"),
      tokens: Math.max(0, props.usage.rulesTokens ?? 0),
    },
    {
      key: "skills",
      label: tr(language.value, "context.breakdownSkills"),
      tokens: Math.max(0, props.usage.skillsTokens ?? 0),
    },
    {
      key: "mcp",
      label: tr(language.value, "context.breakdownMcp"),
      tokens: Math.max(0, props.usage.mcpTokens ?? 0),
    },
    {
      key: "subagents",
      label: tr(language.value, "context.breakdownSubagents"),
      tokens: Math.max(0, props.usage.subagentTokens ?? 0),
    },
    {
      key: "memories",
      label: tr(language.value, "context.breakdownMemories"),
      tokens: Math.max(0, props.usage.memoriesTokens ?? 0),
    },
    {
      key: "summarized",
      label: tr(language.value, "context.breakdownSummarized"),
      tokens: Math.max(0, props.usage.summarizedTokens ?? 0),
    },
    {
      key: "messages",
      label: tr(language.value, "context.breakdownMessages"),
      tokens: Math.max(0, props.usage.messageTokens ?? 0),
    },
  ];
  if (items.every((item) => item.tokens <= 0) && props.usage.estimatedTokens > 0) {
    items[items.length - 1]!.tokens = props.usage.estimatedTokens;
  }
  return items
    .filter((item) => item.tokens > 0)
    .map((item) => ({
      ...item,
      barPercent: Math.max(0.6, (item.tokens / windowTokens) * 100),
      countLabel: formatTokenCount(item.tokens, language.value),
    }));
});

const tone = computed(() => {
  if (props.usage.usageRatio >= 0.9) return "critical";
  if (props.usage.usageRatio >= 0.7) return "warn";
  return "normal";
});
</script>

<style scoped>
.context-usage-ring {
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  padding: 0;
  border: 0;
  background: transparent;
  cursor: pointer;
}

.context-usage-ring:focus-visible {
  outline: none;
  box-shadow: var(--peek-focus-ring);
  border-radius: 999px;
}

.context-usage-ring-svg {
  display: block;
}

.track {
  stroke: color-mix(in srgb, var(--peek-muted) 28%, transparent);
}

.progress {
  transition:
    stroke-dashoffset 220ms ease,
    stroke 180ms ease;
}

.context-usage-ring.normal .progress {
  stroke: color-mix(in srgb, var(--peek-accent) 72%, var(--peek-muted));
}

.context-usage-ring.warn .progress {
  stroke: #f59e0b;
}

.context-usage-ring.critical .progress {
  stroke: var(--destructive);
}
</style>

<style>
.context-usage-card {
  z-index: 80;
  width: min(300px, calc(100vw - 24px));
  padding: 12px 14px 10px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--popover);
  color: var(--popover-foreground);
  box-shadow: var(--peek-shadow, 0 10px 30px rgb(0 0 0 / 12%));
  outline: none;
}

.context-usage-arrow {
  fill: var(--popover);
}

.context-usage-card-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
  font-size: 12px;
  font-weight: 600;
}

.context-usage-card-total {
  color: var(--peek-muted);
  font-weight: 500;
  font-variant-numeric: tabular-nums;
}

.context-usage-bar {
  display: flex;
  overflow: hidden;
  height: 6px;
  margin-bottom: 10px;
  border-radius: 99px;
  background: color-mix(in srgb, var(--peek-muted) 16%, transparent);
}

.context-usage-bar-seg {
  display: block;
  height: 100%;
}

.context-usage-bar-seg.system,
.context-usage-swatch.system {
  background: color-mix(in srgb, var(--peek-muted) 72%, transparent);
}

.context-usage-bar-seg.tools,
.context-usage-swatch.tools {
  background: #a78bfa;
}

.context-usage-bar-seg.environment,
.context-usage-swatch.environment {
  background: #2dd4bf;
}

.context-usage-bar-seg.rules,
.context-usage-swatch.rules {
  background: #34d399;
}

.context-usage-bar-seg.skills,
.context-usage-swatch.skills {
  background: #fb923c;
}

.context-usage-bar-seg.mcp,
.context-usage-swatch.mcp {
  background: #c4b5fd;
}

.context-usage-bar-seg.subagents,
.context-usage-swatch.subagents {
  background: #7dd3fc;
}

.context-usage-bar-seg.memories,
.context-usage-swatch.memories {
  background: #fbbf24;
}

.context-usage-bar-seg.summarized,
.context-usage-swatch.summarized {
  background: #f9a8d4;
}

.context-usage-bar-seg.messages,
.context-usage-swatch.messages {
  background: #6366f1;
}

.context-usage-list {
  display: grid;
  gap: 2px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.context-usage-list li {
  display: grid;
  grid-template-columns: 8px 1fr auto;
  gap: 8px;
  align-items: center;
  margin: 0 -6px;
  padding: 4px 6px;
  border-radius: 6px;
  color: var(--peek-muted);
  font-size: 12px;
}

.context-usage-list li:hover {
  background: color-mix(in srgb, var(--peek-text) 6%, transparent);
}

.context-usage-swatch {
  width: 8px;
  height: 8px;
  border-radius: 2px;
}

.context-usage-count {
  color: var(--peek-text);
  font-variant-numeric: tabular-nums;
}
</style>
