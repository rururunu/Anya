import { describe, expect, it } from "vitest";
import { optimisticUserMatchesServer } from "./optimisticMatch";

describe("optimisticUserMatchesServer", () => {
  it("matches exact content", () => {
    expect(optimisticUserMatchesServer("hello", "hello")).toBe(true);
  });

  it("matches when the server appends a checklist after the approve text", () => {
    const local = "计划已批准。现在严格按临时方案（.anya/plan.md）执行，本回合写操作已解除限制。";
    const server = `${local}\n\n[System] Plan approved. Execute...`;
    expect(optimisticUserMatchesServer(local, server, true)).toBe(true);
  });

  it("does not match unrelated messages", () => {
    expect(optimisticUserMatchesServer("fix the bug", "write tests", true)).toBe(false);
  });
});
