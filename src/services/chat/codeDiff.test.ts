import { describe, expect, it } from "vitest";
import {
  collapseUnchangedRows,
  inlineEditRange,
  inlineRangesForRow,
  type CodeDiffLine,
  type CodeDiffRow,
} from "./codeDiff";

function context(lineNumber: number, text = "keep"): CodeDiffLine {
  return { lineNumber, text, kind: "context" };
}

function row(left: CodeDiffLine | null, right: CodeDiffLine | null = left): CodeDiffRow {
  return { left, right };
}

describe("collapseUnchangedRows", () => {
  it("keeps a tiny file unchanged", () => {
    const rows = [
      row(context(1)),
      row(
        { lineNumber: 2, text: "old", kind: "deletion" },
        { lineNumber: 2, text: "new", kind: "addition" },
      ),
      row(context(3)),
    ];
    expect(collapseUnchangedRows(rows)).toEqual(rows);
  });

  it("folds leading and trailing unchanged lines", () => {
    const rows = [
      ...Array.from({ length: 8 }, (_, index) => row(context(index + 1))),
      row(
        { lineNumber: 9, text: "old", kind: "deletion" },
        { lineNumber: 9, text: "new", kind: "addition" },
      ),
      ...Array.from({ length: 8 }, (_, index) => row(context(index + 10))),
    ];
    const collapsed = collapseUnchangedRows(rows, 3);
    expect(collapsed[0]?.left?.kind).toBe("skip");
    expect(collapsed[0]?.left?.skipped).toBe(5);
    expect(collapsed.at(-1)?.left?.kind).toBe("skip");
    expect(collapsed.at(-1)?.left?.skipped).toBe(5);
    expect(collapsed.some((item) => item.left?.kind === "deletion")).toBe(true);
  });
});

describe("inlineEditRange", () => {
  it("marks the replaced middle of a line", () => {
    expect(inlineEditRange("hello world", "hello there")).toEqual({
      before: { from: 6, to: 11 },
      after: { from: 6, to: 11 },
    });
  });

  it("skips whole-line replacements", () => {
    expect(inlineEditRange("abc", "xyz")).toEqual({ before: null, after: null });
  });
});

describe("inlineRangesForRow", () => {
  it("only pairs a deletion with an addition", () => {
    expect(
      inlineRangesForRow({
        left: { lineNumber: 1, text: "keep old", kind: "deletion" },
        right: { lineNumber: 1, text: "keep new", kind: "addition" },
      }).left,
    ).toEqual({ from: 5, to: 8 });
  });
});
