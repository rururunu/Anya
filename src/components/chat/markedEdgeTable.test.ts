// @vitest-environment node
import { describe, expect, it } from "vitest";
import { marked } from "marked";
import markedKatex from "marked-katex-extension";

describe("marked 18 table parsing edge cases", () => {
  const renderer = new marked.Renderer();
  marked.use(markedKatex({ nonStandard: true, throwOnError: false }));
  marked.setOptions({ breaks: true, gfm: true, renderer });

  it("table directly after paragraph without blank line", () => {
    const md = [
      "以下是 Java 版本历史：",
      "| 版本 | 类型 |",
      "|---|---|",
      "| Java 8 | LTS |",
      "",
    ].join("\n");
    const html = marked.parse(md, { async: false }) as string;
    console.log("NO-BLANK HTML:", html.replace(/\n/g, "\\n"));
    expect(html).toContain("<table>");
  });

  it("table after paragraph with blank line", () => {
    const md = [
      "以下是 Java 版本历史：",
      "",
      "| 版本 | 类型 |",
      "|---|---|",
      "| Java 8 | LTS |",
      "",
    ].join("\n");
    const html = marked.parse(md, { async: false }) as string;
    console.log("BLANK HTML:", html.replace(/\n/g, "\\n"));
    expect(html).toContain("<table>");
  });

  it("table inside list item", () => {
    const md = ["- item", "", "  | A | B |", "  |---|---|", "  | 1 | 2 |", ""].join("\n");
    const html = marked.parse(md, { async: false }) as string;
    console.log("LIST HTML:", html.replace(/\n/g, "\\n"));
    expect(html).toContain("<table>");
  });

  it("table separated by single newline from code fence", () => {
    const md = ["```", "x", "```", "| A | B |", "|---|---|", "| 1 | 2 |", ""].join("\n");
    const html = marked.parse(md, { async: false }) as string;
    console.log("AFTER-FENCE HTML:", html.replace(/\n/g, "\\n"));
    expect(html).toContain("<table>");
  });
});
