import { describe, expect, it } from "vitest";
import {
  escapeInlineHtml,
  renderInlineTokenHighlightHtml,
  splitInlineTokenParts,
} from "./inlineTokenMarks";

describe("inlineTokenMarks", () => {
  it("escapes HTML in plain text", () => {
    expect(escapeInlineHtml(`a <b> & "c"`)).toBe("a &lt;b&gt; &amp; &quot;c&quot;");
  });

  it("splits file and hash tokens", () => {
    const parts = splitInlineTokenParts('see @src/a.ts and #skill:docx #mcp:gmail @"my file.ts"');
    expect(parts.map((part) => part.kind)).toEqual([
      "text",
      "mention",
      "text",
      "skill",
      "text",
      "mcp",
      "text",
      "mention",
    ]);
  });

  it("keeps highlight HTML length-aligned with source tokens", () => {
    const text = "use @docs/a.pdf and #skill:docx please";
    const html = renderInlineTokenHighlightHtml(text);
    expect(html).toContain('class="inline-token inline-token-file"');
    expect(html).toContain('class="inline-token inline-token-skill"');
    expect(html).toContain("@docs/a.pdf");
    expect(html).toContain("#skill:docx");
    // No injected logos / labels that would desync the caret.
    expect(html).not.toContain("<img");
  });
});
