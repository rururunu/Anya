import { describe, expect, it } from "vitest";
import { hasAnsi, parseAnsi, stripAnsi } from "./ansi";

const ESC = "\x1b";

describe("parseAnsi", () => {
  it("returns plain text as a single span", () => {
    expect(parseAnsi("hello world")).toEqual([{ text: "hello world" }]);
  });

  it("parses basic foreground colors", () => {
    const spans = parseAnsi(`${ESC}[31mred${ESC}[0m plain`);
    expect(spans).toHaveLength(2);
    expect(spans[0]).toMatchObject({ text: "red", color: "#ff7b72" });
    expect(spans[1]).toEqual({ text: " plain" });
  });

  it("parses bright colors and bold", () => {
    const spans = parseAnsi(`${ESC}[1;92mok${ESC}[0m`);
    expect(spans[0]).toMatchObject({ text: "ok", bold: true, color: "#56d364" });
  });

  it("parses 256-color sequences", () => {
    const spans = parseAnsi(`${ESC}[38;5;196mX${ESC}[0m`);
    expect(spans[0]!.color).toBe("rgb(255,0,0)");
  });

  it("parses truecolor sequences", () => {
    const spans = parseAnsi(`${ESC}[38;2;18;52;86mX${ESC}[0m`);
    expect(spans[0]!.color).toBe("rgb(18,52,86)");
  });

  it("parses background colors", () => {
    const spans = parseAnsi(`${ESC}[41mwarn${ESC}[49m`);
    expect(spans[0]).toMatchObject({ text: "warn", background: "#ff7b72" });
  });

  it("handles partial resets", () => {
    const spans = parseAnsi(`${ESC}[1;4mstrong${ESC}[22mstill-underline${ESC}[24mplain`);
    expect(spans[0]).toMatchObject({ bold: true, underline: true });
    expect(spans[1]).toMatchObject({ text: "still-underline", underline: true });
    expect(spans[1]!.bold).toBeUndefined();
    expect(spans[2]).toEqual({ text: "plain" });
  });

  it("strips cursor movement and erase sequences", () => {
    expect(stripAnsi(`a${ESC}[2K${ESC}[1Ab`)).toBe("ab");
  });

  it("strips OSC title sequences", () => {
    expect(stripAnsi(`${ESC}]0;window title\x07visible`)).toBe("visible");
  });

  it("collapses carriage-return progress overwrites", () => {
    expect(stripAnsi("progress 10%\rprogress 50%\rprogress 100%\ndone")).toBe(
      "progress 100%\ndone",
    );
  });

  it("normalizes CRLF line endings", () => {
    expect(stripAnsi("line1\r\nline2")).toBe("line1\nline2");
  });

  it("merges adjacent spans with identical styles", () => {
    const spans = parseAnsi(`${ESC}[31ma${ESC}[31mb${ESC}[0m`);
    expect(spans).toEqual([{ text: "ab", color: "#ff7b72" }]);
  });
});

describe("hasAnsi", () => {
  it("detects escape sequences", () => {
    expect(hasAnsi(`${ESC}[31mred`)).toBe(true);
    expect(hasAnsi("plain text")).toBe(false);
  });
});
