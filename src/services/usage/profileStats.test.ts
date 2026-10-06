import { describe, expect, it } from "vitest";
import { activityDays, localDayKey, profileStats, formatProfileTokens } from "./profileStats";
import type { TokenUsageReport } from "@/types/tokenUsage";

function report(days: [string, number][]) {
  return {
    timeline: days.map(([bucket, totalTokens]) => ({ bucket, totalTokens })),
  } as TokenUsageReport;
}

describe("profile activity statistics", () => {
  it.each([
    [999, "999"],
    [1260, "1.26千"],
    [12600, "1.26万"],
    [1260000, "126万"],
    [126000000, "1.26亿"],
    [9999, "1万"],
  ])("formats %i tokens as %s", (value, expected) => {
    expect(formatProfileTokens(value, "zh-CN")).toBe(expected);
  });
  it("merges same-day records and counts streaks across month boundaries", () => {
    const result = profileStats(
      report([
        ["2026-09-29T12:00:00", 10],
        ["2026-09-30T12:00:00", 20],
        ["2026-10-01T12:00:00", 30],
        ["2026-10-01T13:00:00", 40],
        ["2026-10-04T12:00:00", 5],
        ["2026-10-05T12:00:00", 6],
      ]),
      new Date(2026, 9, 6),
    );
    expect(result.peak).toBe(70);
    expect(result.longest).toBe(3);
    expect(result.current).toBe(2);
  });
  it("ends the current streak after a missed day and handles empty histories", () => {
    expect(profileStats(report([["2026-10-03T12:00:00", 8]]), new Date(2026, 9, 6)).current).toBe(
      0,
    );
    expect(profileStats(report([])).longest).toBe(0);
  });
  it("builds complete calendar weeks without day gaps at daylight saving boundaries", () => {
    const days = activityDays(new Date(2026, 9, 6, 12));
    expect(days.length % 7).toBe(0);
    expect(days[0].date.getDay()).toBe(0);
    expect(days.at(-1)!.date.getDay()).toBe(6);
    expect(new Set(days.map((day) => day.key)).size).toBe(days.length);
    expect(days.find((day) => day.key === localDayKey(new Date(2026, 9, 6)))?.future).toBe(false);
  });
});
