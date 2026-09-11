/** @vitest-environment jsdom */
import { beforeEach, describe, expect, it, vi } from "vitest";

import { COLOR_SCHEME_CACHE_KEY } from "@/services/theme/catalog";

import { applyThemeAppearance, readThemeState, THEME_CHANGE_EVENT } from "./ThemeService";

vi.mock("@/services/overlay/appearance", () => ({
  applyChromeFrostedGlass: vi.fn().mockResolvedValue(undefined),
}));

describe("ThemeService", () => {
  beforeEach(() => {
    document.documentElement.lang = "zh-CN";
    document.documentElement.dataset.theme = "light";
    document.documentElement.classList.remove("dark", "chrome-frosted-glass");
    document.documentElement.style.removeProperty("color-scheme");
    document.documentElement.style.removeProperty("background");
    document.body.style.removeProperty("color-scheme");
    document.body.style.removeProperty("background");
    localStorage.clear();
    vi.useRealTimers();
  });

  it("applies dark theme to html[data-theme] with only-light color-scheme", () => {
    applyThemeAppearance({
      colorScheme: "dark",
      language: "zh-CN",
      chromeFrostedGlass: false,
    });

    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
    expect(document.documentElement.style.colorScheme).toBe("only light");
    expect(readThemeState().isDark).toBe(true);
  });

  it("caches color scheme for boot splash", () => {
    applyThemeAppearance({
      colorScheme: "dark",
      language: "en-US",
      chromeFrostedGlass: false,
    });

    expect(localStorage.getItem(COLOR_SCHEME_CACHE_KEY)).toBe("dark");
    expect(document.documentElement.lang).toBe("en-US");
  });

  it("dispatches peek-theme-change", () => {
    const handler = vi.fn();
    window.addEventListener(THEME_CHANGE_EVENT, handler);

    applyThemeAppearance({
      colorScheme: "light",
      language: "zh-CN",
      chromeFrostedGlass: false,
    });

    expect(handler).toHaveBeenCalledTimes(1);
    const event = handler.mock.calls[0][0] as CustomEvent;
    expect(event.detail.themeId).toBe("light");
    expect(event.detail.isDark).toBe(false);

    window.removeEventListener(THEME_CHANGE_EVENT, handler);
  });

  it("keeps only-light color-scheme on body when frosted glass is enabled", () => {
    applyThemeAppearance({
      colorScheme: "dark",
      language: "zh-CN",
      chromeFrostedGlass: true,
    });

    expect(document.documentElement.style.colorScheme).toBe("normal");
    expect(document.body.style.colorScheme).toBe("only light");
  });

  it("keeps only-light color-scheme after rapid light-dark-light switches", async () => {
    vi.useFakeTimers();

    const schemes = ["dark", "light", "dark", "light", "dark"] as const;
    for (const scheme of schemes) {
      applyThemeAppearance({
        colorScheme: scheme,
        language: "zh-CN",
        chromeFrostedGlass: false,
      });
      await vi.runAllTimersAsync();
    }

    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.style.colorScheme).toBe("only light");
    expect(document.body.style.colorScheme).toBe("");
  });

  it("rebinds color-scheme after double rAF on hot switch", async () => {
    vi.useFakeTimers();
    const rafCallbacks: FrameRequestCallback[] = [];
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((cb) => {
      rafCallbacks.push(cb);
      return rafCallbacks.length;
    });

    applyThemeAppearance({
      colorScheme: "dark",
      language: "zh-CN",
      chromeFrostedGlass: false,
    });

    expect(document.documentElement.style.colorScheme).toBe("only light");

    document.documentElement.style.colorScheme = "dark";
    expect(document.documentElement.style.colorScheme).toBe("dark");

    rafCallbacks[0]?.(0);
    rafCallbacks[1]?.(0);

    expect(document.documentElement.style.colorScheme).toBe("only light");
  });

  it("generates --theme-bg-* CSS variables when custom theme has background", () => {
    applyThemeAppearance({
      colorScheme: "custom-neon",
      language: "zh-CN",
      chromeFrostedGlass: false,
      customThemes: [
        {
          id: "custom-neon",
          name: "Neon",
          mode: "dark",
          tokens: {
            "--peek-bg": "#0a0a0a",
            "--peek-accent": "#00ffcc",
          },
          background: {
            image: "path:C:/wallpaper.png",
            opacity: 0.25,
            blur: 4,
            fit: "cover",
          },
          updatedAt: 100,
        },
      ],
    });

    const styleEl = document.getElementById("anya-custom-theme-sheet");
    expect(styleEl).toBeTruthy();
    expect(styleEl?.textContent).not.toContain("--theme-bg-image:");
    expect(styleEl?.textContent).not.toContain("asset:");
    expect(styleEl?.textContent).toContain("--theme-bg-opacity: 0.25;");
    expect(styleEl?.textContent).toContain("--theme-bg-blur: 4px;");
    expect(styleEl?.textContent).toContain("--theme-bg-fit: cover;");
    expect(styleEl?.textContent).toContain("--peek-composer-fill:");
    expect(styleEl?.textContent).toContain("--peek-interaction-fill:");
  });

  it("keeps remote wallpaper URLs on --theme-bg-image", () => {
    applyThemeAppearance({
      colorScheme: "custom-neon",
      language: "zh-CN",
      chromeFrostedGlass: false,
      customThemes: [
        {
          id: "custom-neon",
          name: "Neon",
          mode: "dark",
          tokens: { "--peek-bg": "#0a0a0a" },
          background: { image: "https://example.com/bg.jpg", opacity: 0.2 },
          updatedAt: 100,
        },
      ],
    });

    const styleEl = document.getElementById("anya-custom-theme-sheet");
    expect(styleEl?.textContent).toContain('--theme-bg-image: url("https://example.com/bg.jpg")');
  });
});
