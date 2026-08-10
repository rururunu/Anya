import { beforeAll, describe, expect, it } from "vitest";
import * as echarts from "echarts/core";
import {
  BarChart,
  BoxplotChart,
  CandlestickChart,
  CustomChart,
  FunnelChart,
  GaugeChart,
  GraphChart,
  HeatmapChart,
  LineChart,
  MapChart,
  ParallelChart,
  PieChart,
  RadarChart,
  SankeyChart,
  ScatterChart,
  SunburstChart,
  TreemapChart,
} from "echarts/charts";
import {
  GridComponent,
  LegendComponent,
  ParallelComponent,
  RadarComponent,
  TooltipComponent,
  VisualMapComponent,
} from "echarts/components";
import { SVGRenderer } from "echarts/renderers";

// Keep in sync with ChartCard.vue's echarts.use([...]) — this mirrors the
// exact registration state of the running app (SVGRenderer stands in for
// CanvasRenderer: the failure modes under test live in the model pipeline,
// which runs identically for both renderers).
echarts.use([
  BarChart,
  LineChart,
  ScatterChart,
  PieChart,
  FunnelChart,
  GaugeChart,
  RadarChart,
  HeatmapChart,
  CandlestickChart,
  TreemapChart,
  SankeyChart,
  GraphChart,
  ParallelChart,
  CustomChart,
  SunburstChart,
  MapChart,
  BoxplotChart,
  GridComponent,
  LegendComponent,
  TooltipComponent,
  VisualMapComponent,
  ParallelComponent,
  RadarComponent,
  SVGRenderer,
]);

/**
 * Regression guard for the user-reported crash
 * `Cannot read properties of undefined (reading 'getProgressive')` at
 * Scheduler.restorePipelines: an option whose series entry resolves to no
 * series model leaves an undefined model in the scheduler, and with a canvas
 * painter the scheduler calls `seriesModel.getProgressive()` on it.
 *
 * Node has no canvas, so we init in SSR/SVG mode (identical model pipeline)
 * and then flip the painter type to 'canvas' to force the exact code path
 * from the crash (Scheduler.js: `zr.painter.type === 'canvas' &&
 * seriesModel.getProgressive()`).
 */
async function setOptionOutcome(option: unknown): Promise<{ crashed: boolean; message: string }> {
  const chart = echarts.init(null, null, { renderer: "svg", ssr: true, width: 600, height: 400 });
  (chart.getZr() as { painter: { type: string } }).painter.type = "canvas";
  try {
    await chart.setOption(option as never);
    return { crashed: false, message: "" };
  } catch (error) {
    return { crashed: true, message: error instanceof Error ? error.message : String(error) };
  } finally {
    chart.dispose();
  }
}

describe("every supported 2D chart renders without the getProgressive crash", () => {
  it.each([
    [
      "bar",
      {
        xAxis: { type: "category", data: ["A"] },
        yAxis: { type: "value" },
        series: [{ type: "bar", data: [1] }],
      },
    ],
    [
      "line",
      {
        xAxis: { type: "category", data: ["A"] },
        yAxis: { type: "value" },
        series: [{ type: "line", data: [1] }],
      },
    ],
    [
      "scatter",
      {
        xAxis: { type: "value" },
        yAxis: { type: "value" },
        series: [{ type: "scatter", data: [[1, 2]] }],
      },
    ],
    ["pie", { series: [{ type: "pie", data: [{ name: "a", value: 1 }] }] }],
    ["funnel", { series: [{ type: "funnel", data: [{ name: "a", value: 1 }] }] }],
    ["gauge", { series: [{ type: "gauge", data: [{ name: "a", value: 50 }] }] }],
    [
      "radar",
      {
        radar: { indicator: [{ name: "a", max: 100 }] },
        series: [{ type: "radar", data: [{ value: [50] }] }],
      },
    ],
    [
      "heatmap",
      {
        xAxis: { type: "category", data: ["A"] },
        yAxis: { type: "category", data: ["X"] },
        visualMap: { min: 0, max: 1 },
        series: [{ type: "heatmap", data: [[0, 0, 1]] }],
      },
    ],
    [
      "candlestick",
      {
        xAxis: { type: "category", data: ["A"] },
        yAxis: { type: "value" },
        series: [{ type: "candlestick", data: [[1, 2, 0.5, 1.5]] }],
      },
    ],
    ["treemap", { series: [{ type: "treemap", data: [{ name: "a", value: 1 }] }] }],
    [
      "sankey",
      {
        series: [
          {
            type: "sankey",
            data: [{ name: "a" }, { name: "b" }],
            links: [{ source: "a", target: "b", value: 1 }],
          },
        ],
      },
    ],
    ["graph", { series: [{ type: "graph", layout: "force", data: [{ name: "a" }], links: [] }] }],
    [
      "parallel",
      { parallel: {}, parallelAxis: [{ dim: 0 }], series: [{ type: "parallel", data: [[1]] }] },
    ],
  ])("%s", async (_name, option) => {
    const { crashed, message } = await setOptionOutcome(option);
    expect({ crashed, message }).toEqual({ crashed: false, message: "" });
  });
});

describe("custom passthrough charts in the whitelist render without the crash", () => {
  it.each([
    ["sunburst", { series: [{ type: "sunburst", data: [{ name: "a", value: 1 }] }] }],
    [
      "boxplot",
      {
        xAxis: { type: "category", data: ["A"] },
        yAxis: { type: "value" },
        series: [{ type: "boxplot", data: [[1, 2, 3, 4, 5]] }],
      },
    ],
    // A JSON passthrough can never carry a renderItem function, so a bare
    // custom series fails at the view stage — but it must fail cleanly, not
    // with the scheduler's undefined-series-model crash.
    [
      "custom",
      {
        xAxis: { type: "category", data: ["A"] },
        yAxis: { type: "value" },
        series: [{ type: "custom", data: [] }],
      },
    ],
  ])("%s", async (_name, option) => {
    const { message } = await setOptionOutcome(option);
    expect(message).not.toContain("getProgressive");
    expect(message).not.toContain("undefined (reading 'getProgressive')");
  });
});

describe("3D charts (echarts-gl registered) do not fail in the model pipeline", () => {
  beforeAll(async () => {
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
  });

  const glAxes = {
    axisLine: { lineStyle: { color: "#ccc" } },
    axisLabel: { color: "#888", fontSize: 10 },
    splitLine: { lineStyle: { color: "#ccc", opacity: 0.4 } },
  };

  it.each([
    [
      "bar3D",
      {
        grid3D: {},
        xAxis3D: { type: "category", data: ["A"] },
        yAxis3D: { type: "category", data: ["X"] },
        zAxis3D: { type: "value" },
        series: [{ type: "bar3D", data: [[0, 0, 1]] }],
      },
    ],
    [
      "scatter3D",
      {
        grid3D: {},
        xAxis3D: { type: "value", ...glAxes },
        yAxis3D: { type: "value", ...glAxes },
        zAxis3D: { type: "value", ...glAxes },
        series: [{ type: "scatter3D", data: [[1, 2, 3]] }],
      },
    ],
    [
      "line3D",
      {
        grid3D: {},
        xAxis3D: { type: "value", ...glAxes },
        yAxis3D: { type: "value", ...glAxes },
        zAxis3D: { type: "value", ...glAxes },
        series: [{ type: "line3D", data: [[1, 2, 3]] }],
      },
    ],
    [
      "surface",
      {
        grid3D: {},
        xAxis3D: { type: "value", ...glAxes },
        yAxis3D: { type: "value", ...glAxes },
        zAxis3D: { type: "value", ...glAxes },
        series: [{ type: "surface", data: [[0, 0, 1]] }],
      },
    ],
  ])("%s clears the model pipeline (no getProgressive crash)", async (_name, option) => {
    const { message } = await setOptionOutcome(option);
    // The WebGL render stage cannot run in node (null DOM root), so a DOM
    // artifact error is expected — but it must not be the scheduler crash.
    expect(message).not.toContain("getProgressive");
    expect(message).not.toContain("undefined (reading 'getProgressive')");
  });
});
