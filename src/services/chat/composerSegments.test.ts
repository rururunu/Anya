import { describe, expect, it } from "vitest";
import {
  appendComposerSegment,
  flushLiveMessageToSegments,
  formatMentionPath,
  isEditableTextSegment,
  joinInlineParts,
  mentionDisplayLabel,
  pasteLineCount,
  parseComposerTextToSegments,
  serializeComposerSegments,
  type ComposerSegment,
} from "./composerSegments";

describe("composerSegments", () => {
  it("counts paste lines", () => {
    expect(pasteLineCount("")).toBe(0);
    expect(pasteLineCount("a\nb\nc")).toBe(3);
  });

  it("quotes mention paths that contain spaces", () => {
    expect(formatMentionPath("src/a.ts")).toBe("@src/a.ts");
    expect(formatMentionPath("my file.ts")).toBe('@"my file.ts"');
  });

  it("serializes directory mentions with a trailing slash", () => {
    expect(formatMentionPath("src/main", true)).toBe("@src/main/");
    expect(formatMentionPath("src/main/")).toBe("@src/main/");
  });

  it("shows full path for folders and disambiguates duplicate file names", () => {
    expect(mentionDisplayLabel("src/main/", { isDir: true })).toBe("src/main");
    expect(mentionDisplayLabel("src/a.ts", { catalog: ["src/a.ts"] })).toBe("a.ts");
    expect(mentionDisplayLabel("src/a.ts", { catalog: ["src/a.ts", "lib/a.ts"] })).toBe("src/a.ts");
  });

  it("serializes segments ahead of live text without forced blank lines", () => {
    const segments: ComposerSegment[] = [
      { kind: "text", text: "see " },
      { kind: "mention", path: "src/a.ts" },
    ];
    expect(serializeComposerSegments(segments, " please")).toBe("see @src/a.ts please");
  });

  it("merges adjacent text and paste chips", () => {
    const segments: ComposerSegment[] = [];
    appendComposerSegment(segments, { kind: "text", text: "hello" });
    appendComposerSegment(segments, { kind: "text", text: " world" });
    appendComposerSegment(segments, { kind: "paste", text: "p1" });
    appendComposerSegment(segments, { kind: "paste", text: "p2" });
    expect(segments).toEqual([
      { kind: "text", text: "hello world" },
      { kind: "paste", text: "p1\np2" },
    ]);
  });

  it("flushes live message into a trailing text segment", () => {
    const segments: ComposerSegment[] = [{ kind: "mention", path: "a.ts" }];
    const next = flushLiveMessageToSegments(segments, " note");
    expect(next.liveMessage).toBe("");
    expect(next.segments).toEqual([
      { kind: "mention", path: "a.ts" },
      { kind: "text", text: " note" },
    ]);
  });

  it("joins inline parts with a single separating space when needed", () => {
    expect(joinInlineParts(["a", "b"])).toBe("a b");
    expect(joinInlineParts(["a ", "b"])).toBe("a b");
  });

  it("serializes directory segment with trailing slash", () => {
    const segments: ComposerSegment[] = [{ kind: "mention", path: "src/main", isDir: true }];
    expect(serializeComposerSegments(segments, "")).toBe("@src/main/");
  });

  it("restores chips from serialized rewind text", () => {
    const parsed = parseComposerTextToSegments('#skill:docx @src/a.ts @"my file.ts" please review');
    expect(parsed.segments).toEqual([
      { kind: "skill", id: "docx" },
      { kind: "mention", path: "src/a.ts" },
      { kind: "mention", path: "my file.ts" },
    ]);
    expect(parsed.liveMessage).toBe("please review");
  });

  it("keeps plain text when there are no chips", () => {
    const parsed = parseComposerTextToSegments("just a draft");
    expect(parsed.segments).toEqual([]);
    expect(parsed.liveMessage).toBe("just a draft");
  });

  it("identifies editable text segments", () => {
    expect(isEditableTextSegment({ kind: "text", text: "hi" })).toBe(true);
    expect(isEditableTextSegment({ kind: "paste", text: "a\nb" })).toBe(true);
    expect(isEditableTextSegment({ kind: "paste", text: "a\nb\nc\nd\ne\nf" })).toBe(false);
    expect(isEditableTextSegment({ kind: "mention", path: "a.ts" })).toBe(false);
  });
});
