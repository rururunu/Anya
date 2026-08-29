<template>
  <div class="chart-card">
    <div class="chart-card-header">
      <span v-if="spec.title" class="chart-card-title">{{ spec.title }}</span>
      <span class="chart-card-type">{{ chartLabel }}</span>
      <button
        type="button"
        class="chart-copy-button"
        :class="{ copied }"
        :aria-label="copyLabel"
        :title="copyLabel"
        @click="copyData"
      >
        <component :is="copyIcon" :size="13" :stroke-width="2" aria-hidden="true" />
      </button>
      <button
        type="button"
        class="chart-copy-button"
        :aria-label="downloadLabel"
        :title="downloadLabel"
        @click="downloadImage"
      >
        <Download :size="13" :stroke-width="2" aria-hidden="true" />
      </button>
    </div>
    <div ref="chartEl" class="chart-canvas" />
    <div v-if="errorMessage" class="chart-error" role="alert">{{ errorMessage }}</div>
  </div>
</template>

<script setup lang="ts">
import { Check, Copy, Download } from "@lucide/vue";
import { useTheme } from "@/composables/useTheme";
import { readThemeToken } from "@/services/theme";
import { computed, onMounted, onUnmounted, ref, watch, type Component } from "vue";
import { copyText } from "@/services/clipboard";
import { exportChartPng } from "./chartExport";
import { echarts, ensureChartModules, GL_TYPES } from "./chartEcharts";
import type { ChartSpec, ChartType } from "@/services/chat/chartSpec";
// Everyday series register in chartEcharts.ts; rarer 2D series + echarts-gl
// load on demand so the ChartCard async chunk stays under the Vite size warn.

type EChartsInstance = ReturnType<typeof echarts.init>;
type EChartsCoreOption = Parameters<EChartsInstance["setOption"]>[0];

interface ChartTheme {
  palette: string[];
  axisText: string;
  labelText: string;
  splitLine: string;
  cardBg: string;
  success: string;
  danger: string;
}

const TYPE_LABELS: Record<ChartType, string> = {
  bar: "Bar",
  line: "Line",
  scatter: "Scatter",
  pie: "Pie",
  funnel: "Funnel",
  gauge: "Gauge",
  radar: "Radar",
  heatmap: "Heatmap",
  candlestick: "Candlestick",
  treemap: "Treemap",
  sankey: "Sankey",
  graph: "Graph",
  parallel: "Parallel",
  bar3d: "3D Bar",
  scatter3d: "3D Scatter",
  surface: "3D Surface",
  line3d: "3D Line",
  custom: "Custom",
};

const FALLBACK_PALETTE = [
  "#4f8ef7",
  "#34c98f",
  "#f7a44f",
  "#e05c7a",
  "#9b6df2",
  "#3ec6d9",
  "#d9c14f",
  "#8a9aa8",
];

const props = defineProps<{ spec: ChartSpec }>();

const chartEl = ref<HTMLDivElement | null>(null);
const { isDark } = useTheme();
const copied = ref(false);
const errorMessage = ref("");
let chart: EChartsInstance | null = null;
let resetTimer: number | undefined;
let glRegistered = false;

const chartLabel = computed(() => TYPE_LABELS[props.spec.type]);
const copyIcon = computed<Component>(() => (copied.value ? Check : Copy));
const copyLabel = computed(() => (copied.value ? "Copied" : "Copy data"));
const downloadLabel = "Download image";

function cssVar(name: string, fallback: string): string {
  return readThemeToken(name, fallback);
}

function readTheme(): ChartTheme {
  return {
    palette: Array.from({ length: 8 }, (_, i) =>
      cssVar(`--peek-chart-${i + 1}`, FALLBACK_PALETTE[i] ?? FALLBACK_PALETTE[0]),
    ),
    axisText: cssVar("--peek-faint", "#8a8a8a"),
    labelText: cssVar("--peek-text", "#242424"),
    splitLine: cssVar("--peek-border", "rgba(0,0,0,0.16)"),
    cardBg: cssVar("--peek-bg", "#f8f8f8"),
    success: cssVar("--peek-success", "#18794e"),
    danger: cssVar("--peek-danger", "#c42b1c"),
  };
}

function parseHex(color: string): [number, number, number] | null {
  const match = /^#([0-9a-f]{6})$/i.exec(color.trim());
  if (!match) return null;
  const n = Number.parseInt(match[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** Linear mix of two hex colors (`t` = 0 → a, 1 → b). Falls back to `a`
 *  when either side is not a hex color. */
function mixHex(a: string, b: string, t: number): string {
  const ca = parseHex(a);
  const cb = parseHex(b);
  if (!ca || !cb) return a;
  const mix = (x: number, y: number) => Math.round(x + (y - x) * t);
  return `rgb(${mix(ca[0], cb[0])},${mix(ca[1], cb[1])},${mix(ca[2], cb[2])})`;
}

function formatValue(value: unknown, unit?: string): string {
  return unit ? `${String(value)} ${unit}` : String(value);
}

function axisTextStyle(theme: ChartTheme) {
  return { color: theme.axisText, fontSize: 11 };
}

function legendConfig(theme: ChartTheme, position: "top" | "bottom") {
  return {
    [position]: 4,
    icon: "circle",
    itemWidth: 9,
    itemHeight: 9,
    textStyle: { color: theme.axisText, fontSize: 11 },
  };
}

function buildCartesian(spec: ChartSpec, theme: ChartTheme, type: "bar" | "line") {
  const series = spec.series ?? [];
  const hasLegend = series.length > 1 || Boolean(series[0]?.name);
  return {
    animationDuration: 300,
    color: theme.palette,
    tooltip: {
      trigger: "axis",
      valueFormatter: (value: unknown) => formatValue(value, spec.unit),
    },
    grid: { left: 8, right: 16, top: hasLegend ? 36 : 16, bottom: 6, containLabel: true },
    xAxis: {
      type: "category",
      data: spec.x ?? [],
      axisLine: { lineStyle: { color: theme.splitLine } },
      axisTick: { show: false },
      axisLabel: axisTextStyle(theme),
    },
    yAxis: {
      type: "value",
      splitLine: { lineStyle: { color: theme.splitLine } },
      axisLabel: axisTextStyle(theme),
    },
    legend: hasLegend ? legendConfig(theme, "top") : undefined,
    series: series.map((entry) => ({
      name: entry.name,
      type,
      data: entry.data as number[],
      smooth: type === "line",
      barMaxWidth: 28,
      itemStyle: type === "bar" ? { borderRadius: [3, 3, 0, 0] } : undefined,
    })),
  };
}

function buildScatter(spec: ChartSpec, theme: ChartTheme) {
  const series = spec.series ?? [];
  const hasLegend = series.length > 1 || Boolean(series[0]?.name);
  return {
    animationDuration: 300,
    color: theme.palette,
    tooltip: {
      trigger: "item",
      valueFormatter: (value: unknown) => formatValue(value, spec.unit),
    },
    grid: { left: 8, right: 16, top: hasLegend ? 36 : 16, bottom: 6, containLabel: true },
    xAxis: {
      type: "value",
      axisLine: { lineStyle: { color: theme.splitLine } },
      splitLine: { show: false },
      axisLabel: axisTextStyle(theme),
    },
    yAxis: {
      type: "value",
      splitLine: { lineStyle: { color: theme.splitLine } },
      axisLabel: axisTextStyle(theme),
    },
    legend: hasLegend ? legendConfig(theme, "top") : undefined,
    series: series.map((entry) => ({
      name: entry.name,
      type: "scatter",
      data: entry.data as number[][],
      symbolSize: 10,
      itemStyle: { opacity: 0.85 },
    })),
  };
}

function buildSlices(spec: ChartSpec, theme: ChartTheme, type: "pie" | "funnel") {
  const isPie = type === "pie";
  return {
    animationDuration: 300,
    color: theme.palette,
    tooltip: {
      trigger: "item",
      valueFormatter: (value: unknown) => formatValue(value, spec.unit),
    },
    legend: legendConfig(theme, "bottom"),
    series: [
      {
        type,
        radius: isPie ? ["38%", "66%"] : undefined,
        center: isPie ? ["50%", "45%"] : undefined,
        left: isPie ? undefined : "12%",
        width: isPie ? undefined : "76%",
        top: isPie ? undefined : 40,
        bottom: isPie ? undefined : 34,
        minSize: isPie ? undefined : "16%",
        itemStyle: {
          borderRadius: isPie ? 4 : 0,
          borderWidth: isPie ? 2 : 1,
          borderColor: theme.cardBg,
        },
        label: {
          color: theme.labelText,
          fontSize: 11,
          position: isPie ? "outside" : "inside",
        },
        labelLine: { lineStyle: { color: theme.splitLine } },
        data: spec.items ?? [],
      },
    ],
  };
}

function buildGauge(spec: ChartSpec, theme: ChartTheme) {
  const min = spec.min ?? 0;
  const max = spec.max ?? 100;
  return {
    animationDuration: 300,
    color: theme.palette,
    series: [
      {
        type: "gauge",
        min,
        max,
        radius: "92%",
        center: ["50%", "62%"],
        axisLine: { lineStyle: { width: 10, color: [[1, theme.splitLine]] } },
        pointer: { itemStyle: { color: theme.palette[0] } },
        axisTick: { distance: -14, lineStyle: { color: theme.splitLine } },
        splitLine: { distance: -14, length: 8, lineStyle: { color: theme.axisText, width: 1 } },
        axisLabel: { color: theme.axisText, distance: -26, fontSize: 9 },
        title: { color: theme.axisText, fontSize: 11, offsetCenter: [0, "78%"] },
        detail: { color: theme.labelText, fontSize: 14, offsetCenter: [0, "56%"] },
        data: (spec.items ?? []).map((item) => ({ name: item.name, value: item.value })),
      },
    ],
  };
}

function buildRadar(spec: ChartSpec, theme: ChartTheme) {
  const series = spec.series ?? [];
  return {
    animationDuration: 300,
    color: theme.palette,
    tooltip: {
      trigger: "item",
      valueFormatter: (value: unknown) => formatValue(value, spec.unit),
    },
    legend: series.length > 1 ? legendConfig(theme, "bottom") : undefined,
    radar: {
      indicator: spec.indicators ?? [],
      radius: "62%",
      axisName: { color: theme.axisText, fontSize: 11 },
      axisLine: { lineStyle: { color: theme.splitLine } },
      splitLine: { lineStyle: { color: theme.splitLine } },
    },
    series: series.map((entry) => ({
      name: entry.name,
      type: "radar",
      data: [{ value: entry.data as number[], name: entry.name }],
    })),
  };
}

function buildHeatmap(spec: ChartSpec, theme: ChartTheme) {
  const series = spec.series ?? [];
  const cells = series.flatMap((entry) => entry.data as number[][]);
  const min = Math.min(...cells.map((cell) => cell[2]));
  let max = Math.max(...cells.map((cell) => cell[2]));
  if (min === max) max = min + 1;
  return {
    animationDuration: 300,
    color: theme.palette,
    tooltip: {
      position: "top",
      trigger: "item",
      valueFormatter: (value: unknown) => formatValue(value, spec.unit),
    },
    grid: { left: 8, right: 44, top: 16, bottom: 6, containLabel: true },
    xAxis: {
      type: "category",
      data: spec.x ?? [],
      axisLine: { lineStyle: { color: theme.splitLine } },
      axisTick: { show: false },
      axisLabel: axisTextStyle(theme),
    },
    yAxis: {
      type: "category",
      data: spec.y ?? [],
      axisLine: { lineStyle: { color: theme.splitLine } },
      axisTick: { show: false },
      axisLabel: axisTextStyle(theme),
    },
    visualMap: {
      min,
      max,
      orient: "vertical",
      right: 4,
      top: "center",
      textStyle: { color: theme.axisText, fontSize: 10 },
      inRange: {
        color: [mixHex(theme.cardBg, theme.palette[0], 0.25), theme.palette[0], theme.palette[3]],
      },
    },
    series: series.map((entry) => ({
      name: entry.name,
      type: "heatmap",
      data: entry.data,
      itemStyle: { borderColor: theme.cardBg, borderWidth: 1 },
    })),
  };
}

function buildCandlestick(spec: ChartSpec, theme: ChartTheme) {
  const series = spec.series ?? [];
  const hasLegend = series.length > 1 || Boolean(series[0]?.name);
  return {
    animationDuration: 300,
    color: theme.palette,
    tooltip: { trigger: "axis", axisPointer: { type: "cross" } },
    grid: { left: 8, right: 16, top: hasLegend ? 36 : 16, bottom: 6, containLabel: true },
    xAxis: {
      type: "category",
      data: spec.x ?? [],
      axisLine: { lineStyle: { color: theme.splitLine } },
      axisTick: { show: false },
      axisLabel: axisTextStyle(theme),
    },
    yAxis: {
      type: "value",
      scale: true,
      splitLine: { lineStyle: { color: theme.splitLine } },
      axisLabel: axisTextStyle(theme),
    },
    legend: hasLegend ? legendConfig(theme, "top") : undefined,
    series: series.map((entry) => ({
      name: entry.name,
      type: "candlestick",
      data: entry.data,
      itemStyle: {
        color: theme.success,
        color0: theme.danger,
        borderColor: theme.success,
        borderColor0: theme.danger,
      },
    })),
  };
}

function buildTreemap(spec: ChartSpec, theme: ChartTheme) {
  return {
    animationDuration: 300,
    color: theme.palette,
    tooltip: {
      trigger: "item",
      valueFormatter: (value: unknown) => formatValue(value, spec.unit),
    },
    series: [
      {
        type: "treemap",
        roam: false,
        nodeClick: false,
        breadcrumb: { show: false },
        top: 8,
        left: 4,
        right: 4,
        bottom: 4,
        label: { color: theme.labelText, fontSize: 11 },
        itemStyle: { borderColor: theme.cardBg, borderWidth: 1, gapWidth: 1 },
        data: spec.data,
      },
    ],
  };
}

function buildFlow(spec: ChartSpec, theme: ChartTheme, type: "sankey" | "graph") {
  const nodes = spec.nodes ?? [];
  const links = spec.links ?? [];
  if (type === "graph") {
    return {
      animationDuration: 300,
      color: theme.palette,
      tooltip: { trigger: "item" },
      series: [
        {
          type: "graph",
          layout: "force",
          roam: false,
          draggable: true,
          label: { show: true, color: theme.labelText, fontSize: 10, position: "right" },
          lineStyle: { color: theme.splitLine, width: 1, curveness: 0.2 },
          itemStyle: { borderColor: theme.cardBg, borderWidth: 1 },
          data: nodes,
          links,
        },
      ],
    };
  }
  return {
    animationDuration: 300,
    color: theme.palette,
    tooltip: { trigger: "item" },
    series: [
      {
        type: "sankey",
        top: 8,
        bottom: 8,
        left: 8,
        right: 48,
        nodeAlign: "justify",
        label: { color: theme.labelText, fontSize: 10 },
        itemStyle: { borderColor: theme.cardBg, borderWidth: 1 },
        lineStyle: { color: "gradient", opacity: 0.4 },
        data: nodes,
        links,
      },
    ],
  };
}

function buildParallel(spec: ChartSpec, theme: ChartTheme) {
  const dimensions = spec.dimensions ?? [];
  const series = spec.series ?? [];
  return {
    animationDuration: 300,
    color: theme.palette,
    tooltip: { trigger: "item" },
    parallel: { left: 48, right: 48, top: 24, bottom: 8 },
    parallelAxis: dimensions.map((name, dim) => ({
      dim,
      name,
      nameTextStyle: { color: theme.axisText, fontSize: 11 },
      axisLabel: axisTextStyle(theme),
      axisLine: { lineStyle: { color: theme.splitLine } },
    })),
    series: series.map((entry, i) => ({
      name: entry.name,
      type: "parallel",
      data: entry.data,
      lineStyle: {
        color: theme.palette[i % theme.palette.length],
        width: 1.5,
        opacity: 0.7,
      },
    })),
  };
}

function build3DAxes(theme: ChartTheme) {
  return {
    axisLine: { lineStyle: { color: theme.splitLine } },
    axisLabel: { color: theme.axisText, fontSize: 10 },
    splitLine: { lineStyle: { color: theme.splitLine, opacity: 0.4 } },
  };
}

function buildBar3D(spec: ChartSpec, theme: ChartTheme) {
  const series = spec.series ?? [];
  return {
    animationDuration: 300,
    color: theme.palette,
    tooltip: {},
    grid3D: {
      boxWidth: 90,
      boxDepth: 90,
      viewControl: { autoRotate: true, autoRotateSpeed: 6 },
    },
    xAxis3D: { type: "category", data: spec.x ?? [], ...build3DAxes(theme) },
    yAxis3D: { type: "category", data: spec.y ?? [], ...build3DAxes(theme) },
    zAxis3D: { type: "value", ...build3DAxes(theme) },
    series: series.map((entry, i) => ({
      name: entry.name,
      type: "bar3D",
      data: entry.data,
      shading: "lambert",
      bevelSize: 0.3,
      itemStyle: { color: theme.palette[i % theme.palette.length], opacity: 0.9 },
    })),
  };
}

function buildPoint3D(
  spec: ChartSpec,
  theme: ChartTheme,
  type: "scatter3D" | "surface" | "line3D",
) {
  const series = spec.series ?? [];
  const axes = {
    xAxis3D: { type: "value", ...build3DAxes(theme) },
    yAxis3D: { type: "value", ...build3DAxes(theme) },
    zAxis3D: { type: "value", ...build3DAxes(theme) },
  };
  return {
    animationDuration: 300,
    color: theme.palette,
    tooltip: {},
    grid3D: {
      boxWidth: 90,
      boxDepth: 90,
      viewControl: { autoRotate: true, autoRotateSpeed: 6 },
    },
    ...axes,
    series: series.map((entry, i) => {
      const color = theme.palette[i % theme.palette.length];
      if (type === "surface") {
        return {
          name: entry.name,
          type,
          data: entry.data,
          shading: "lambert",
          wireframe: { show: false },
          itemStyle: { color, opacity: 0.85 },
        };
      }
      if (type === "line3D") {
        return { name: entry.name, type, data: entry.data, lineStyle: { width: 2, color } };
      }
      return {
        name: entry.name,
        type,
        data: entry.data,
        symbolSize: 8,
        itemStyle: { color, opacity: 0.85 },
      };
    }),
  };
}

function buildCustom(spec: ChartSpec, theme: ChartTheme) {
  const option = spec.option ?? {};
  const rawTooltip =
    typeof option.tooltip === "object" && option.tooltip !== null && !Array.isArray(option.tooltip)
      ? option.tooltip
      : {};
  return {
    ...option,
    color: Array.isArray(option.color) && option.color.length > 0 ? option.color : theme.palette,
    backgroundColor: "transparent",
    animationDuration: 300,
    // Force canvas-rendered tooltips: model-supplied formatter strings must
    // never become HTML in the DOM (everything else stays canvas-drawn).
    tooltip: { renderMode: "richText", ...rawTooltip },
  };
}

function buildOption(spec: ChartSpec, theme: ChartTheme): EChartsCoreOption {
  switch (spec.type) {
    case "bar":
    case "line":
      return buildCartesian(spec, theme, spec.type);
    case "scatter":
      return buildScatter(spec, theme);
    case "pie":
    case "funnel":
      return buildSlices(spec, theme, spec.type);
    case "gauge":
      return buildGauge(spec, theme);
    case "radar":
      return buildRadar(spec, theme);
    case "heatmap":
      return buildHeatmap(spec, theme);
    case "candlestick":
      return buildCandlestick(spec, theme);
    case "treemap":
      return buildTreemap(spec, theme);
    case "sankey":
    case "graph":
      return buildFlow(spec, theme, spec.type);
    case "parallel":
      return buildParallel(spec, theme);
    case "bar3d":
      return buildBar3D(spec, theme);
    case "scatter3d":
      return buildPoint3D(spec, theme, "scatter3D");
    case "surface":
      return buildPoint3D(spec, theme, "surface");
    case "line3d":
      return buildPoint3D(spec, theme, "line3D");
    case "custom":
      return buildCustom(spec, theme);
  }
}

/** Lazily registers the echarts-gl extension (kept out of the main bundle;
 *  only loaded the first time a 3D chart is rendered). */
async function ensureGlRegistered() {
  if (glRegistered) return;
  const [charts, components] = await Promise.all([
    import("echarts-gl/charts"),
    import("echarts-gl/components"),
  ]);
  echarts.use([
    charts.Bar3DChart,
    charts.Line3DChart,
    charts.Scatter3DChart,
    charts.SurfaceChart,
    components.Grid3DComponent,
  ]);
  glRegistered = true;
}

async function renderChart() {
  if (!chart) return;
  try {
    const spec = props.spec;
    const optionSeries = spec.option?.series;
    const customSeriesTypes =
      spec.type === "custom" && Array.isArray(optionSeries)
        ? optionSeries
            .map((series) =>
              series && typeof series === "object" && "type" in series
                ? String((series as { type?: unknown }).type ?? "")
                : "",
            )
            .filter(Boolean)
        : [];
    await ensureChartModules(spec.type, customSeriesTypes);
    if (GL_TYPES.has(spec.type)) {
      await ensureGlRegistered();
      if (!chart) return; // disposed while the dynamic import was in flight
    }
    // The spec prop is a Vue reactive proxy. Deep-clone it to plain data so
    // echarts' merge/clone machinery never sees proxied objects (a known
    // source of subtle echarts-in-Vue breakage).
    const plainSpec = JSON.parse(JSON.stringify(spec)) as ChartSpec;
    chart.setOption(buildOption(plainSpec, readTheme()), { notMerge: true });
    errorMessage.value = "";
  } catch (error) {
    // renderChart is async and called fire-and-forget, so an unhandled throw
    // would surface as an unhandledrejection; show the failure instead.
    console.warn("chart render failed:", error);
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
}

async function downloadImage() {
  if (!chart) return;
  const ok = await exportChartPng(chart, props.spec.title || chartLabel.value, {
    // 3D charts must stay at the device pixel ratio: zrender's
    // getRenderedCanvas only composites GL layers when
    // pixelRatio <= devicePixelRatio (higher values re-brush 2D only).
    pixelRatio: GL_TYPES.has(props.spec.type) ? undefined : 2,
    backgroundColor: readTheme().cardBg,
  });
  if (!ok) errorMessage.value = "Failed to export image. Please try again.";
}

async function copyData() {
  try {
    await copyText(JSON.stringify(props.spec, null, 2));
    copied.value = true;
  } catch (error) {
    console.error("failed to copy chart data:", error);
  }
  window.clearTimeout(resetTimer);
  resetTimer = window.setTimeout(() => {
    copied.value = false;
  }, 1600);
}

onMounted(() => {
  if (!chartEl.value) return;
  chart = echarts.init(chartEl.value);
  void renderChart();
});

watch(
  () => props.spec,
  () => void renderChart(),
);
watch(isDark, () => void renderChart());

onUnmounted(() => {
  window.clearTimeout(resetTimer);
  chart?.dispose();
  chart = null;
});
</script>

<style scoped>
.chart-card {
  box-sizing: border-box;
  margin: 0.65em 0;
  overflow: hidden;
  border: 1px solid var(--peek-code-border, var(--peek-border));
  border-radius: 6px;
  background: color-mix(in srgb, var(--peek-input-bg) 55%, transparent);
}

.chart-card-header {
  box-sizing: border-box;
  display: flex;
  align-items: center;
  gap: 8px;
  height: 34px;
  padding: 0 8px 0 12px;
  border-bottom: 1px solid var(--peek-code-border, var(--peek-border));
  background: var(--peek-code-toolbar-bg, color-mix(in srgb, var(--peek-text) 5%, transparent));
}

.chart-card-title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--peek-text);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chart-card-type {
  flex: none;
  padding: 2px 7px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--peek-muted) 25%, transparent);
  color: var(--peek-muted);
  font-size: 10px;
  line-height: 1.4;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.chart-copy-button {
  box-sizing: border-box;
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 24px;
  padding: 0;
  border: 1px solid
    var(--peek-code-border, color-mix(in srgb, var(--peek-text) 14%, var(--peek-border)));
  border-radius: 5px;
  background: color-mix(in srgb, var(--peek-code-fg, var(--peek-text)) 8%, transparent);
  color: var(--peek-code-icon, var(--peek-code-fg, var(--peek-text)));
  cursor: pointer;
}

.chart-copy-button svg {
  display: block;
  width: 13px;
  height: 13px;
}

.chart-copy-button:hover {
  border-color: var(--peek-code-border, var(--peek-border));
  background: var(--peek-code-hover-bg, color-mix(in srgb, var(--peek-text) 8%, transparent));
  color: var(--peek-code-fg, var(--peek-text));
}

.chart-copy-button:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--peek-accent) 55%, transparent);
  outline-offset: 1px;
}

.chart-copy-button.copied {
  color: #36a269;
}

.chart-canvas {
  box-sizing: border-box;
  width: 100%;
  height: 260px;
  padding: 6px 4px 2px;
}

.chart-error {
  box-sizing: border-box;
  min-height: 60px;
  padding: 10px 12px;
  color: var(--peek-danger, #c42b1c);
  font-size: 12px;
  line-height: 1.5;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}
</style>
