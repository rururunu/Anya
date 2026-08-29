// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import {
  buildMermaidPlaceholder,
  isMermaidDiagramSource,
  readMermaidSource,
  shouldRenderMermaidBlock,
} from "@/services/chat/mermaidDiagram";

describe("mermaidDiagram", () => {
  it("detects mermaid language aliases", () => {
    expect(shouldRenderMermaidBlock("mermaid", "flowchart LR\nA-->B")).toBe(true);
    expect(shouldRenderMermaidBlock("flowchart", "flowchart LR\nA-->B")).toBe(true);
  });

  it("detects bare flowchart blocks without a language tag", () => {
    expect(shouldRenderMermaidBlock("", "flowchart TB\n  A --> B")).toBe(true);
    expect(isMermaidDiagramSource("graph LR\n  A --> B")).toBe(true);
  });

  it("builds a hidden pre placeholder with multiline source", () => {
    const html = buildMermaidPlaceholder("flowchart TB\n  A --> B");
    expect(html).toContain('class="mermaid-block"');
    expect(html).toContain('<pre class="mermaid-source" hidden');
    expect(html).toContain("flowchart TB");
    expect(html).toContain("A --&gt; B");
    expect(html).not.toContain("data-mermaid-source=");
  });

  it("reads source back from the placeholder node", () => {
    const html = buildMermaidPlaceholder("flowchart LR\n  A --> B");
    document.body.innerHTML = html;
    const node = document.body.querySelector(".mermaid-block") as HTMLElement;
    expect(readMermaidSource(node)).toBe("flowchart LR\n  A --> B");
    document.body.innerHTML = "";
  });
});
