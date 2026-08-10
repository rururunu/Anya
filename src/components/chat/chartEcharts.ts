/**
 * ECharts registration split for code-splitting.
 *
 * Core + everyday series stay in the ChartCard async chunk. Less common 2D
 * series load from a separate chunk on first use so the ChartCard entry stays
 * under Vite's 500 kB warning. 3D (echarts-gl) remains separately lazy.
 */
import { BarChart, LineChart, PieChart, ScatterChart } from "echarts/charts";
import {
  GridComponent,
  LegendComponent,
  TooltipComponent,
  VisualMapComponent,
} from "echarts/components";
import * as echarts from "echarts/core";
import { CanvasRenderer } from "echarts/renderers";
import type { ChartType } from "@/services/chat/chartSpec";

echarts.use([
  BarChart,
  LineChart,
  ScatterChart,
  PieChart,
  GridComponent,
  LegendComponent,
  TooltipComponent,
  VisualMapComponent,
  CanvasRenderer,
]);

/** Series that ship with the ChartCard entry chunk. */
const CORE_TYPES = new Set<ChartType>(["bar", "line", "scatter", "pie"]);

/** 3D types — handled by ChartCard's echarts-gl loader. */
const GL_TYPES = new Set<ChartType>(["bar3d", "scatter3d", "surface", "line3d"]);

let extraRegistered = false;
let extraLoading: Promise<void> | null = null;

async function registerExtraCharts() {
  if (extraRegistered) return;
  if (!extraLoading) {
    extraLoading = import("./chartEchartsExtra").then((mod) => {
      mod.registerExtraCharts(echarts);
      extraRegistered = true;
    });
  }
  await extraLoading;
}

/**
 * Ensure every series class needed for `type` (and custom passthrough series)
 * is registered before setOption.
 */
export async function ensureChartModules(
  type: ChartType,
  customSeriesTypes: readonly string[] = [],
): Promise<void> {
  if (GL_TYPES.has(type)) return;
  const needsExtra =
    !CORE_TYPES.has(type) ||
    customSeriesTypes.some((seriesType) => {
      const lower = seriesType.toLowerCase();
      return !(lower === "bar" || lower === "line" || lower === "scatter" || lower === "pie");
    });
  if (needsExtra) {
    await registerExtraCharts();
  }
}

export { echarts, GL_TYPES };
