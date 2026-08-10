import { describe, expect, it } from "vitest";
import { parseChartSpec } from "./chartSpec";

describe("parseChartSpec", () => {
  it("parses a valid bar spec", () => {
    const spec = parseChartSpec(
      JSON.stringify({
        type: "bar",
        title: "营收",
        unit: "万元",
        x: ["1月", "2月"],
        series: [{ name: "收入", data: [120, 150] }],
      }),
    );
    expect(spec).toEqual({
      type: "bar",
      title: "营收",
      unit: "万元",
      x: ["1月", "2月"],
      series: [{ name: "收入", data: [120, 150] }],
    });
  });

  it("parses a valid line spec with unnamed series", () => {
    const spec = parseChartSpec(
      JSON.stringify({ type: "line", x: ["a"], series: [{ data: [1] }] }),
    );
    expect(spec).toEqual({ type: "line", x: ["a"], series: [{ data: [1] }] });
  });

  it("parses a valid pie spec", () => {
    const spec = parseChartSpec(JSON.stringify({ type: "pie", items: [{ name: "A", value: 45 }] }));
    expect(spec).toEqual({ type: "pie", items: [{ name: "A", value: 45 }] });
  });

  it("rejects invalid JSON", () => {
    expect(parseChartSpec("{not json")).toBeNull();
    expect(parseChartSpec("")).toBeNull();
  });

  it("rejects unknown or missing types", () => {
    expect(
      parseChartSpec(JSON.stringify({ type: "map", x: ["a"], series: [{ data: [1] }] })),
    ).toBeNull();
    expect(parseChartSpec(JSON.stringify({ type: "wordcloud" }))).toBeNull();
    expect(parseChartSpec(JSON.stringify({}))).toBeNull();
    expect(parseChartSpec(JSON.stringify({ type: 3 }))).toBeNull();
  });

  it("rejects bar/line without x or series", () => {
    expect(parseChartSpec(JSON.stringify({ type: "line" }))).toBeNull();
    expect(
      parseChartSpec(JSON.stringify({ type: "line", x: [], series: [{ data: [1] }] })),
    ).toBeNull();
    expect(parseChartSpec(JSON.stringify({ type: "line", x: ["a"], series: [] }))).toBeNull();
    expect(
      parseChartSpec(JSON.stringify({ type: "line", x: ["a"], series: [{ data: [] }] })),
    ).toBeNull();
  });

  it("rejects non-numeric or empty series data", () => {
    expect(
      parseChartSpec(JSON.stringify({ type: "bar", x: ["a"], series: [{ data: [1, "x"] }] })),
    ).toBeNull();
    expect(
      parseChartSpec(JSON.stringify({ type: "bar", x: ["a"], series: [{ data: [] }] })),
    ).toBeNull();
    expect(
      parseChartSpec(JSON.stringify({ type: "bar", x: ["a"], series: [{ data: [NaN] }] })),
    ).toBeNull();
  });

  it("rejects pie without items or with invalid values", () => {
    expect(parseChartSpec(JSON.stringify({ type: "pie" }))).toBeNull();
    expect(parseChartSpec(JSON.stringify({ type: "pie", items: [] }))).toBeNull();
    expect(
      parseChartSpec(JSON.stringify({ type: "pie", items: [{ name: "A", value: null }] })),
    ).toBeNull();
    expect(parseChartSpec(JSON.stringify({ type: "pie", items: [{ value: 1 }] }))).toBeNull();
  });

  it("trims optional title/unit and drops blanks", () => {
    const spec = parseChartSpec(
      JSON.stringify({
        type: "line",
        title: "  ",
        unit: " 件 ",
        x: ["a"],
        series: [{ data: [1] }],
      }),
    );
    expect(spec?.title).toBeUndefined();
    expect(spec?.unit).toBe("件");
  });

  it("rejects non-object payloads", () => {
    expect(parseChartSpec("[1,2]")).toBeNull();
    expect(parseChartSpec("42")).toBeNull();
    expect(parseChartSpec("null")).toBeNull();
  });

  it("parses scatter with [x, y] point tuples", () => {
    const spec = parseChartSpec(
      JSON.stringify({
        type: "scatter",
        series: [
          {
            name: "A",
            data: [
              [1, 2],
              [3, 4],
            ],
          },
        ],
      }),
    );
    expect(spec).toEqual({
      type: "scatter",
      series: [
        {
          name: "A",
          data: [
            [1, 2],
            [3, 4],
          ],
        },
      ],
    });
    expect(
      parseChartSpec(JSON.stringify({ type: "scatter", series: [{ data: [[1]] }] })),
    ).toBeNull();
    expect(
      parseChartSpec(JSON.stringify({ type: "scatter", series: [{ data: [[1, "x"]] }] })),
    ).toBeNull();
  });

  it("parses funnel and gauge with optional bounds", () => {
    const funnel = parseChartSpec(
      JSON.stringify({ type: "funnel", items: [{ name: "A", value: 10 }] }),
    );
    expect(funnel?.type).toBe("funnel");
    const gauge = parseChartSpec(
      JSON.stringify({ type: "gauge", items: [{ name: "CPU", value: 63 }], min: 0, max: 100 }),
    );
    expect(gauge).toEqual({ type: "gauge", items: [{ name: "CPU", value: 63 }], min: 0, max: 100 });
    expect(
      parseChartSpec(
        JSON.stringify({ type: "gauge", items: [{ name: "A", value: 1 }], min: 5, max: 1 }),
      ),
    ).toBeNull();
    expect(
      parseChartSpec(JSON.stringify({ type: "gauge", items: [{ name: "A", value: "x" }] })),
    ).toBeNull();
  });

  it("parses radar and matches value count to indicators", () => {
    const spec = parseChartSpec(
      JSON.stringify({
        type: "radar",
        indicators: [
          { name: "速度", max: 10 },
          { name: "稳定", max: 10 },
        ],
        series: [{ name: "A", data: [8, 9] }],
      }),
    );
    expect(spec?.series?.[0].data).toEqual([8, 9]);
    expect(
      parseChartSpec(
        JSON.stringify({
          type: "radar",
          indicators: [{ name: "速度", max: 10 }],
          series: [{ data: [8, 9] }],
        }),
      ),
    ).toBeNull();
    expect(
      parseChartSpec(JSON.stringify({ type: "radar", indicators: [], series: [{ data: [] }] })),
    ).toBeNull();
  });

  it("parses heatmap with bounded cell indices", () => {
    const spec = parseChartSpec(
      JSON.stringify({
        type: "heatmap",
        x: ["a", "b"],
        y: ["x", "y", "z"],
        series: [
          {
            data: [
              [0, 0, 1],
              [1, 2, 5],
            ],
          },
        ],
      }),
    );
    expect(spec?.series?.[0].data).toEqual([
      [0, 0, 1],
      [1, 2, 5],
    ]);
    expect(
      parseChartSpec(
        JSON.stringify({ type: "heatmap", x: ["a"], y: ["x"], series: [{ data: [[1, 0, 3]] }] }),
      ),
    ).toBeNull();
    expect(
      parseChartSpec(
        JSON.stringify({ type: "heatmap", x: ["a"], y: ["x"], series: [{ data: [[0, 0]] }] }),
      ),
    ).toBeNull();
  });

  it("parses candlestick with [open, close, low, high] tuples", () => {
    const spec = parseChartSpec(
      JSON.stringify({ type: "candlestick", x: ["D1"], series: [{ data: [[100, 110, 95, 115]] }] }),
    );
    expect(spec?.series?.[0].data).toEqual([[100, 110, 95, 115]]);
    expect(
      parseChartSpec(
        JSON.stringify({ type: "candlestick", x: ["D1"], series: [{ data: [[1, 2, 3]] }] }),
      ),
    ).toBeNull();
  });

  it("parses nested treemap trees", () => {
    const spec = parseChartSpec(
      JSON.stringify({
        type: "treemap",
        data: [{ name: "A", value: 10, children: [{ name: "A1", value: 4 }] }],
      }),
    );
    expect(spec?.data?.[0].children?.[0].name).toBe("A1");
    expect(parseChartSpec(JSON.stringify({ type: "treemap", data: [{ value: 1 }] }))).toBeNull();
    expect(parseChartSpec(JSON.stringify({ type: "treemap", data: [] }))).toBeNull();
  });

  it("parses sankey and graph with nodes and links", () => {
    const nodes = [{ name: "A" }, { name: "B" }];
    const links = [{ source: "A", target: "B", value: 5 }];
    for (const type of ["sankey", "graph"] as const) {
      const spec = parseChartSpec(JSON.stringify({ type, nodes, links }));
      expect(spec?.nodes).toEqual(nodes);
      expect(spec?.links).toEqual(links);
    }
    expect(parseChartSpec(JSON.stringify({ type: "graph", nodes }))).toBeNull();
    expect(
      parseChartSpec(
        JSON.stringify({ type: "sankey", nodes, links: [{ source: 1, target: "B" }] }),
      ),
    ).toBeNull();
  });

  it("parses parallel with matching row widths", () => {
    const spec = parseChartSpec(
      JSON.stringify({
        type: "parallel",
        dimensions: ["温度", "湿度"],
        series: [
          {
            data: [
              [20, 60],
              [25, 70],
            ],
          },
        ],
      }),
    );
    expect(spec?.series?.[0].data).toEqual([
      [20, 60],
      [25, 70],
    ]);
    expect(
      parseChartSpec(
        JSON.stringify({ type: "parallel", dimensions: ["温度"], series: [{ data: [[1, 2]] }] }),
      ),
    ).toBeNull();
  });

  it("parses bar3d with bounded cell indices", () => {
    const spec = parseChartSpec(
      JSON.stringify({
        type: "bar3d",
        x: ["周一", "周二"],
        y: ["上午", "下午"],
        series: [
          {
            data: [
              [0, 0, 120],
              [1, 1, 80],
            ],
          },
        ],
      }),
    );
    expect(spec?.series?.[0].data).toEqual([
      [0, 0, 120],
      [1, 1, 80],
    ]);
    expect(
      parseChartSpec(
        JSON.stringify({
          type: "bar3d",
          x: ["周一"],
          y: ["上午"],
          series: [{ data: [[2, 0, 5]] }],
        }),
      ),
    ).toBeNull();
  });

  it("parses scatter3d, surface and line3d with [x, y, z] tuples", () => {
    for (const type of ["scatter3d", "surface", "line3d"] as const) {
      const spec = parseChartSpec(JSON.stringify({ type, series: [{ data: [[1, 2, 3]] }] }));
      expect(spec?.type).toBe(type);
      expect(spec?.series?.[0].data).toEqual([[1, 2, 3]]);
    }
    expect(
      parseChartSpec(JSON.stringify({ type: "scatter3d", series: [{ data: [[1, 2]] }] })),
    ).toBeNull();
  });

  it("parses custom option passthrough when every series type is whitelisted", () => {
    const spec = parseChartSpec(
      JSON.stringify({
        type: "custom",
        title: "层级占比",
        option: { series: [{ type: "sunburst", data: [{ name: "a", value: 1 }] }] },
      }),
    );
    expect(spec).toEqual({
      type: "custom",
      title: "层级占比",
      option: { series: [{ type: "sunburst", data: [{ name: "a", value: 1 }] }] },
    });
    for (const type of ["sunburst", "map", "boxplot", "custom", "bar3D", "line3D"]) {
      expect(
        parseChartSpec(JSON.stringify({ type: "custom", option: { series: [{ type }] } })),
      ).not.toBeNull();
    }
    // Lowercase 3D variants are normalized to ECharts' canonical names.
    expect(
      parseChartSpec(JSON.stringify({ type: "custom", option: { series: [{ type: "bar3d" }] } }))
        ?.option,
    ).toEqual({ series: [{ type: "bar3D" }] });
  });

  it("rejects custom passthrough series types that are not registered", () => {
    // wordCloud is not an ECharts built-in and is not registered: passing it
    // through would leave an undefined series model in the scheduler and
    // crash with `getProgressive` on undefined.
    expect(
      parseChartSpec(
        JSON.stringify({ type: "custom", option: { series: [{ type: "wordCloud" }] } }),
      ),
    ).toBeNull();
    expect(
      parseChartSpec(
        JSON.stringify({ type: "custom", option: { series: [{ type: "heatmap3d" }] } }),
      ),
    ).toBeNull();
    expect(
      parseChartSpec(JSON.stringify({ type: "custom", option: { series: [{ type: " bar" }] } })),
    ).toBeNull();
    expect(parseChartSpec(JSON.stringify({ type: "custom", option: { series: [{}] } }))).toBeNull();
    expect(
      parseChartSpec(JSON.stringify({ type: "custom", option: { series: [{ data: [1] }] } })),
    ).toBeNull();
    expect(
      parseChartSpec(JSON.stringify({ type: "custom", option: { series: "bar" } })),
    ).toBeNull();
    expect(
      parseChartSpec(JSON.stringify({ type: "custom", option: { series: [null] } })),
    ).toBeNull();
    expect(parseChartSpec(JSON.stringify({ type: "custom" }))).toBeNull();
    expect(parseChartSpec(JSON.stringify({ type: "custom", option: [] }))).toBeNull();
    expect(parseChartSpec(JSON.stringify({ type: "custom", option: "option" }))).toBeNull();
  });
});
