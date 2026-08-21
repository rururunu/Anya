import { describe, expect, it } from "vitest";
import { JSDOM } from "jsdom";
import createDOMPurify from "dompurify";
import { marked } from "marked";
import markedKatex from "marked-katex-extension";

import { normalizeMarkdownInput } from "@/services/chat/markdownNormalize";

function renderLinkHtml(content: string): string {
  const renderer = new marked.Renderer();
  marked.use(markedKatex({ nonStandard: true, throwOnError: false }));
  marked.setOptions({ breaks: true, gfm: true, renderer });
  const raw = marked.parse(normalizeMarkdownInput(content), { async: false }) as string;
  const dom = new JSDOM("<!doctype html><html><body></body></html>");
  const DOMPurify = createDOMPurify(dom.window);
  return DOMPurify.sanitize(raw, {
    ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto|tel|file|sms):|[^&#]*?:|data:image\/)/i,
  });
}

describe("markdownNormalize", () => {
  it("collapses a vertical streamed URL into one autolink", () => {
    const vertical = [
      "[DeepSeek 官方 API 文档]",
      "h",
      "t",
      "t",
      "p",
      "s",
      ":",
      "/",
      "/",
      "p",
      "l",
      "a",
      "t",
      "f",
      "o",
      "r",
      "m",
      ".",
      "d",
      "e",
      "e",
      "p",
      "s",
      "e",
      "e",
      "k",
      ".",
      "c",
      "o",
      "m",
      "/",
      "a",
      "p",
      "i",
      "−",
      "d",
      "o",
      "c",
      "s",
      "/",
      "z",
      "h",
      "−",
      "c",
      "n",
      "/",
      "q",
      "u",
      "i",
      "c",
      "k",
      "s",
      "t",
      "a",
      "r",
      "t",
      "/",
      "p",
      "r",
      "i",
      "c",
      "i",
      "n",
      "g",
    ].join("\n");

    const html = renderLinkHtml(vertical);
    expect(html).toContain(
      '<a href="https://platform.deepseek.com/api-docs/zh-cn/quickstart/pricing"',
    );
    expect(html).not.toContain("<br>h<br>");
  });

  it("merges a full URL line with short tail fragments", () => {
    const md = [
      "https://platform.deepseek.com/api-docs/zh-cn/quick",
      "s",
      "t",
      "a",
      "r",
      "t",
      "/",
      "p",
      "r",
      "i",
      "c",
      "i",
      "n",
      "g",
    ].join("\n");

    const html = renderLinkHtml(md);
    expect(html).toContain(
      '<a href="https://platform.deepseek.com/api-docs/zh-cn/quickstart/pricing"',
    );
  });

  it("leaves fenced code untouched", () => {
    const md = ["```", "h", "t", "t", "p", "s", "://x", "```"].join("\n");
    expect(normalizeMarkdownInput(md)).toBe(md);
  });
});
