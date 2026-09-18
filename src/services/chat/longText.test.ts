import { describe, expect, it } from "vitest";
import {
  countTextLines,
  pastedTextFilename,
  shouldAttachPasteAsFile,
  shouldFoldSoftInject,
  shouldFoldUserMessage,
  PASTE_AS_FILE_MIN_CHARS,
  PASTE_AS_FILE_MIN_LINES,
  SOFT_INJECT_FOLD_MIN_CHARS,
  SOFT_INJECT_FOLD_MIN_LINES,
  USER_MESSAGE_FOLD_MIN_CHARS,
  USER_MESSAGE_FOLD_MIN_LINES,
} from "./longText";

describe("longText", () => {
  it("counts lines with mixed newlines", () => {
    expect(countTextLines("a\nb\r\nc\rd")).toBe(4);
    expect(countTextLines("")).toBe(0);
  });

  it("folds when over line or char thresholds", () => {
    const short = "hello";
    expect(shouldFoldUserMessage(short)).toBe(false);

    const manyLines = Array.from(
      { length: USER_MESSAGE_FOLD_MIN_LINES + 1 },
      (_, i) => `L${i}`,
    ).join("\n");
    expect(shouldFoldUserMessage(manyLines)).toBe(true);

    const longChars = "x".repeat(USER_MESSAGE_FOLD_MIN_CHARS + 1);
    expect(shouldFoldUserMessage(longChars)).toBe(true);
  });

  it("folds mid-turn injects on a shorter threshold", () => {
    expect(shouldFoldSoftInject("short")).toBe(false);
    const manyLines = Array.from(
      { length: SOFT_INJECT_FOLD_MIN_LINES + 1 },
      (_, i) => `L${i}`,
    ).join("\n");
    expect(shouldFoldSoftInject(manyLines)).toBe(true);
    expect(shouldFoldSoftInject("x".repeat(SOFT_INJECT_FOLD_MIN_CHARS + 1))).toBe(true);
  });

  it("attaches paste when over line or char thresholds", () => {
    expect(shouldAttachPasteAsFile("short")).toBe(false);

    const manyLines = Array.from({ length: PASTE_AS_FILE_MIN_LINES }, (_, i) => `L${i}`).join("\n");
    expect(shouldAttachPasteAsFile(manyLines)).toBe(true);

    const longChars = "y".repeat(PASTE_AS_FILE_MIN_CHARS);
    expect(shouldAttachPasteAsFile(longChars)).toBe(true);
  });

  it("builds a stable pasted filename", () => {
    const name = pastedTextFilename("a\nb\nc", new Date("2026-09-09T10:11:12.000Z"));
    expect(name).toBe("pasted-3lines-2026-09-09T10-11-12.txt");
  });
});
