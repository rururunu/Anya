import { formatModelDisplayName, getProviderDisplayName } from "@/lib/providerIcons";
import { formatTokenCount } from "@/services/chat/tokenEstimate";
import type { TokenUsageReport } from "@/types/tokenUsage";

export const CHART_WIDTH = 760;
export const CHART_HEIGHT = 210;
export const CHART_TOP = 14;
export const CHART_BOTTOM = 196;
export const CHART_LEFT = 8;
export const CHART_RIGHT = 752;
export const MAX_CHART_SERIES = 5;
export const OTHER_SERIES_ID = "other";
export const OTHER_SERIES_COLOR = "#8a8f98";

export const CHART_COLORS = [
  "#5b8def",
  "#2fbfa0",
  "#e0a24b",
  "#c57bd6",
  "#6c8aef",
  "#4f9d6e",
  "#e07a4a",
  "#5aa6c9",
];

const BRANDS: Array<{ key: string; pattern: RegExp }> = [
  { key: "deepseek", pattern: /deepseek/i },
  { key: "gemini", pattern: /gemini|gemma|antigravity/i },
  { key: "claude", pattern: /claude|anthropic/i },
  { key: "openai", pattern: /gpt-|o[1-4](?:-mini|-pro)?\b|openai|chatgpt/i },
  { key: "grok", pattern: /grok|x-?ai/i },
  { key: "qwen", pattern: /qwen|qwq|qvq|tongyi|通义|千问/i },
  { key: "kimi", pattern: /kimi|moonshot/i },
  { key: "zhipu", pattern: /glm|zhipu|智谱/i },
  { key: "minimax", pattern: /minimax/i },
  { key: "volcengine", pattern: /volcengine|doubao|豆包|火山/i },
  { key: "mimo", pattern: /mimo|小米/i },
];

export type UsageModelRef = {
  model: string;
  provider?: string;
};

export type ChartSeriesDef = {
  id: string;
  label: string;
  color: string;
  values: number[];
};

export type StackedBar = {
  index: number;
  x: number;
  width: number;
  cx: number;
  label: string;
  total: number;
  segments: Array<{ id: string; color: string; y: number; height: number; value: number }>;
};

export type ModelListRow = {
  id: string;
  model: string;
  label: string;
  provider: string;
  color: string;
  tokens: number;
  calls: number;
  share: number;
};

export type ModelListGroup = {
  id: string;
  label: string;
  total: number;
  models: ModelListRow[];
};

/** Brand key for grouping: prefer the model id, then the stored provider. */
export function inferUsageBrand(model: string, provider?: string | null): string {
  for (const { key, pattern } of BRANDS) {
    if (pattern.test(model) || (provider && pattern.test(provider))) return key;
  }
  const fallback = provider?.trim();
  return fallback || "other";
}

export function seriesColor(index: number, isOther = false): string {
  if (isOther) return OTHER_SERIES_COLOR;
  return CHART_COLORS[index % CHART_COLORS.length] ?? OTHER_SERIES_COLOR;
}

export function niceMax(value: number): number {
  if (value <= 0) return 1;
  const exp = 10 ** Math.floor(Math.log10(value));
  const n = value / exp;
  const nice = n <= 1 ? 1 : n <= 2 ? 2 : n <= 5 ? 5 : 10;
  return nice * exp;
}

export function yAxisTicks(max: number): Array<{ value: number; y: number; label: string }> {
  const top = niceMax(max);
  const steps = 4;
  const ticks = [];
  for (let i = 0; i <= steps; i += 1) {
    const value = (top * (steps - i)) / steps;
    const y = CHART_TOP + (i / steps) * (CHART_BOTTOM - CHART_TOP);
    ticks.push({
      value,
      y,
      label: formatTokenCount(value),
    });
  }
  return ticks;
}

function chartModelLabel(model: string, brand: string): string {
  const formatted = formatModelDisplayName(model, brand);
  const slash = formatted.indexOf("/");
  if (slash > 0 && slash < formatted.length - 1) return formatted.slice(slash + 1);
  return formatted;
}

function modelTokensInBucket(bucket: TokenUsageReport["timeline"][number], model: string): number {
  return bucket.models[model] ?? 0;
}

/** Top models stay separate; the rest fold into one "Other" series for the chart. */
export function buildChartSeries(report: TokenUsageReport, otherLabel: string): ChartSeriesDef[] {
  const ranked = report.byModel;
  if (!ranked.length || !report.timeline.length) return [];

  const lead = ranked.slice(0, MAX_CHART_SERIES);
  const rest = ranked.slice(MAX_CHART_SERIES);
  const series: ChartSeriesDef[] = lead.map((item, index) => ({
    id: `${item.provider ?? ""}:${item.model}`,
    label: chartModelLabel(item.model, inferUsageBrand(item.model, item.provider)),
    color: seriesColor(index),
    values: report.timeline.map((bucket) => modelTokensInBucket(bucket, item.model)),
  }));

  if (rest.length) {
    series.push({
      id: OTHER_SERIES_ID,
      label: otherLabel,
      color: OTHER_SERIES_COLOR,
      values: report.timeline.map((bucket) =>
        rest.reduce((sum, item) => sum + modelTokensInBucket(bucket, item.model), 0),
      ),
    });
  }
  return series;
}

export function buildStackedBars(
  timeline: TokenUsageReport["timeline"],
  series: ChartSeriesDef[],
): { bars: StackedBar[]; max: number } {
  const totals = timeline.map((_, index) =>
    series.reduce((sum, item) => sum + (item.values[index] ?? 0), 0),
  );
  const max = niceMax(Math.max(...totals, 1));
  const count = Math.max(timeline.length, 1);
  const slot = (CHART_RIGHT - CHART_LEFT) / count;
  const width = Math.max(3, slot * (count > 40 ? 0.72 : 0.62));
  const range = CHART_BOTTOM - CHART_TOP;

  const bars = timeline.map((bucket, index): StackedBar => {
    const x = CHART_LEFT + index * slot + (slot - width) / 2;
    let baseline = 0;
    const segments = [];
    for (const item of series) {
      const value = item.values[index] ?? 0;
      const height = max === 0 ? 0 : (value / max) * range;
      const y = CHART_BOTTOM - baseline - height;
      if (height > 0) {
        segments.push({ id: item.id, color: item.color, y, height, value });
      }
      baseline += height;
    }
    return {
      index,
      x,
      width,
      cx: x + width / 2,
      label: bucket.label,
      total: totals[index] ?? 0,
      segments,
    };
  });

  return { bars, max };
}

export type TimelineChartKind = "stacked" | "line" | "area";

export type ChartPoint = { x: number; y: number; value: number };

export type LineSeries = {
  id: string;
  label: string;
  color: string;
  points: ChartPoint[];
  path: string;
};

export type AreaSeries = LineSeries & { area: string };

function polyline(points: ChartPoint[]): string {
  if (!points.length) return "";
  return points.map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`).join(" ");
}

/** Independent lines, scaled to the tallest series value. */
export function buildLineSeries(
  timeline: TokenUsageReport["timeline"],
  series: ChartSeriesDef[],
): { series: LineSeries[]; max: number } {
  const { bars } = buildStackedBars(timeline, series);
  const max = niceMax(Math.max(1, ...series.flatMap((item) => item.values)));
  const range = CHART_BOTTOM - CHART_TOP;
  return {
    max,
    series: series.map((item) => {
      const points = bars.map((bar) => {
        const value = item.values[bar.index] ?? 0;
        return {
          x: bar.cx,
          y: CHART_BOTTOM - (value / max) * range,
          value,
        };
      });
      return {
        id: item.id,
        label: item.label,
        color: item.color,
        points,
        path: polyline(points),
      };
    }),
  };
}

/** Stacked areas that share the same y-max as the bar chart. */
export function buildStackedAreas(
  timeline: TokenUsageReport["timeline"],
  series: ChartSeriesDef[],
): { series: AreaSeries[]; max: number } {
  const stacked = buildStackedBars(timeline, series);
  const range = CHART_BOTTOM - CHART_TOP;
  return {
    max: stacked.max,
    series: series.map((item, seriesIndex) => {
      const tops: ChartPoint[] = [];
      const bottoms: ChartPoint[] = [];
      for (const bar of stacked.bars) {
        const segment = bar.segments.find((entry) => entry.id === item.id);
        const value = item.values[bar.index] ?? 0;
        if (segment) {
          tops.push({ x: bar.cx, y: segment.y, value });
          bottoms.push({ x: bar.cx, y: segment.y + segment.height, value: 0 });
        } else {
          let below = 0;
          for (let index = 0; index < seriesIndex; index += 1) {
            below += series[index]?.values[bar.index] ?? 0;
          }
          const y = CHART_BOTTOM - (below / stacked.max) * range;
          tops.push({ x: bar.cx, y, value });
          bottoms.push({ x: bar.cx, y, value: 0 });
        }
      }
      const path = polyline(tops);
      const area = tops.length
        ? `${path} ${[...bottoms]
            .reverse()
            .map((point) => `L ${point.x} ${point.y}`)
            .join(" ")} Z`
        : "";
      return {
        id: item.id,
        label: item.label,
        color: item.color,
        points: tops,
        path,
        area,
      };
    }),
  };
}

export function bucketIndexAt(x: number, barCount: number): number {
  if (barCount <= 0) return 0;
  const slot = (CHART_RIGHT - CHART_LEFT) / barCount;
  const clamped = Math.min(CHART_RIGHT, Math.max(CHART_LEFT, x));
  return Math.min(barCount - 1, Math.max(0, Math.floor((clamped - CHART_LEFT) / slot)));
}

export function colorForModel(models: UsageModelRef[], model: string): string {
  const index = models.findIndex((item) => item.model === model);
  if (index < 0) return OTHER_SERIES_COLOR;
  if (index >= MAX_CHART_SERIES) return OTHER_SERIES_COLOR;
  return seriesColor(index);
}

export function buildModelGroups(report: TokenUsageReport, otherLabel = "Other"): ModelListGroup[] {
  const groups = new Map<string, ModelListGroup>();
  for (const item of report.byModel) {
    const brand = inferUsageBrand(item.model, item.provider);
    const row: ModelListRow = {
      id: `${item.provider ?? ""}:${item.model}`,
      model: item.model,
      label: chartModelLabel(item.model, brand),
      provider: item.provider ?? brand,
      color: colorForModel(report.byModel, item.model),
      tokens: item.usage.totalTokens,
      calls: item.calls,
      share: item.share,
    };
    const existing = groups.get(brand);
    if (existing) {
      existing.models.push(row);
      existing.total += row.tokens;
    } else {
      groups.set(brand, {
        id: brand,
        label: brand === "other" ? otherLabel : getProviderDisplayName(brand),
        total: row.tokens,
        models: [row],
      });
    }
  }
  return [...groups.values()]
    .map((group) => ({
      ...group,
      models: [...group.models].sort((a, b) => b.tokens - a.tokens),
    }))
    .sort((a, b) => b.total - a.total);
}

export const DONUT_SIZE = 160;
export const DONUT_CX = 80;
export const DONUT_CY = 80;
export const DONUT_OUTER = 72;
export const DONUT_INNER = 46;

export const BREAKDOWN_COLORS = {
  input: "#5b8def",
  output: "#2fbfa0",
  tools: "#7c3aed",
  context: "#0f766e",
  system: "#b45309",
  memory: "#be185d",
  reasoning: "#c2410c",
  cache: "#0369a1",
} as const;

export type ShareDatum = {
  id: string;
  label: string;
  color: string;
  value: number;
  hint?: string;
  children?: ShareDatum[];
};

export type DonutArc = ShareDatum & {
  share: number;
  path: string;
};

function polar(cx: number, cy: number, radius: number, angle: number) {
  return {
    x: cx + radius * Math.cos(angle),
    y: cy + radius * Math.sin(angle),
  };
}

function donutSegment(
  cx: number,
  cy: number,
  outerR: number,
  innerR: number,
  start: number,
  end: number,
): string {
  const sweep = end - start;
  if (sweep <= 1e-6) return "";
  if (sweep >= Math.PI * 2 - 1e-6) {
    const mid = start + Math.PI;
    return `${donutSegment(cx, cy, outerR, innerR, start, mid)} ${donutSegment(cx, cy, outerR, innerR, mid, start + Math.PI * 2)}`.trim();
  }
  const large = sweep > Math.PI ? 1 : 0;
  const outerStart = polar(cx, cy, outerR, start);
  const outerEnd = polar(cx, cy, outerR, end);
  const innerEnd = polar(cx, cy, innerR, end);
  const innerStart = polar(cx, cy, innerR, start);
  return `M ${outerStart.x} ${outerStart.y} A ${outerR} ${outerR} 0 ${large} 1 ${outerEnd.x} ${outerEnd.y} L ${innerEnd.x} ${innerEnd.y} A ${innerR} ${innerR} 0 ${large} 0 ${innerStart.x} ${innerStart.y} Z`;
}

/** Format a 0–1 share for chart legends. */
export function formatSharePercent(share: number): string {
  const pct = share * 100;
  if (pct > 0 && pct < 1) return "<1%";
  return `${Math.round(pct)}%`;
}

/** SVG donut arcs from non-zero share items, starting at 12 o'clock. */
export function buildDonutArcs(items: ShareDatum[]): DonutArc[] {
  const positive = items.filter((item) => item.value > 0);
  const total = positive.reduce((sum, item) => sum + item.value, 0);
  if (total <= 0) return [];
  const gap = positive.length > 1 ? 0.03 : 0;
  let angle = -Math.PI / 2;
  return positive.map((item) => {
    const share = item.value / total;
    const sweep = Math.max(0, share * Math.PI * 2 - gap);
    const start = angle + gap / 2;
    const end = start + sweep;
    angle += share * Math.PI * 2;
    return {
      ...item,
      share,
      path: donutSegment(DONUT_CX, DONUT_CY, DONUT_OUTER, DONUT_INNER, start, end),
    };
  });
}

/** Brand slices when several providers exist; otherwise the models in the only group. */
export function buildModelShareItems(groups: ModelListGroup[]): ShareDatum[] {
  if (groups.length > 1) {
    return groups.map((group, index) => ({
      id: group.id,
      label: group.label,
      color: group.models[0]?.color ?? seriesColor(index, group.id === "other"),
      value: group.total,
      children: group.models.map((model) => ({
        id: model.id,
        label: model.label,
        color: model.color,
        value: model.tokens,
      })),
    }));
  }
  return (groups[0]?.models ?? []).map((model) => ({
    id: model.id,
    label: model.label,
    color: model.color,
    value: model.tokens,
  }));
}

export const RADAR_SIZE = 240;
export const RADAR_CX = 120;
export const RADAR_CY = 120;
export const RADAR_RADIUS = 78;

export type RadarAxis = ShareDatum & {
  share: number;
  ratio: number;
  x: number;
  y: number;
  axisX: number;
  axisY: number;
  labelX: number;
  labelY: number;
  anchor: "start" | "middle" | "end";
};

export type RadarChart = {
  axes: RadarAxis[];
  rings: string[];
  spokes: Array<{ x: number; y: number }>;
  polygon: string;
};

/** Log mapping so a dominant category does not flatten the rest. */
export function radarRatio(value: number, max: number): number {
  if (max <= 0 || value <= 0) return 0;
  return Math.log1p(value) / Math.log1p(max);
}

/** Radar polygon, rings, and label anchors for token categories. */
export function buildRadarChart(items: ShareDatum[]): RadarChart {
  const axesItems = items.length ? items : [];
  const count = axesItems.length;
  const max = Math.max(1, ...axesItems.map((item) => item.value));
  const total = axesItems.reduce((sum, item) => sum + Math.max(0, item.value), 0) || 1;
  const angleAt = (index: number) => -Math.PI / 2 + (index * 2 * Math.PI) / Math.max(count, 1);
  const rings = [0.25, 0.5, 0.75, 1].map((ring) =>
    axesItems
      .map((_, index) => {
        const angle = angleAt(index);
        return `${RADAR_CX + RADAR_RADIUS * ring * Math.cos(angle)},${RADAR_CY + RADAR_RADIUS * ring * Math.sin(angle)}`;
      })
      .join(" "),
  );
  const axes = axesItems.map((item, index) => {
    const angle = angleAt(index);
    const ratio = radarRatio(item.value, max);
    const cos = Math.cos(angle);
    const sin = Math.sin(angle);
    const labelR = RADAR_RADIUS + 20;
    return {
      ...item,
      share: item.value / total,
      ratio,
      x: RADAR_CX + RADAR_RADIUS * ratio * cos,
      y: RADAR_CY + RADAR_RADIUS * ratio * sin,
      axisX: RADAR_CX + RADAR_RADIUS * cos,
      axisY: RADAR_CY + RADAR_RADIUS * sin,
      labelX: RADAR_CX + labelR * cos,
      labelY: RADAR_CY + labelR * sin,
      anchor: (cos > 0.35 ? "start" : cos < -0.35 ? "end" : "middle") as RadarAxis["anchor"],
    };
  });
  return {
    axes,
    rings,
    spokes: axes.map((axis) => ({ x: axis.axisX, y: axis.axisY })),
    polygon: axes.map((axis) => `${axis.x},${axis.y}`).join(" "),
  };
}
