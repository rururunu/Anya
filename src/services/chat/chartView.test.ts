import { describe, expect, it } from "vitest";
import type { ChartSpec } from "./chartSpec";
import { chartViewsFor, defaultChartView, shouldUseHorizontalBar } from "./chartView";

const grouped: ChartSpec = {
  type: "bar",
  title: "文档体积变化 (KB)",
  unit: "KB",
  x: ["TECHNICAL.md", "TECHNICAL.zh-CN.md"],
  series: [
    { name: "重写前", data: [16, 15] },
    { name: "重写后", data: [30, 29] },
  ],
};

describe("chartViewsFor", () => {
  it("offers grouped, horizontal, stacked, line, and area for cartesian data", () => {
    expect(chartViewsFor(grouped).map((view) => view.id)).toEqual([
      "bar",
      "bar-h",
      "bar-stacked",
      "bar-h-stacked",
      "line",
      "area",
    ]);
  });

  it("lets pie specs switch to funnel", () => {
    expect(
      chartViewsFor({ type: "pie", items: [{ name: "A", value: 1 }] }).map((view) => view.id),
    ).toEqual(["pie", "funnel"]);
  });
});

describe("defaultChartView", () => {
  it("uses a horizontal bar when category labels are long filenames", () => {
    expect(shouldUseHorizontalBar(grouped)).toBe(true);
    expect(defaultChartView(grouped).id).toBe("bar-h");
  });

  it("keeps a vertical bar for short month labels", () => {
    const spec: ChartSpec = {
      type: "bar",
      x: ["1月", "2月", "3月"],
      series: [{ data: [1, 2, 3] }],
    };
    expect(defaultChartView(spec).id).toBe("bar");
  });
});
