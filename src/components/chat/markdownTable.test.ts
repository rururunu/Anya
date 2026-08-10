// @vitest-environment node
import { describe, expect, it } from "vitest";
import { JSDOM } from "jsdom";
import createDOMPurify from "dompurify";
import { marked } from "marked";
import markedKatex from "marked-katex-extension";

const TABLE_MD = [
  "| 版本 | 发布时间 | 类型 | 核心变化 |",
  "|---|---|---|---|",
  "| Java 8 | 2014-03 | LTS | Lambda、Stream API、Optional、接口默认方法、java.time、CompletableFuture |",
  "| Java 9 | 2017-09 | 常规 | 模块系统 JPMS、JShell、集合工厂方法 List.of() |",
  "| Java 11 | 2018-09 | LTS | HttpClient、String 新方法（isBlank()/repeat()）、java Hello.java 直接运行 |",
  "| Java 17 | 2021-09 | LTS | 密封类转正、强封装 JDK 内部 API、伪随机数生成器 |",
  "| Java 21 | 2023-09 | LTS | 虚拟线程、Record 模式 + 模式匹配 switch 转正、Sequenced Collections、分代 ZGC |",
  "",
].join("\n");

/** Mirrors Markdown.vue's exact pipeline (katex extension + custom renderer
 * + DOMPurify sanitize) so this test verifies what the chat panel renders. */
function renderMarkdown(content: string, DOMPurify: ReturnType<typeof createDOMPurify>): string {
  const renderer = new marked.Renderer();
  renderer.code = ({ text, lang }) =>
    `<div class="code-block"><pre><code class="lang-${lang ?? ""}">${text}</code></pre></div>`;
  marked.use(markedKatex({ nonStandard: true, throwOnError: false }));
  marked.setOptions({ breaks: true, gfm: true, renderer });
  const raw = marked.parse(content, { async: false }) as string;
  return DOMPurify.sanitize(raw, {
    ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto|tel|file|sms):|[^&#]*?:|data:image\/)/i,
    ADD_TAGS: ["table", "thead", "tbody", "tfoot", "tr", "th", "td", "caption", "colgroup", "col"],
  });
}

describe("markdown table rendering (jsdom = browser behavior)", () => {
  const dom = new JSDOM("<!doctype html><html><body></body></html>");
  const DOMPurify = createDOMPurify(dom.window);

  it("DOMPurify keeps a plain table element", () => {
    const out = DOMPurify.sanitize("<table><tbody><tr><td>x</td></tr></tbody></table>");
    expect(out).toContain("<table>");
    expect(out).toContain("<tbody>");
  });

  it("full Markdown.vue pipeline renders the Java table", () => {
    const clean = renderMarkdown(TABLE_MD, DOMPurify);
    expect(clean).toContain("<table>");
    expect(clean).toContain("<th>版本</th>");
    expect(clean).toContain("<td>Java 8</td>");
    expect(clean).toContain("<td>Java 21</td>");
  });

  it("Node.prototype.nodeName getter is intact (DOMPurify tag detection)", () => {
    const table = dom.window.document.createElement("table");
    const getter = Object.getOwnPropertyDescriptor(dom.window.Node.prototype, "nodeName")!.get!;
    expect(getter.call(table)).toBe("TABLE");
  });

  it("table structure survives DOMPurify: thead rows and cell text", () => {
    const clean = renderMarkdown(TABLE_MD, DOMPurify);
    const doc = new JSDOM(clean).window.document;
    const table = doc.querySelector("table");
    expect(table).not.toBeNull();
    expect(table!.querySelectorAll("th").length).toBe(4);
    expect(table!.querySelectorAll("tbody tr").length).toBe(5);
    const cells = Array.from(table!.querySelectorAll("td")).map((td) => td.textContent);
    expect(cells[0]).toContain("Java 8");
    expect(cells.at(-1)).toContain("分代 ZGC");
  });
});
