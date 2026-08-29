<template>
  <section class="settings-page is-wide usage-page">
    <SettingsPageHeader :title="copy.title">
      <template #actions>
        <Select v-model="range" @update:model-value="applyRange">
          <SelectTrigger class="range-select"><SelectValue /></SelectTrigger>
          <SelectContent position="popper">
            <SelectItem v-for="item in rangeOptions" :key="item.value" :value="item.value">
              {{ item.label }}
            </SelectItem>
          </SelectContent>
        </Select>
        <button
          type="button"
          class="icon-button"
          :title="copy.refresh"
          :aria-label="copy.refresh"
          @click="load"
        >
          <RefreshCw :size="14" :class="{ spinning: loading }" />
        </button>
      </template>
    </SettingsPageHeader>

    <div class="filter-row">
      <div class="settings-seg" role="group">
        <button
          v-for="item in granularityOptions"
          :key="item.value"
          type="button"
          :class="{ on: granularity === item.value }"
          @click="setGranularity(item.value)"
        >
          {{ item.label }}
        </button>
      </div>
      <div v-if="range === 'custom'" class="custom-range">
        <label>
          <span>{{ copy.from }}</span>
          <input v-model="customFrom" type="date" @change="load" />
        </label>
        <label>
          <span>{{ copy.to }}</span>
          <input v-model="customTo" type="date" @change="load" />
        </label>
      </div>
    </div>

    <div v-if="error" class="settings-form-error">
      <CircleAlert :size="16" class="inline mr-1.5 align-text-bottom" />
      <span>{{ copy.error }}: {{ error }}</span>
    </div>
    <div v-else-if="loading" class="settings-empty">
      <span class="loader" />
      {{ copy.loading }}
    </div>

    <template v-else>
      <section class="settings-card summary-strip" aria-label="Token summary">
        <div class="summary-item total">
          <span>{{ copy.total }}</span>
          <strong>{{ format(report.total.totalTokens) }}</strong>
          <small>{{ copy.calls }}</small>
        </div>
        <div class="summary-item">
          <span>{{ copy.input }}</span>
          <strong>{{ format(report.total.inputTokens) }}</strong>
          <small>{{ share(report.total.inputTokens) }}</small>
        </div>
        <div class="summary-item">
          <span>{{ copy.output }}</span>
          <strong>{{ format(report.total.outputTokens) }}</strong>
          <small>{{ share(report.total.outputTokens) }}</small>
        </div>
        <div class="summary-item">
          <span>{{ copy.accuracy }}</span>
          <strong class="accuracy">{{ accuracyLabel }}</strong>
          <small>{{ copy.accuracyHint }}</small>
        </div>
      </section>

      <div v-if="report.modelCalls === 0" class="empty-state">
        <BarChart3 :size="25" />
        <strong>{{ copy.emptyTitle }}</strong>
        <span>{{ copy.emptyDescription }}</span>
      </div>

      <template v-else>
        <section class="data-section timeline-section">
          <header class="section-header">
            <div>
              <h2>{{ copy.timeline }}</h2>
              <p>{{ copy.timelineHint }}</p>
            </div>
            <div class="legend">
              <span v-for="series in modelSeries" :key="series.id">
                <i :style="{ background: series.color }" />
                {{ series.label }}
              </span>
            </div>
          </header>
          <div class="chart-wrap">
            <svg
              viewBox="0 0 760 210"
              preserveAspectRatio="none"
              class="usage-chart"
              role="img"
              @mouseleave="hoveredPoint = null"
            >
              <line
                v-for="y in [18, 64, 110, 156, 202]"
                :key="y"
                x1="4"
                :y1="y"
                x2="756"
                :y2="y"
                class="grid-line"
              />
              <g v-for="series in modelSeries" :key="series.id" class="model-series">
                <path
                  v-if="series.points.length > 1"
                  :d="series.path"
                  :stroke="series.color"
                  class="series-line"
                />
                <circle
                  v-for="point in series.points"
                  :key="`${series.id}:${point.index}`"
                  :cx="point.x"
                  :cy="point.y"
                  r="9"
                  class="point-target"
                  tabindex="0"
                  @mouseenter="
                    hoveredPoint = { ...point, model: series.label, color: series.color }
                  "
                  @focus="hoveredPoint = { ...point, model: series.label, color: series.color }"
                  @blur="hoveredPoint = null"
                />
                <circle
                  v-if="series.points.length === 1"
                  :cx="series.points[0].x"
                  :cy="series.points[0].y"
                  r="3"
                  :fill="series.color"
                  class="single-point"
                />
              </g>
            </svg>
            <div
              v-if="hoveredPoint"
              class="chart-tooltip"
              :class="{ start: hoveredPoint.x < 90, end: hoveredPoint.x > 670 }"
              :style="{
                left: `${(hoveredPoint.x / 760) * 100}%`,
                top: `${(hoveredPoint.y / 210) * 198}px`,
              }"
            >
              <span>
                <i :style="{ background: hoveredPoint.color }" />
                {{ hoveredPoint.model }}
              </span>
              <strong>{{ format(hoveredPoint.value) }}</strong>
              <small>{{ hoveredPoint.label }}</small>
            </div>
            <div class="chart-labels">
              <span
                v-for="label in chartLabels"
                :key="`${label.label}:${label.index}`"
                :style="{ left: `${(label.x / 760) * 100}%` }"
              >
                {{ label.label }}
              </span>
            </div>
          </div>
        </section>

        <div class="detail-grid">
          <section class="data-section">
            <header class="section-header">
              <div>
                <h2>{{ copy.models }}</h2>
                <p>{{ copy.modelsHint }}</p>
              </div>
            </header>
            <div class="model-list">
              <div
                v-for="model in report.byModel"
                :key="`${model.provider}:${model.model}`"
                class="model-row"
              >
                <div class="model-meta">
                  <span>
                    <strong>{{ model.model }}</strong>
                    <small v-if="model.provider">{{ model.provider }}</small>
                  </span>
                  <span>
                    {{ format(model.usage.totalTokens) }} · {{ Math.round(model.share * 100) }}%
                  </span>
                </div>
                <div class="track"><span :style="{ width: `${model.share * 100}%` }" /></div>
              </div>
            </div>
          </section>

          <section class="data-section">
            <header class="section-header">
              <div>
                <h2>{{ copy.breakdown }}</h2>
                <p>{{ copy.breakdownHint }}</p>
              </div>
            </header>
            <div class="breakdown-list">
              <div v-for="item in breakdown" :key="item.label">
                <span>
                  <i :class="item.class" />
                  {{ item.label }}
                </span>
                <strong>{{ format(item.value) }}</strong>
              </div>
            </div>
          </section>
        </div>
      </template>
    </template>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { BarChart3, CircleAlert, RefreshCw } from "@lucide/vue";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import SettingsPageHeader from "@/components/settings/SettingsPageHeader.vue";
import { getTokenUsageReport } from "@/services/ipc";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import type { TokenUsageReport } from "@/types/tokenUsage";

type Granularity = "day" | "week" | "month";
const settingStore = useSettingStore();
const loading = ref(true);
const error = ref("");
const range = ref("30d");
const granularity = ref<Granularity>("day");
const customFrom = ref("");
const customTo = ref("");
const hoveredPoint = ref<ChartHoverPoint | null>(null);

type ChartPoint = { index: number; x: number; y: number; value: number; label: string };
type ChartHoverPoint = ChartPoint & { model: string; color: string };
const CHART_COLORS = [
  "#2563eb",
  "#0f766e",
  "#b45309",
  "#be185d",
  "#7c3aed",
  "#4f6f52",
  "#c2410c",
  "#0369a1",
];
const CHART_TOP = 18;
const CHART_BOTTOM = 202;
const CHART_LEFT = 8;
const CHART_RIGHT = 752;

const emptyUsage = (): TokenUsageReport => ({
  from: 0,
  to: 0,
  granularity: "day",
  modelCalls: 0,
  total: {
    inputTokens: 0,
    outputTokens: 0,
    systemTokens: 0,
    contextTokens: 0,
    toolCallTokens: 0,
    toolResultTokens: 0,
    memoryTokens: 0,
    totalTokens: 0,
    accuracy: "estimated",
  },
  byModel: [],
  timeline: [],
});
const report = ref<TokenUsageReport>(emptyUsage());

const copy = computed(() => {
  const language = settingStore.language;
  return {
    title: tr(language, "usage.title"),
    from: tr(language, "usage.from"),
    to: tr(language, "usage.to"),
    refresh: tr(language, "usage.refresh"),
    loading: tr(language, "usage.loading"),
    error: tr(language, "usage.error"),
    emptyTitle: tr(language, "usage.empty.title"),
    emptyDescription: tr(language, "usage.empty.description"),
    total: tr(language, "usage.total"),
    calls: tr(language, "usage.calls", { count: report.value.modelCalls }),
    input: tr(language, "usage.input"),
    output: tr(language, "usage.output"),
    accuracy: tr(language, "usage.accuracy"),
    accuracyHint: tr(language, "usage.accuracyHint"),
    timeline: tr(language, "usage.timeline"),
    timelineHint: tr(language, "usage.timelineHint", {
      granularity: tr(language, `usage.granularity.${granularity.value}`),
    }),
    models: tr(language, "usage.models"),
    modelsHint: tr(language, "usage.modelsHint"),
    breakdown: tr(language, "usage.breakdown"),
    breakdownHint: tr(language, "usage.breakdownHint"),
    tools: tr(language, "usage.tools"),
    context: tr(language, "usage.context"),
    system: tr(language, "usage.system"),
    memory: tr(language, "usage.memory"),
    reasoning: tr(language, "usage.reasoning"),
    cacheRead: tr(language, "usage.cacheRead"),
  };
});

const rangeOptions = computed(() =>
  ["today", "7d", "30d", "month", "custom"].map((value) => ({
    value,
    label: tr(settingStore.language, `usage.range.${value}` as "usage.range.today"),
  })),
);
const granularityOptions = computed(() =>
  (["day", "week", "month"] as Granularity[]).map((value) => ({
    value,
    label: tr(settingStore.language, `usage.granularity.${value}`),
  })),
);
const accuracyLabel = computed(() =>
  tr(
    settingStore.language,
    report.value.total.accuracy === "exact"
      ? "runtime.accuracy.exact"
      : report.value.total.accuracy === "mixed"
        ? "runtime.accuracy.mixed"
        : "runtime.accuracy.estimated",
  ),
);

const bounds = computed(() => {
  const now = new Date();
  let from = new Date(now);
  if (range.value === "today") from.setHours(0, 0, 0, 0);
  else if (range.value === "7d") from.setDate(now.getDate() - 7);
  else if (range.value === "30d") from.setDate(now.getDate() - 30);
  else if (range.value === "month") from = new Date(now.getFullYear(), now.getMonth(), 1);
  else if (customFrom.value) from = new Date(`${customFrom.value}T00:00:00`);
  const to =
    range.value === "custom" && customTo.value ? new Date(`${customTo.value}T23:59:59`) : now;
  return { from: from.getTime(), to: to.getTime() + 1 };
});

function smoothPath(points: ChartPoint[]) {
  if (!points.length) return "";
  if (points.length === 1) return `M ${points[0].x} ${points[0].y}`;
  let path = `M ${points[0].x} ${points[0].y}`;
  for (let index = 0; index < points.length - 1; index += 1) {
    const p0 = points[Math.max(0, index - 1)];
    const p1 = points[index];
    const p2 = points[index + 1];
    const p3 = points[Math.min(points.length - 1, index + 2)];
    const cp1x = p1.x + (p2.x - p0.x) / 6;
    const cp1y = Math.min(CHART_BOTTOM, Math.max(CHART_TOP, p1.y + (p2.y - p0.y) / 6));
    const cp2x = p2.x - (p3.x - p1.x) / 6;
    const cp2y = Math.min(CHART_BOTTOM, Math.max(CHART_TOP, p2.y - (p3.y - p1.y) / 6));
    path += ` C ${cp1x} ${cp1y}, ${cp2x} ${cp2y}, ${p2.x} ${p2.y}`;
  }
  return path;
}

const modelSeries = computed(() => {
  const timeline = report.value.timeline;
  const max = Math.max(...timeline.flatMap((bucket) => Object.values(bucket.models)), 1);
  const span = Math.max(timeline.length - 1, 1);
  return report.value.byModel.map((model, modelIndex) => {
    const points = timeline.map((bucket, index): ChartPoint => ({
      index,
      x:
        timeline.length === 1
          ? (CHART_LEFT + CHART_RIGHT) / 2
          : CHART_LEFT + (index / span) * (CHART_RIGHT - CHART_LEFT),
      y: CHART_BOTTOM - ((bucket.models[model.model] ?? 0) / max) * (CHART_BOTTOM - CHART_TOP),
      value: bucket.models[model.model] ?? 0,
      label: bucket.label,
    }));
    return {
      id: `${model.provider ?? ""}:${model.model}`,
      label: model.model,
      color: CHART_COLORS[modelIndex % CHART_COLORS.length],
      points,
      path: smoothPath(points),
    };
  });
});

const chartLabels = computed(() => {
  const timeline = report.value.timeline;
  if (!timeline.length) return [];
  const step = Math.max(1, Math.ceil((timeline.length - 1) / 6));
  const indexes = timeline.map((_, index) => index).filter((index) => index % step === 0);
  if (indexes[indexes.length - 1] !== timeline.length - 1) indexes.push(timeline.length - 1);
  const span = Math.max(timeline.length - 1, 1);
  return indexes.map((index) => ({
    index,
    label: timeline[index].label,
    x:
      timeline.length === 1
        ? (CHART_LEFT + CHART_RIGHT) / 2
        : CHART_LEFT + (index / span) * (CHART_RIGHT - CHART_LEFT),
  }));
});

const breakdown = computed(() => {
  const items = [
    { label: copy.value.input, value: report.value.total.inputTokens, class: "input-dot" },
    { label: copy.value.output, value: report.value.total.outputTokens, class: "output-dot" },
    {
      label: copy.value.tools,
      value: report.value.total.toolCallTokens + report.value.total.toolResultTokens,
      class: "tool-dot",
    },
    { label: copy.value.context, value: report.value.total.contextTokens, class: "context-dot" },
    { label: copy.value.system, value: report.value.total.systemTokens, class: "system-dot" },
    { label: copy.value.memory, value: report.value.total.memoryTokens, class: "memory-dot" },
  ];
  // Provider-reported sub-metrics (DeepSeek only); omit when the provider
  // didn't report them so non-DeepSeek usage is unchanged.
  const reasoning = report.value.total.reasoningTokens ?? 0;
  const cacheRead = report.value.total.cacheReadTokens ?? 0;
  if (reasoning > 0)
    items.push({ label: copy.value.reasoning, value: reasoning, class: "reasoning-dot" });
  if (cacheRead > 0)
    items.push({ label: copy.value.cacheRead, value: cacheRead, class: "cache-dot" });
  return items;
});

const format = (value: number) => new Intl.NumberFormat(settingStore.language).format(value);
const share = (value: number) =>
  tr(settingStore.language, "usage.share", {
    value: report.value.total.totalTokens
      ? Math.round((value / report.value.total.totalTokens) * 100)
      : 0,
  });

async function load() {
  loading.value = true;
  error.value = "";
  try {
    report.value = await getTokenUsageReport({
      from: bounds.value.from,
      to: bounds.value.to,
      granularity: granularity.value,
    });
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}
function applyRange() {
  void load();
}
function setGranularity(value: Granularity) {
  granularity.value = value;
  void load();
}
onMounted(() => void load());
</script>

<style scoped>
.usage-page {
  color: var(--peek-text);
}
.section-header p {
  margin: 4px 0 0;
  color: var(--peek-muted);
  font-size: 11px;
  line-height: 17px;
}
.range-select {
  min-width: 132px;
}
.icon-button {
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  border: 1px solid var(--peek-border);
  border-radius: 6px;
  background: transparent;
  color: var(--peek-muted);
}
.icon-button:hover {
  background: var(--peek-hover-bg);
  color: var(--peek-text);
}
.filter-row,
.custom-range,
.legend,
.legend span {
  display: flex;
  align-items: center;
}
.filter-row {
  min-height: 48px;
  justify-content: space-between;
  gap: 12px;
  border-bottom: 1px solid var(--peek-border);
}
.filter-row .settings-seg {
  width: auto;
}
.custom-range {
  gap: 8px;
}
.custom-range label {
  display: flex;
  align-items: center;
  gap: 5px;
  color: var(--peek-muted);
  font-size: 10px;
}
.custom-range input {
  height: 28px;
  padding: 0 6px;
  border: 1px solid var(--peek-border);
  border-radius: 5px;
  background: transparent;
  color: var(--peek-text);
  font-size: 10px;
}
.loader {
  width: 15px;
  height: 15px;
  border: 2px solid var(--peek-border);
  border-top-color: var(--peek-text);
  border-radius: 50%;
  animation: spin 700ms linear infinite;
}
.summary-strip {
  display: grid;
  grid-template-columns: 1.35fr repeat(3, 1fr);
  margin-bottom: 18px;
  overflow: hidden;
}
.summary-item {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding: 15px 14px;
  border-right: 1px solid var(--peek-border);
}
.summary-item:last-child {
  border-right: 0;
}
.summary-item > span,
.summary-item small {
  color: var(--peek-muted);
  font-size: 10px;
}
.summary-item strong {
  overflow: hidden;
  font-size: 19px;
  font-weight: 650;
  text-overflow: ellipsis;
}
.summary-item.total strong {
  font-size: 23px;
}
.summary-item .accuracy {
  font-size: 14px;
  text-transform: capitalize;
}
.empty-state {
  min-height: 280px;
  flex-direction: column;
  gap: 7px;
}
.empty-state svg {
  margin-bottom: 5px;
  color: var(--peek-faint);
}
.empty-state strong {
  color: var(--peek-text);
  font-size: 13px;
}
.data-section {
  min-width: 0;
  padding-top: 18px;
}
.timeline-section {
  border-bottom: 1px solid var(--peek-border);
}
.section-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 12px;
}
.section-header h2 {
  margin: 0;
  font-size: 12px;
  font-weight: 650;
}
.legend {
  max-width: 58%;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 5px 12px;
  color: var(--peek-muted);
  font-size: 10px;
}
.legend span {
  gap: 5px;
}
.legend i {
  width: 14px;
  height: 2px;
  display: inline-block;
  border-radius: 1px;
}
.chart-wrap {
  position: relative;
  height: 225px;
}
.usage-chart {
  width: 100%;
  height: 198px;
  display: block;
}
.grid-line {
  stroke: color-mix(in srgb, var(--peek-border) 75%, transparent);
  stroke-width: 1;
}
.series-line {
  fill: none;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
  vector-effect: non-scaling-stroke;
}
.point-target {
  fill: transparent;
  outline: none;
  cursor: crosshair;
}
.point-target:focus {
  stroke: var(--peek-surface);
  stroke-width: 2;
}
.single-point {
  pointer-events: none;
  vector-effect: non-scaling-stroke;
}
.chart-labels {
  position: relative;
  height: 18px;
  color: var(--peek-faint);
  font-size: 9px;
}
.chart-labels span {
  position: absolute;
  max-width: 70px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  transform: translateX(-50%);
}
.chart-labels span:first-child {
  transform: none;
}
.chart-labels span:last-child {
  transform: translateX(-100%);
}
.chart-tooltip {
  position: absolute;
  z-index: 2;
  min-width: 116px;
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 3px 10px;
  padding: 7px 9px;
  border: 1px solid var(--peek-border);
  border-radius: 6px;
  background: var(--peek-surface);
  color: var(--peek-text);
  box-shadow: 0 6px 18px color-mix(in srgb, #000 16%, transparent);
  pointer-events: none;
  transform: translate(-50%, calc(-100% - 9px));
}
.chart-tooltip.start {
  transform: translate(0, calc(-100% - 9px));
}
.chart-tooltip.end {
  transform: translate(-100%, calc(-100% - 9px));
}
.chart-tooltip span {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  overflow: hidden;
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.chart-tooltip i {
  width: 7px;
  height: 7px;
  flex: 0 0 auto;
  border-radius: 50%;
}
.chart-tooltip strong {
  font-size: 11px;
  font-weight: 650;
}
.chart-tooltip small {
  grid-column: 1 / -1;
  color: var(--peek-muted);
  font-size: 9px;
}
.detail-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.2fr) minmax(220px, 0.8fr);
  gap: 24px;
}
.model-list,
.breakdown-list {
  border-top: 1px solid var(--peek-border);
}
.model-row {
  padding: 10px 2px;
  border-bottom: 1px solid var(--peek-border);
}
.model-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  font-size: 10px;
}
.model-meta > span:first-child {
  min-width: 0;
  display: flex;
  align-items: baseline;
  gap: 7px;
}
.model-meta strong {
  overflow: hidden;
  font-size: 11px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.model-meta small,
.model-meta > span:last-child {
  color: var(--peek-muted);
  white-space: nowrap;
}
.track {
  height: 3px;
  margin-top: 7px;
  overflow: hidden;
  border-radius: 2px;
  background: color-mix(in srgb, var(--peek-text) 8%, transparent);
}
.track span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--peek-accent);
}
.breakdown-list > div {
  min-height: 38px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  border-bottom: 1px solid var(--peek-border);
  font-size: 11px;
}
.breakdown-list span {
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--peek-muted);
}
.breakdown-list strong {
  font-size: 11px;
  font-weight: 600;
}
.input-dot,
.output-dot,
.tool-dot,
.context-dot,
.system-dot,
.memory-dot,
.reasoning-dot,
.cache-dot {
  width: 6px;
  height: 6px;
  display: inline-block;
  border-radius: 50%;
  background: var(--peek-text);
}
.output-dot {
  background: color-mix(in srgb, var(--peek-accent) 55%, var(--peek-muted));
}
.tool-dot {
  background: #7c3aed;
}
.context-dot {
  background: #0f766e;
}
.system-dot {
  background: #b45309;
}
.memory-dot {
  background: #be185d;
}
.reasoning-dot {
  background: #c2410c;
}
.cache-dot {
  background: #0369a1;
}
.spinning {
  animation: spin 700ms linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
@media (max-width: 700px) {
  .summary-strip {
    grid-template-columns: 1fr 1fr;
  }
  .summary-item:nth-child(2) {
    border-right: 0;
  }
  .summary-item:nth-child(-n + 2) {
    border-bottom: 1px solid var(--peek-border);
  }
  .detail-grid {
    grid-template-columns: 1fr;
    gap: 0;
  }
  .filter-row {
    align-items: flex-start;
    flex-direction: column;
    padding: 9px 0;
  }
  .custom-range {
    flex-wrap: wrap;
  }
}
</style>
