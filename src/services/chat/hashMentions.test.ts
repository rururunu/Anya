import { describe, expect, it } from "vitest";
import {
  activeFilePathMention,
  activeHashMention,
  filterHashMentionItems,
  formatHashMention,
  parseHashMentions,
  type HashMentionItem,
} from "./hashMentions";

describe("hashMentions", () => {
  it("formats and parses skill/mcp tokens", () => {
    expect(formatHashMention("skill", "generate_bid_tech")).toBe("#skill:generate_bid_tech");
    expect(parseHashMentions("use #skill:docx and #mcp:gmail")).toEqual([
      { kind: "skill", id: "docx" },
      { kind: "mcp", id: "gmail" },
    ]);
  });

  it("detects # at the start, middle, and end relative to caret", () => {
    expect(activeHashMention("#", 1)).toEqual({ query: "", start: 0, end: 1 });
    expect(activeHashMention("#sk", 3)).toEqual({ query: "sk", start: 0, end: 3 });

    const mid = "根据这份 #sk 继续";
    const hashAt = mid.indexOf("#");
    expect(activeHashMention(mid, hashAt + 3)).toEqual({
      query: "sk",
      start: hashAt,
      end: hashAt + 3,
    });

    // Inserting `#` in front of existing text must not swallow the following prose.
    expect(activeHashMention("#根据这份", 1)).toEqual({ query: "", start: 0, end: 1 });

    // Mid-word hash should not open the picker.
    expect(activeHashMention("foo#bar", 7)).toBeNull();

    // Completed resource tokens should not keep the picker open.
    expect(activeHashMention("#skill:docx", 11)).toBeNull();
    expect(activeHashMention("#mcp:gmail", 10)).toBeNull();
    expect(activeHashMention("use #skill:docx", 15)).toBeNull();
  });

  it("detects @ file mentions with caret awareness", () => {
    expect(activeFilePathMention("@re", 3)).toEqual({ query: "re", start: 0, end: 3 });
    const text = "请看 @docs/a.docx 然后";
    const at = text.indexOf("@");
    expect(activeFilePathMention(text, at + 5)).toEqual({
      query: "docs",
      start: at,
      end: at + 5,
    });
  });

  it("filters catalog by kind prefixes", () => {
    const items: HashMentionItem[] = [
      { kind: "skill", id: "docx", title: "Docx" },
      { kind: "mcp", id: "gmail", title: "Gmail" },
    ];
    expect(filterHashMentionItems(items, "skill:").map((item) => item.id)).toEqual(["docx"]);
    expect(filterHashMentionItems(items, "mcp").map((item) => item.id)).toEqual(["gmail"]);
  });

  it("ranks frequently used items first when query is empty", () => {
    const items: HashMentionItem[] = [
      { kind: "skill", id: "pandoc", title: "Pandoc" },
      { kind: "skill", id: "docx", title: "Docx" },
      { kind: "mcp", id: "gmail", title: "Gmail" },
    ];
    const usage = {
      skill: { docx: { count: 8, lastUsedAt: 1000 } },
      mcp: { gmail: { count: 30, lastUsedAt: 1000 } },
    };
    // Skills stay above MCP even if an MCP is used more often.
    expect(filterHashMentionItems(items, "", usage, 2000).map((item) => item.id)).toEqual([
      "docx",
      "pandoc",
      "gmail",
    ]);
  });
});
