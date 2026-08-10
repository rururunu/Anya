/**
 * Less-common ECharts series — loaded only when a chart needs them.
 * Keep in sync with CUSTOM_SERIES_TYPES / ChartCard renderers.
 */
import {
  BoxplotChart,
  CandlestickChart,
  CustomChart,
  FunnelChart,
  GaugeChart,
  GraphChart,
  HeatmapChart,
  MapChart,
  ParallelChart,
  RadarChart,
  SankeyChart,
  SunburstChart,
  TreemapChart,
} from "echarts/charts";
import { ParallelComponent, RadarComponent } from "echarts/components";
import type * as echartsCore from "echarts/core";

export function registerExtraCharts(echarts: typeof echartsCore): void {
  echarts.use([
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
    ParallelComponent,
    RadarComponent,
  ]);
}
