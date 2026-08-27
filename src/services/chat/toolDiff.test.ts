import { describe, expect, it } from "vitest";
import { fileBasename, fileParentDir, hunkFromPlainEdit, parseUnifiedDiffHunks } from "./toolDiff";

describe("parseUnifiedDiffHunks", () => {
  it("keeps interleaved context and change lines with line numbers", () => {
    const diff = [
      "--- a/limits.rs",
      "+++ b/limits.rs",
      "@@ -29,3 +29,3 @@",
      " pub const MAX_CONSECUTIVE_TOOL_FAILURES: u32 = 3;",
      "-/// Mid-turn auto-compact window when large context is off.",
      "+/// Default mid-turn auto-compact window.",
      " pub const DEFAULT_MAX_TURN_TOKENS: usize = 64_000;",
    ].join("\n");

    const hunks = parseUnifiedDiffHunks(diff);
    expect(hunks).toHaveLength(1);
    expect(hunks[0].added).toBe(1);
    expect(hunks[0].removed).toBe(1);
    expect(hunks[0].lines.map((line) => line.kind)).toEqual([
      "context",
      "deletion",
      "addition",
      "context",
    ]);
    expect(hunks[0].lines[0].oldNo).toBe(29);
    expect(hunks[0].lines[0].newNo).toBe(29);
    expect(hunks[0].lines[1].oldNo).toBe(30);
    expect(hunks[0].lines[2].newNo).toBe(30);
  });
});

describe("hunkFromPlainEdit", () => {
  it("builds deletion then addition blocks", () => {
    const hunk = hunkFromPlainEdit("old\nline", "new");
    expect(hunk.removed).toBe(2);
    expect(hunk.added).toBe(1);
    expect(hunk.lines[0]).toMatchObject({ kind: "deletion", oldNo: 1, text: "old" });
    expect(hunk.lines[2]).toMatchObject({ kind: "addition", newNo: 1, text: "new" });
  });
});

describe("fileBasename", () => {
  it("returns the last path segment", () => {
    expect(fileBasename("src/core/chat/limits.rs")).toBe("limits.rs");
    expect(fileBasename("src\\core\\chat\\service.rs")).toBe("service.rs");
  });
});

describe("fileParentDir", () => {
  it("returns the directory without the file name", () => {
    expect(fileParentDir("src/main/java/VirtualThreadsDemo.java")).toBe("src/main/java");
    expect(fileParentDir("VirtualThreadsDemo.java")).toBe("");
  });
});
