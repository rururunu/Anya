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
  it("uses PingFang SC first for default Chinese UI", () => {
    expect(fontStack("cjk")).toMatch(/^"PingFang SC"/);
  });

  it("uses Inter Variable first for default English UI", () => {
    expect(fontStack("latin")).toMatch(/^"Inter Variable"/);
  });

  it("puts Fira Code first for default code", () => {
    expect(fontStack("mono")).toMatch(/^"Fira Code"/);
  });

  it("prepends a custom family and keeps fallbacks", () => {
    const stack = fontStack("cjk", "LXGW WenKai");
    expect(stack.startsWith('"LXGW WenKai"')).toBe(true);
    expect(stack).toContain("PingFang SC");
  });
});

describe("uiSansStack", () => {
  it("puts Latin faces before PingFang and the generic fallback after it", () => {
    const stack = uiSansStack({});
    expect(stack.indexOf("Inter Variable")).toBeLessThan(stack.indexOf("PingFang SC"));
    expect(stack.indexOf("PingFang SC")).toBeLessThan(stack.indexOf("sans-serif"));
  });

  it("puts a chosen English font ahead of PingFang", () => {
    const stack = uiSansStack({ fontLatin: "Calibri" });
    expect(stack.indexOf("Calibri")).toBeLessThan(stack.indexOf("PingFang SC"));
  });
});

describe("fontSelectValue", () => {
  it("maps empty storage to default", () => {
    expect(fontSelectValue("", "latin")).toBe(FONT_DEFAULT);
  });

  it("maps unknown families to custom", () => {
    expect(fontSelectValue("LXGW WenKai", "cjk")).toBe(FONT_CUSTOM);
  });

  it("uses installed font names from the system list", () => {
    expect(fontSelectValue("noto sans sc", "cjk", ["Noto Sans SC", "LXGW WenKai"])).toBe(
      "Noto Sans SC",
    );
    expect(fontSelectValue("LXGW WenKai", "cjk", ["Noto Sans SC"])).toBe(FONT_CUSTOM);
  });
});
