import { describe, expect, it } from "vitest";
import { formatTokenCount, promptCacheHitPercent } from "./tokenEstimate";

describe("formatTokenCount", () => {
  it("keeps small counts plain", () => {
    expect(formatTokenCount(0)).toBe("0");
    expect(formatTokenCount(999)).toBe("999");
  });

  it("uses k below one million even for zh-CN", () => {
    expect(formatTokenCount(1_000, "zh-CN")).toBe("1k");
    expect(formatTokenCount(6_300, "zh-CN")).toBe("6.3k");
    expect(formatTokenCount(63_000, "zh-CN")).toBe("63k");
    expect(formatTokenCount(126_000, "zh-CN")).toBe("126k");
    expect(formatTokenCount(999_999, "zh-CN")).toBe("999.9k");
  });

  it("uses M at one million and above", () => {
    expect(formatTokenCount(1_000_000, "zh-CN")).toBe("1M");
    expect(formatTokenCount(1_260_000)).toBe("1.3M");
  });
});

describe("promptCacheHitPercent", () => {
  it("returns the cached share of prompt tokens", () => {
    expect(promptCacheHitPercent(20, 80)).toBe(80);
    expect(promptCacheHitPercent(100, 0)).toBe(0);
    expect(promptCacheHitPercent(0, 0)).toBeNull();
  });
});
