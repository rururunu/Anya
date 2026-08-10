import { describe, expect, it } from "vitest";
import * as echarts from "echarts/core";
import { Bar3DChart, Line3DChart, Scatter3DChart, SurfaceChart } from "echarts-gl/charts";
import { Grid3DComponent } from "echarts-gl/components";

/**
 * Import-chain guard for the 3D path: echarts-gl must stay bundleable against
 * the installed echarts major (the package historically lagged ECharts
 * releases, so a future echarts bump could silently break this import).
 */
describe("echarts-gl registration", () => {
  it("registers 3D chart types against echarts/core", () => {
    expect(() =>
      echarts.use([Bar3DChart, Line3DChart, Scatter3DChart, SurfaceChart, Grid3DComponent]),
    ).not.toThrow();
  });
});
