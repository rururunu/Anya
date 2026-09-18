import type { ChartSpec, ChartType } from "./chartSpec";

export interface ChartView {
  id: string;
  type: ChartType;
  stacked?: boolean;
  area?: boolean;
  horizontal?: boolean;
}

const CARTESIAN = new Set<ChartType>(["bar", "line"]);
const SLICES = new Set<ChartType>(["pie", "funnel"]);

function longestLabel(labels: string[]): number {
  return labels.reduce((max, label) => Math.max(max, label.length), 0);
}

export function shouldUseHorizontalBar(spec: ChartSpec): boolean {
  const labels = spec.x ?? [];
  if (labels.length === 0) return false;
  const longest = longestLabel(labels);
  return longest >= 12 || (longest >= 8 && labels.length <= 4);
}

export function chartViewsFor(spec: ChartSpec): ChartView[] {
  if (CARTESIAN.has(spec.type)) {
    const multi = (spec.series?.length ?? 0) > 1;
    const views: ChartView[] = [
      { id: "bar", type: "bar" },
      { id: "bar-h", type: "bar", horizontal: true },
    ];
    if (multi) {
      views.push({ id: "bar-stacked", type: "bar", stacked: true });
      views.push({ id: "bar-h-stacked", type: "bar", stacked: true, horizontal: true });
    }
    views.push({ id: "line", type: "line" });
    views.push({ id: "area", type: "line", area: true });
    return views;
  }
  if (SLICES.has(spec.type)) {
    return [
      { id: "pie", type: "pie" },
      { id: "funnel", type: "funnel" },
    ];
  }
  return [{ id: spec.type, type: spec.type }];
}

export function defaultChartView(spec: ChartSpec): ChartView {
  const views = chartViewsFor(spec);
  if (spec.type === "bar" && shouldUseHorizontalBar(spec)) {
    return views.find((view) => view.id === "bar-h") ?? views[0]!;
  }
  return views.find((view) => view.id === spec.type) ?? views[0]!;
}

export function chartViewLabelKey(id: string): string {
  switch (id) {
    case "bar":
      return "chart.view.bar";
    case "bar-h":
      return "chart.view.barH";
    case "bar-stacked":
      return "chart.view.barStacked";
    case "bar-h-stacked":
      return "chart.view.barHStacked";
    case "line":
      return "chart.view.line";
    case "area":
      return "chart.view.area";
    case "pie":
      return "chart.view.pie";
    case "funnel":
      return "chart.view.funnel";
    case "scatter":
      return "chart.view.scatter";
    case "gauge":
      return "chart.view.gauge";
    case "radar":
      return "chart.view.radar";
    case "heatmap":
      return "chart.view.heatmap";
    case "candlestick":
      return "chart.view.candlestick";
    case "treemap":
      return "chart.view.treemap";
    case "sankey":
      return "chart.view.sankey";
    case "graph":
      return "chart.view.graph";
    case "parallel":
      return "chart.view.parallel";
    case "bar3d":
      return "chart.view.bar3d";
    case "scatter3d":
      return "chart.view.scatter3d";
    case "surface":
      return "chart.view.surface";
    case "line3d":
      return "chart.view.line3d";
    default:
      return "chart.view.custom";
  }
}
