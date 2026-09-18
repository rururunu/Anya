import { describe, expect, it } from "vitest";
import {
  FONT_CUSTOM,
  FONT_DEFAULT,
  fontSelectValue,
  fontStack,
  quoteFontFamily,
  sanitizeFontName,
  uiSansStack,
} from "./fonts";

describe("sanitizeFontName", () => {
  it("strips CSS-breaking characters", () => {
    expect(sanitizeFontName("Comic Sans MS; } body { color: red")).toBe(
      "Comic Sans MS  body  color: red",
    );
  });
});

describe("quoteFontFamily", () => {
  it("leaves generic families unquoted", () => {
    expect(quoteFontFamily("system-ui")).toBe("system-ui");
    expect(quoteFontFamily("sans-serif")).toBe("sans-serif");
  });

  it("quotes names with spaces", () => {
    expect(quoteFontFamily("Microsoft YaHei UI")).toBe('"Microsoft YaHei UI"');
  });
});

describe("fontStack", () => {
  it("uses Microsoft YaHei first for default Chinese UI", () => {
    expect(fontStack("cjk")).toMatch(/^"Microsoft YaHei"/);
  });

  it("uses Source Sans 3 first for default English UI", () => {
    expect(fontStack("latin")).toMatch(/^"Source Sans 3 Variable"/);
  });

  it("puts Cascadia Code first for default code", () => {
    expect(fontStack("mono")).toMatch(/^"Cascadia Code"/);
  });

  it("prepends a custom family and keeps fallbacks", () => {
    const stack = fontStack("cjk", "LXGW WenKai");
    expect(stack.startsWith('"LXGW WenKai"')).toBe(true);
    expect(stack).toContain("Microsoft YaHei");
  });
});

describe("uiSansStack", () => {
  it("puts Source Sans before YaHei so Latin is not YaHei", () => {
    const stack = uiSansStack({});
    expect(stack.indexOf("Source Sans 3 Variable")).toBeLessThan(stack.indexOf("Microsoft YaHei"));
  });

  it("puts a chosen English font ahead of YaHei", () => {
    const stack = uiSansStack({ fontLatin: "Calibri" });
    expect(stack.indexOf("Calibri")).toBeLessThan(stack.indexOf("Microsoft YaHei"));
  });
});

describe("fontSelectValue", () => {
  it("maps empty storage to default", () => {
    expect(fontSelectValue("", "latin")).toBe(FONT_DEFAULT);
  });

  it("maps unknown families to custom", () => {
    expect(fontSelectValue("LXGW WenKai", "cjk")).toBe(FONT_CUSTOM);
  });
});
