<template>
  <div class="timeline-chart">
    <div class="timeline-tools">
      <div class="settings-seg chart-kind" role="group">
        <button
          v-for="item in kindOptions"
          :key="item.id"
          type="button"
          :class="{ on: kind === item.id }"
          :title="item.label"
          :aria-label="item.label"
          @click="kind = item.id"
        >
          <component :is="item.icon" :size="13" />
        </button>
      </div>
      <div class="legend">
        <span v-for="series in props.series" :key="series.id">
          <i :style="{ background: series.color }" />
          {{ series.label }}
        </span>
      </div>
    </div>
    <div class="chart-wrap">
      <div class="chart-y" aria-hidden="true">
        <span v-for="tick in yTicks" :key="tick.value" :style="{ top: `${tick.top}%` }">
          {{ tick.label }}
        </span>
      </div>
      <div class="chart-plot">
        <svg
          :viewBox="`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`"
          preserveAspectRatio="none"
          class="usage-chart"
          role="img"
          :aria-label="chartLabel"
          @mousemove="onChartMove"
          @mouseleave="hoveredBucket = null"
        >
          <line
            v-for="tick in yTicks"
            :key="`grid:${tick.value}`"
            :x1="CHART_LEFT"
            :y1="tick.y"
            :x2="CHART_RIGHT"
            :y2="tick.y"
            class="grid-line"
          />
          <template v-if="kind === 'stacked'">
            <g v-for="bar in stackedBars" :key="bar.index">
              <rect
                v-for="segment in bar.segments"
                :key="`${bar.index}:${segment.id}`"
                :x="bar.x"
                :y="segment.y"
                :width="bar.width"
                :height="segment.height"
                :fill="segment.color"
                class="bar-seg"
                :class="{ dim: hoveredBucket != null && hoveredBucket !== bar.index }"
              />
            </g>
          </template>
          <template v-else-if="kind === 'area'">
            <path
              v-for="item in areaSeries"
              :key="`area:${item.id}`"
              :d="item.area"
              :fill="item.color"
              class="area-fill"
            />
            <path
              v-for="item in areaSeries"
              :key="`area-line:${item.id}`"
              :d="item.path"
              :stroke="item.color"
              class="series-line"
            />
          </template>
          <template v-else>
            <path
              v-for="item in lineSeries"
              :key="`line:${item.id}`"
              :d="item.path"
              :stroke="item.color"
              class="series-line"
            />
            <circle
              v-for="point in lineDots"
              :key="`${point.id}:${point.x}`"
              :cx="point.x"
              :cy="point.y"
              r="2.4"
              :fill="point.color"
              class="line-dot"
            />
          </template>
          <line
            v-if="hoveredBar"
            class="hover-rule"
            :x1="hoveredBar.cx"
            :x2="hoveredBar.cx"
            :y1="CHART_TOP"
            :y2="CHART_BOTTOM"
          />
        </svg>
        <div
          v-if="hoveredBar"
          class="chart-tooltip"
          :class="{ start: hoveredBar.cx < 110, end: hoveredBar.cx > 650 }"
          :style="{ left: `${(hoveredBar.cx / CHART_WIDTH) * 100}%` }"
        >
          <small>{{ hoveredBar.label }}</small>
          <div class="tooltip-total">
            <span>{{ totalLabel }}</span>
            <strong>{{ format(hoveredBar.total) }}</strong>
          </div>
          <div v-for="row in hoveredRows" :key="row.id" class="tooltip-row">
            <span>
              <i :style="{ background: row.color }" />
              {{ row.label }}
            </span>
            <strong>{{ format(row.value) }}</strong>
          </div>
        </div>
      </div>
      <div class="chart-labels">
        <span
          v-for="label in chartLabels"
          :key="`${label.label}:${label.index}`"
          :style="{ left: `${(label.x / CHART_WIDTH) * 100}%` }"
        >
          {{ label.label }}
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { ChartArea, ChartColumnStacked, ChartLine } from "@lucide/vue";
import {
  CHART_BOTTOM,
  CHART_HEIGHT,
  CHART_LEFT,
  CHART_RIGHT,
  CHART_TOP,
  CHART_WIDTH,
  bucketIndexAt,
  buildLineSeries,
  buildStackedAreas,
  buildStackedBars,
  yAxisTicks,
  type ChartSeriesDef,
  type TimelineChartKind,
} from "@/services/usage/tokenUsageChart";
import type { TokenUsageReport } from "@/types/tokenUsage";

const props = defineProps<{
  series: ChartSeriesDef[];
  timeline: TokenUsageReport["timeline"];
  format: (value: number) => string;
  totalLabel: string;
  chartLabel: string;
  kindLabels: Record<TimelineChartKind, string>;
}>();

const kind = ref<TimelineChartKind>("stacked");
const hoveredBucket = ref<number | null>(null);

const kindOptions = computed(() =>
  (
    [
      { id: "stacked", icon: ChartColumnStacked },
      { id: "line", icon: ChartLine },
      { id: "area", icon: ChartArea },
    ] as const
  ).map((item) => ({ ...item, label: props.kindLabels[item.id] })),
);

const stacked = computed(() => buildStackedBars(props.timeline, props.series));
const stackedBars = computed(() => stacked.value.bars);
const lines = computed(() => buildLineSeries(props.timeline, props.series));
const areas = computed(() => buildStackedAreas(props.timeline, props.series));
const lineSeries = computed(() => lines.value.series);
const areaSeries = computed(() => areas.value.series);
const chartMax = computed(() => {
  if (kind.value === "line") return lines.value.max;
  if (kind.value === "area") return areas.value.max;
  return stacked.value.max;
});
const yTicks = computed(() =>
  yAxisTicks(chartMax.value).map((tick) => ({
    ...tick,
    top: (tick.y / CHART_HEIGHT) * 100,
  })),
);
const hoveredBar = computed(() => {
  if (hoveredBucket.value == null) return null;
  return stackedBars.value[hoveredBucket.value] ?? null;
});
const hoveredRows = computed(() => {
  const index = hoveredBucket.value;
  if (index == null) return [];
  const rows = props.series
    .map((item) => ({
      id: item.id,
      label: item.label,
      color: item.color,
      value: item.values[index] ?? 0,
    }))
    .filter((item) => item.value > 0);
  return kind.value === "line"
    ? [...rows].sort((a, b) => b.value - a.value)
    : rows.slice().reverse();
});
const chartLabels = computed(() => {
  const bars = stackedBars.value;
  if (!bars.length) return [];
  const step = Math.max(1, Math.ceil((bars.length - 1) / 6));
  const indexes = bars.map((_, index) => index).filter((index) => index % step === 0);
  if (indexes[indexes.length - 1] !== bars.length - 1) indexes.push(bars.length - 1);
  return indexes.map((index) => ({
    index,
    label: bars[index]?.label ?? "",
    x: bars[index]?.cx ?? 0,
  }));
});
const lineDots = computed(() => {
  if (kind.value !== "line" || (lineSeries.value[0]?.points.length ?? 0) > 24) return [];
  return lineSeries.value.flatMap((item) =>
    item.points.map((point) => ({ ...point, id: item.id, color: item.color })),
  );
});

function onChartMove(event: MouseEvent) {
  const svg = event.currentTarget as SVGSVGElement;
  const rect = svg.getBoundingClientRect();
  if (rect.width <= 0) return;
  const x = ((event.clientX - rect.left) / rect.width) * CHART_WIDTH;
  hoveredBucket.value = bucketIndexAt(x, stackedBars.value.length);
}
</script>

<style scoped>
.timeline-tools,
.legend,
.legend span {
  display: flex;
  align-items: center;
}
.timeline-tools {
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}
.chart-kind {
  width: auto;
  flex: none;
}
.chart-kind button {
  width: 30px;
  flex: none;
  display: grid;
  place-items: center;
  padding: 0;
}
.legend {
  min-width: 0;
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
  width: 8px;
  height: 8px;
  display: inline-block;
  border-radius: 2px;
}
.chart-wrap {
  position: relative;
  display: grid;
  grid-template-columns: 44px minmax(0, 1fr);
  grid-template-rows: 198px 18px;
  column-gap: 8px;
}
.chart-y {
  position: relative;
  grid-row: 1;
  color: var(--peek-faint);
  font-size: 9px;
  line-height: 1;
}
.chart-y span {
  position: absolute;
  right: 0;
  transform: translateY(-50%);
}
.chart-plot {
  position: relative;
  grid-row: 1;
  min-width: 0;
}
.usage-chart {
  width: 100%;
  height: 198px;
  display: block;
  cursor: crosshair;
}
.grid-line {
  stroke: color-mix(in srgb, var(--peek-border) 75%, transparent);
  stroke-width: 1;
}
.bar-seg,
.area-fill,
.series-line {
  transition: opacity 120ms ease;
}
.bar-seg.dim,
.area-fill.dim,
.series-line.dim {
  opacity: 0.45;
}
.area-fill {
  fill-opacity: 0.28;
}
.series-line {
  fill: none;
  stroke-width: 2;
  stroke-linejoin: round;
  stroke-linecap: round;
  vector-effect: non-scaling-stroke;
}
.line-dot {
  vector-effect: non-scaling-stroke;
}
.hover-rule {
  stroke: color-mix(in srgb, var(--peek-text) 28%, transparent);
  stroke-width: 1;
  stroke-dasharray: 3 3;
  pointer-events: none;
}
.chart-labels {
  grid-column: 2;
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
.chart-labels span:first-child:not(:last-child) {
  transform: none;
}
.chart-labels span:last-child:not(:first-child) {
  transform: translateX(-100%);
}
.chart-tooltip {
  position: absolute;
  top: 10px;
  z-index: 2;
  min-width: 148px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px 10px;
  border: 1px solid var(--peek-border);
  border-radius: 8px;
  background: var(--peek-surface);
  color: var(--peek-text);
  box-shadow: 0 8px 22px color-mix(in srgb, #000 16%, transparent);
  pointer-events: none;
  transform: translate(-50%, 0);
}
.chart-tooltip.start {
  transform: translate(0, 0);
}
.chart-tooltip.end {
  transform: translate(-100%, 0);
}
.chart-tooltip small {
  color: var(--peek-muted);
  font-size: 9px;
}
.tooltip-total,
.tooltip-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  font-size: 10px;
}
.tooltip-total {
  padding-bottom: 4px;
  border-bottom: 1px solid var(--peek-border);
  font-weight: 650;
}
.tooltip-row span {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tooltip-row i {
  width: 7px;
  height: 7px;
  flex: none;
  border-radius: 50%;
}
.tooltip-row strong,
.tooltip-total strong {
  flex: none;
  font-size: 11px;
  font-weight: 650;
}
</style>
