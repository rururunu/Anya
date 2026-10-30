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
  it("supports custom fonts, footer chip colors, radii, and shadows in theme tokens", () => {
    const tokens = validateThemeTokens({
      "--peek-font-sans": '"LXGW WenKai", "PingFang SC", sans-serif',
      "--peek-font-mono": '"JetBrains Mono", monospace',
      "--peek-sidebar-text": "#112233",
      "--peek-composer-chip-text": "#445566",
      "--peek-composer-chip-radius": "999px",
      "--peek-composer-chip-shadow": "0 2px 6px rgba(0, 0, 0, 0.12)",
      "--peek-composer-full-access-text": "#ff3b30",
      "--peek-composer-meta-text": "#778899",
      "--peek-card-radius": "16px",
      "--peek-card-shadow": "0 8px 24px rgba(0, 0, 0, 0.16)",
    });
    expect(tokens["--peek-font-sans"]).toBe('"LXGW WenKai", "PingFang SC", sans-serif');
    expect(tokens["--peek-composer-chip-radius"]).toBe("999px");
    expect(tokens["--peek-composer-full-access-text"]).toBe("#ff3b30");
  });
});
