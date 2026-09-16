import { describe, expect, it } from "vitest";
import type { TokenUsageReport } from "@/types/tokenUsage";
import {
  MAX_CHART_SERIES,
  OTHER_SERIES_ID,
  bucketIndexAt,
  buildChartSeries,
  buildDonutArcs,
  buildLineSeries,
  buildModelGroups,
  buildModelShareItems,
  buildRadarChart,
  formatSharePercent,
  inferUsageBrand,
  niceMax,
  radarRatio,
} from "./tokenUsageChart";

function emptyUsage() {
  return {
    inputTokens: 0,
    outputTokens: 0,
    systemTokens: 0,
    contextTokens: 0,
    toolCallTokens: 0,
    toolResultTokens: 0,
    memoryTokens: 0,
    totalTokens: 0,
    accuracy: "estimated" as const,
  };
}

function report(partial: Partial<TokenUsageReport> = {}): TokenUsageReport {
  return {
    from: 0,
    to: 1,
    granularity: "day",
    modelCalls: 0,
    total: emptyUsage(),
    byModel: [],
    timeline: [],
    ...partial,
  };
}

describe("inferUsageBrand", () => {
  it("reads the vendor from the model id even on a proxy provider", () => {
    expect(inferUsageBrand("deepseek-v4-pro", "openrouter")).toBe("deepseek");
    expect(inferUsageBrand("anthropic/claude-sonnet-4", "openai")).toBe("claude");
    expect(inferUsageBrand("gpt-4o-mini", "custom")).toBe("openai");
  });

  it("falls back to the stored provider, then other", () => {
    expect(inferUsageBrand("mystery-7b", "kimi")).toBe("kimi");
    expect(inferUsageBrand("local-llama")).toBe("other");
  });
});

describe("buildChartSeries", () => {
  it("keeps the top models and folds the rest into Other", () => {
    const models = Array.from({ length: MAX_CHART_SERIES + 2 }, (_, index) => ({
      model: `m${index}`,
      provider: "p",
      usage: { ...emptyUsage(), totalTokens: 100 - index },
      calls: 1,
      share: 0.1,
    }));
    const timeline = [
      {
        bucket: "a",
        label: "01/01",
        totalTokens: 10,
        inputTokens: 0,
        outputTokens: 0,
        models: Object.fromEntries(models.map((item) => [item.model, 4])),
      },
    ];
    const series = buildChartSeries(report({ byModel: models, timeline }), "Other");
    expect(series).toHaveLength(MAX_CHART_SERIES + 1);
    expect(series[series.length - 1]?.id).toBe(OTHER_SERIES_ID);
    expect(series[series.length - 1]?.values[0]).toBe(8);
  });
});

describe("buildModelGroups", () => {
  it("groups models by inferred brand", () => {
    const groups = buildModelGroups(
      report({
        byModel: [
          {
            model: "deepseek-v4-pro",
            provider: "deepseek",
            usage: { ...emptyUsage(), totalTokens: 80 },
            calls: 3,
            share: 0.8,
          },
          {
            model: "deepseek-chat",
            provider: "openrouter",
            usage: { ...emptyUsage(), totalTokens: 20 },
            calls: 1,
            share: 0.2,
          },
        ],
      }),
    );
    expect(groups).toHaveLength(1);
    expect(groups[0]?.id).toBe("deepseek");
    expect(groups[0]?.models).toHaveLength(2);
    expect(groups[0]?.models[0]?.label).toBe("v4-pro");
  });

  it("strips vendor prefixes from OpenRouter-style ids", () => {
    const groups = buildModelGroups(
      report({
        byModel: [
          {
            model: "anthropic/claude-sonnet-4",
            provider: "openrouter",
            usage: { ...emptyUsage(), totalTokens: 10 },
            calls: 1,
            share: 1,
          },
        ],
      }),
    );
    expect(groups[0]?.id).toBe("claude");
    expect(groups[0]?.models[0]?.label).toBe("claude-sonnet-4");
  });
});

describe("niceMax", () => {
  it("rounds up to 1-2-5 scale", () => {
    expect(niceMax(0)).toBe(1);
    expect(niceMax(12)).toBe(20);
    expect(niceMax(3500)).toBe(5000);
  });
});

describe("bucketIndexAt", () => {
  it("maps chart x to a bar slot", () => {
    expect(bucketIndexAt(8, 10)).toBe(0);
    expect(bucketIndexAt(752, 10)).toBe(9);
  });
});

describe("buildDonutArcs", () => {
  it("skips empty slices and keeps share totals at 1", () => {
    const arcs = buildDonutArcs([
      { id: "a", label: "A", color: "#000", value: 75 },
      { id: "b", label: "B", color: "#fff", value: 25 },
      { id: "c", label: "C", color: "#aaa", value: 0 },
    ]);
    expect(arcs).toHaveLength(2);
    expect(arcs[0]?.share).toBe(0.75);
    expect(arcs[1]?.share).toBe(0.25);
    expect(arcs[0]?.path.startsWith("M ")).toBe(true);
  });

  it("draws a full ring for a single slice", () => {
    const arcs = buildDonutArcs([{ id: "a", label: "A", color: "#000", value: 10 }]);
    expect(arcs).toHaveLength(1);
    expect(arcs[0]?.share).toBe(1);
    expect(arcs[0]?.path.split("M ").length).toBeGreaterThan(2);
  });
});

describe("formatSharePercent", () => {
  it("rounds ordinary shares and marks tiny ones", () => {
    expect(formatSharePercent(0)).toBe("0%");
    expect(formatSharePercent(0.004)).toBe("<1%");
    expect(formatSharePercent(0.62)).toBe("62%");
  });
});

describe("buildModelShareItems", () => {
  it("uses brand slices when more than one provider is present", () => {
    const groups = buildModelGroups(
      report({
        byModel: [
          {
            model: "deepseek-v4-pro",
            provider: "deepseek",
            usage: { ...emptyUsage(), totalTokens: 80 },
            calls: 3,
            share: 0.8,
          },
          {
            model: "gpt-4o-mini",
            provider: "openai",
            usage: { ...emptyUsage(), totalTokens: 20 },
            calls: 1,
            share: 0.2,
          },
        ],
      }),
    );
    const items = buildModelShareItems(groups);
    expect(items.map((item) => item.id)).toEqual(["deepseek", "openai"]);
    expect(items[0]?.children).toHaveLength(1);
  });
});

describe("radarRatio", () => {
  it("keeps small values visible against a large max", () => {
    expect(radarRatio(0, 1000)).toBe(0);
    expect(radarRatio(1000, 1000)).toBe(1);
    expect(radarRatio(10, 1000)).toBeGreaterThan(0.3);
  });
});

describe("buildRadarChart", () => {
  it("places one vertex per category", () => {
    const chart = buildRadarChart([
      { id: "a", label: "A", color: "#000", value: 80 },
      { id: "b", label: "B", color: "#111", value: 20 },
      { id: "c", label: "C", color: "#222", value: 0 },
    ]);
    expect(chart.axes).toHaveLength(3);
    expect(chart.polygon.split(" ")).toHaveLength(3);
    expect(chart.axes[2]?.ratio).toBe(0);
  });
});

describe("buildLineSeries", () => {
  it("maps each series onto shared x positions", () => {
    const timeline = [
      {
        bucket: "a",
        label: "01",
        totalTokens: 10,
        inputTokens: 0,
        outputTokens: 0,
        models: { m: 4 },
      },
      {
        bucket: "b",
        label: "02",
        totalTokens: 20,
        inputTokens: 0,
        outputTokens: 0,
        models: { m: 8 },
      },
    ];
    const series = [
      { id: "m", label: "M", color: "#000", values: [4, 8] },
      { id: "n", label: "N", color: "#111", values: [1, 2] },
    ];
    const lines = buildLineSeries(timeline, series);
    expect(lines.series).toHaveLength(2);
    expect(lines.series[0]?.points).toHaveLength(2);
    expect(lines.series[0]?.path.startsWith("M ")).toBe(true);
    expect(lines.max).toBe(10);
  });
});
