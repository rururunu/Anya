import { describe, expect, it } from "vitest";
import { resolveThemePreview, validateThemeTokens } from "./editor";

describe("theme editor values", () => {
  it("accepts CSS expressions and leaves unset overrides absent", () => {
    expect(
      validateThemeTokens({
        "--peek-user-bubble-bg": "color-mix(in srgb, red 30%, blue)",
        "--peek-text": "",
      }),
    ).toEqual({ "--peek-user-bubble-bg": "color-mix(in srgb, red 30%, blue)" });
  });
  it("rejects stylesheet injection and non-string or non-object values", () => {
    for (const value of [
      [],
      null,
      { bad: "red" },
      { "--peek-text": 1 },
      { "--peek-text": "red;}body{display:none" },
    ])
      expect(() => validateThemeTokens(value)).toThrow();
  });
  it("isolates default region styles while retaining explicit overrides", () => {
    const preview = resolveThemePreview({
      id: "dark",
      name: "Dark",
      mode: "dark",
      tokens: { "--peek-code-body-bg": "#abcdef", "--peek-assistant-radius": "30px" },
      updatedAt: 1,
    });
    expect(preview["--peek-code-fg"]).toBe("#e6e6e6");
    expect(preview["--peek-code-body-bg"]).toBe("#abcdef");
    expect(preview["--peek-assistant-radius"]).toBe("30px");
    expect(preview["--peek-header-bg"]).toBe("var(--peek-list-bg)");
  });
});
