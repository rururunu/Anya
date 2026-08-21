import { describe, expect, it } from "vitest";
import { formatSessionPreview } from "./sessionPreview";

describe("formatSessionPreview", () => {
  it("prefers prose after leading @/# chips", () => {
    expect(
      formatSessionPreview(
        "@2026年喀什市.docx 根据这份招标文件 和 @参考文件.docx 的信息完成标书",
        48,
      ),
    ).toBe("根据这份招标文件 和 @参考文件.docx 的信息完成标书");
  });

  it("shortens #skill/#mcp tokens", () => {
    expect(formatSessionPreview("#skill:generate_word 写技术方案")).toBe("写技术方案");
    expect(formatSessionPreview("#mcp:filesystem 列出目录")).toBe("列出目录");
  });

  it("falls back to prettified chips when there is no prose", () => {
    expect(formatSessionPreview("@docs/a.docx #skill:generate_word")).toBe("@a.docx generate_word");
    expect(formatSessionPreview("#mcp:sm-gmail")).toBe("gmail");
  });
});
