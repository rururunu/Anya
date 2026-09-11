import { describe, expect, it } from "vitest";
import {
  activeFilePathMention,
  activeHashMention,
  filterHashMentionItems,
  formatHashMention,
  isHashableAgentPlugin,
  parseHashMentions,
  type HashMentionItem,
} from "./hashMentions";

describe("hashMentions", () => {
  it("formats and parses skill/mcp/plugin tokens", () => {
    expect(formatHashMention("skill", "generate_word")).toBe("#skill:generate_word");
    expect(parseHashMentions("use #skill:docx and #mcp:gmail and #plugin:computer-use")).toEqual([
      { kind: "skill", id: "docx" },
      { kind: "mcp", id: "gmail" },
      { kind: "plugin", id: "computer-use" },
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
    expect(activeHashMention("#plugin:computer-use", 20)).toBeNull();
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
      { kind: "plugin", id: "computer-use", title: "Computer use" },
    ];
    expect(filterHashMentionItems(items, "skill:").map((item) => item.id)).toEqual(["docx"]);
    expect(filterHashMentionItems(items, "mcp").map((item) => item.id)).toEqual(["gmail"]);
    expect(filterHashMentionItems(items, "plugin:").map((item) => item.id)).toEqual([
      "computer-use",
    ]);
  });

  it("ranks frequently used items first when query is empty", () => {
    const items: HashMentionItem[] = [
      { kind: "skill", id: "pandoc", title: "Pandoc" },
      { kind: "skill", id: "docx", title: "Docx" },
      { kind: "plugin", id: "computer-use", title: "Computer use" },
      { kind: "mcp", id: "gmail", title: "Gmail" },
    ];
    const usage = {
      skill: { docx: { count: 8, lastUsedAt: 1000 } },
      mcp: { gmail: { count: 30, lastUsedAt: 1000 } },
      plugin: { "computer-use": { count: 40, lastUsedAt: 1000 } },
    };
    // Skills stay above plugins, plugins above MCP, even if MCP/plugin is used more often.
    expect(filterHashMentionItems(items, "", usage, 2000).map((item) => item.id)).toEqual([
      "docx",
      "pandoc",
      "computer-use",
      "gmail",
    ]);
  });

  it("lists only enabled agent-tool plugins in the # catalog", () => {
    expect(
      isHashableAgentPlugin({
        enabled: true,
        role: "agent",
        contributes: { agent: { tools: true } },
      }),
    ).toBe(true);
    expect(isHashableAgentPlugin({ enabled: true, contributes: { agent: { tools: true } } })).toBe(
      true,
    );
    expect(isHashableAgentPlugin({ enabled: false, role: "agent" })).toBe(false);
    expect(isHashableAgentPlugin({ enabled: true, role: "ui" })).toBe(false);
  });
});
