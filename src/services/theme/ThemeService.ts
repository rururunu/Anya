import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

import { applyChromeFrostedGlass } from "@/services/overlay/appearance";
import { isLocalImagePath, resolveChatImageSrc } from "@/services/chat/localImageSrc";
import type { AppLanguage, ColorScheme } from "@/types/setting";
import {
  COLOR_SCHEME_CACHE_KEY,
  CUSTOM_THEMES_CACHE_KEY,
  isDarkTheme,
  normalizeColorScheme,
  readCachedColorScheme,
} from "./catalog";

import {
  THEME_CHANGE_EVENT,
  type CustomThemeConfig,
  type ThemeAppearanceInput,
  type ThemeChangeDetail,
  type ThemeState,
} from "./types";

export { THEME_CHANGE_EVENT };
export type { ThemeAppearanceInput, ThemeChangeDetail, ThemeState, ThemeId } from "./types";

const CUSTOM_STYLE_TAG_ID = "anya-custom-theme-sheet";

function rootElement(): HTMLElement {
  return document.documentElement;
}

/** WebView2 inverts paints when color-scheme is dark; appearance is CSS tokens only. */
export function documentColorScheme(_colorScheme: ColorScheme): "only light" {
  return "only light";
}

function cacheColorScheme(colorScheme: ColorScheme) {
  try {
    localStorage.setItem(COLOR_SCHEME_CACHE_KEY, colorScheme);
  } catch {
    // ignore quota / private mode
  }
}

function cacheCustomThemes(themes?: CustomThemeConfig[]) {
  if (!themes) return;
  try {
    localStorage.setItem(CUSTOM_THEMES_CACHE_KEY, JSON.stringify(themes));
  } catch {
    // ignore quota / private mode
  }
}

export function readCachedCustomThemes(): CustomThemeConfig[] {
  try {
    const raw = localStorage.getItem(CUSTOM_THEMES_CACHE_KEY);
    if (raw) {
      return JSON.parse(raw);
    }
  } catch {
    // ignore
  }
  return [];
}

/** Parse hex or rgb color string to { r, g, b } in [0, 255] */
function parseRgbColor(color: string): { r: number; g: number; b: number } | null {
  const trimmed = color.trim();
  if (trimmed.startsWith("#")) {
    const hex = trimmed.slice(1);
    if (hex.length === 3) {
      return {
        r: parseInt(hex[0] + hex[0], 16),
        g: parseInt(hex[1] + hex[1], 16),
        b: parseInt(hex[2] + hex[2], 16),
      };
    }
    if (hex.length >= 6) {
      return {
        r: parseInt(hex.slice(0, 2), 16),
        g: parseInt(hex.slice(2, 4), 16),
        b: parseInt(hex.slice(4, 6), 16),
      };
    }
  }
  const rgbMatch = trimmed.match(/^rgba?\((\d+),\s*(\d+),\s*(\d+)/i);
  if (rgbMatch) {
    return {
      r: Math.max(0, Math.min(255, Number(rgbMatch[1]))),
      g: Math.max(0, Math.min(255, Number(rgbMatch[2]))),
      b: Math.max(0, Math.min(255, Number(rgbMatch[3]))),
    };
  }
  return null;
}

/** Compute WCAG relative luminance: 0 (black) to 1 (white). */
function getRelativeLuminance(r: number, g: number, b: number): number {
  const [rs, gs, bs] = [r / 255, g / 255, b / 255].map((c) =>
    c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4),
  );
  return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs;
}

/** Compute contrast ratio between two colors (1 to 21). */
function getContrastRatio(
  fg: { r: number; g: number; b: number },
  bg: { r: number; g: number; b: number },
): number {
  const l1 = getRelativeLuminance(fg.r, fg.g, fg.b);
  const l2 = getRelativeLuminance(bg.r, bg.g, bg.b);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

/** Mix two rgb colors with a weight for c1 (0..1). */
function mixColors(
  c1: { r: number; g: number; b: number },
  c2: { r: number; g: number; b: number },
  weight: number,
): string {
  const r = Math.round(c1.r * weight + c2.r * (1 - weight));
  const g = Math.round(c1.g * weight + c2.g * (1 - weight));
  const b = Math.round(c1.b * weight + c2.b * (1 - weight));
  return `#${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}`;
}

/** Harmonize custom theme tokens with contrast protection and full-UI coverage. */
export function harmonizeCustomTheme(theme: CustomThemeConfig): Record<string, string> {
  const isDark = theme.mode === "dark";
  const tokens: Record<string, string> = { ...theme.tokens };

  // 1. Resolve base background
  const bgRaw = tokens["--peek-bg"] || (isDark ? "#141414" : "#f8f8f8");
  tokens["--peek-bg"] = bgRaw;
  const bgRgb =
    parseRgbColor(bgRaw) || (isDark ? { r: 20, g: 20, b: 20 } : { r: 248, g: 248, b: 248 });

  // 2. Resolve surfaces & sidebar
  if (!tokens["--peek-sidebar"]) {
    tokens["--peek-sidebar"] = isDark
      ? mixColors(bgRgb, { r: 0, g: 0, b: 0 }, 0.85)
      : mixColors(bgRgb, { r: 235, g: 235, b: 235 }, 0.9);
  }
  if (!tokens["--peek-surface"]) {
    tokens["--peek-surface"] = isDark
      ? mixColors(bgRgb, { r: 255, g: 255, b: 255 }, 0.88)
      : "#ffffff";
  }

  const surfaceRgb = parseRgbColor(tokens["--peek-surface"]) || bgRgb;

  // 3. Resolve list and composer fill (prevents stark white patches in dark mode)
  if (!tokens["--peek-list-bg"]) {
    tokens["--peek-list-bg"] = isDark ? tokens["--peek-surface"] : "#ffffff";
  }
  if (!tokens["--peek-composer-fill"]) {
    tokens["--peek-composer-fill"] = isDark ? tokens["--peek-surface"] : tokens["--peek-list-bg"];
  }
  if (!tokens["--peek-interaction-fill"]) {
    tokens["--peek-interaction-fill"] = tokens["--peek-composer-fill"];
  }
  if (!tokens["--peek-input-bg"]) {
    tokens["--peek-input-bg"] = isDark ? tokens["--peek-surface"] : "#ffffff";
  }
  if (!tokens["--peek-user-bubble-bg"]) {
    tokens["--peek-user-bubble-bg"] = isDark
      ? mixColors(surfaceRgb, { r: 255, g: 255, b: 255 }, 0.85)
      : "#ececec";
  }

  // 4. Contrast Guard for Primary Text (ensure minimum 4.5:1 ratio)
  const textRaw = tokens["--peek-text"];
  const textRgb = textRaw ? parseRgbColor(textRaw) : null;
  const currentRatio = textRgb ? getContrastRatio(textRgb, bgRgb) : 1;

  if (!textRaw || currentRatio < 4.5) {
    tokens["--peek-text"] = isDark ? "#f1f5f9" : "#0f172a";
  }

  const mutedRaw = tokens["--peek-muted"];
  const mutedRgb = mutedRaw ? parseRgbColor(mutedRaw) : null;
  if (!mutedRaw || (mutedRgb && getContrastRatio(mutedRgb, bgRgb) < 3.0)) {
    tokens["--peek-muted"] = isDark ? "#94a3b8" : "#64748b";
  }
  if (!tokens["--peek-faint"]) {
    tokens["--peek-faint"] = isDark ? "#64748b" : "#94a3b8";
  }

  // User bubble text contrast
  const bubbleBgRgb = parseRgbColor(tokens["--peek-user-bubble-bg"]) || surfaceRgb;
  const bubbleLuminance = getRelativeLuminance(bubbleBgRgb.r, bubbleBgRgb.g, bubbleBgRgb.b);
  if (!tokens["--peek-user-bubble-text"]) {
    tokens["--peek-user-bubble-text"] = bubbleLuminance < 0.45 ? "#f8fafc" : "#0f172a";
  }

  // 5. Accent & Action Buttons
  const accentRaw = tokens["--peek-accent"] || (isDark ? "#38bdf8" : "#2563eb");
  tokens["--peek-accent"] = accentRaw;
  const accentRgb =
    parseRgbColor(accentRaw) || (isDark ? { r: 56, g: 189, b: 248 } : { r: 37, g: 99, b: 235 });
  const accentLuminance = getRelativeLuminance(accentRgb.r, accentRgb.g, accentRgb.b);

  if (!tokens["--peek-accent-fg"]) {
    tokens["--peek-accent-fg"] = accentLuminance > 0.45 ? "#09090b" : "#ffffff";
  }
  tokens["--peek-send-active-bg"] = tokens["--peek-accent"];
  tokens["--peek-send-active-fg"] = tokens["--peek-accent-fg"];
  tokens["--peek-primary-foreground"] = tokens["--peek-accent-fg"];

  if (!tokens["--peek-send-bg"]) {
    tokens["--peek-send-bg"] = isDark
      ? mixColors(surfaceRgb, { r: 255, g: 255, b: 255 }, 0.88)
      : "#e5e5e5";
  }
  if (!tokens["--peek-send-fg"]) {
    tokens["--peek-send-fg"] = isDark ? "#d1d5db" : "#4b5563";
  }
  if (!tokens["--peek-row-active"]) {
    tokens["--peek-row-active"] = isDark
      ? mixColors(surfaceRgb, accentRgb, 0.82)
      : mixColors({ r: 255, g: 255, b: 255 }, accentRgb, 0.88);
  }

  // 6. Icon Customization Tokens
  if (!tokens["--peek-icon"]) {
    tokens["--peek-icon"] = isDark ? "#e2e8f0" : "#334155";
  }
  if (!tokens["--peek-icon-muted"]) {
    tokens["--peek-icon-muted"] = tokens["--peek-muted"];
  }
  if (!tokens["--peek-icon-accent"]) {
    tokens["--peek-icon-accent"] = tokens["--peek-accent"];
  }
  if (!tokens["--peek-icon-glow"]) {
    tokens["--peek-icon-glow"] = "none";
  }

  // 7. Borders & Composer
  if (!tokens["--peek-border"]) {
    tokens["--peek-border"] = isDark ? "rgba(255, 255, 255, 0.1)" : "rgba(0, 0, 0, 0.14)";
  }
  if (!tokens["--peek-strong-border"]) {
    tokens["--peek-strong-border"] = isDark ? "rgba(255, 255, 255, 0.18)" : "rgba(0, 0, 0, 0.22)";
  }
  if (!tokens["--peek-composer-border"]) {
    tokens["--peek-composer-border"] = isDark
      ? "color-mix(in srgb, var(--peek-text) 12%, transparent)"
      : "color-mix(in srgb, var(--peek-text) 9%, transparent)";
  }
  if (!tokens["--peek-composer-border-focus"]) {
    tokens["--peek-composer-border-focus"] = tokens["--peek-composer-border"];
  }
  if (!tokens["--peek-composer-shadow"]) {
    tokens["--peek-composer-shadow"] = "none";
  }
  if (!tokens["--peek-composer-shadow-focus"]) {
    tokens["--peek-composer-shadow-focus"] = "none";
  }
  if (!tokens["--workbench-glass-fill"]) {
    tokens["--workbench-glass-fill"] =
      `color-mix(in srgb, ${tokens["--peek-sidebar"]} var(--workbench-glass-opacity, 82%), transparent)`;
  }
  if (!tokens["--workbench-glass-fill-covering"]) {
    tokens["--workbench-glass-fill-covering"] =
      `color-mix(in srgb, ${tokens["--peek-sidebar"]} 98%, transparent)`;
  }

  return tokens;
}

export function ensureCustomThemeStyleSheet(themes: CustomThemeConfig[] = []): void {
  if (typeof document === "undefined") return;
  let styleEl = document.getElementById(CUSTOM_STYLE_TAG_ID) as HTMLStyleElement | null;
  if (!styleEl) {
    styleEl = document.createElement("style");
    styleEl.id = CUSTOM_STYLE_TAG_ID;
    document.head.appendChild(styleEl);
  }

  if (themes.length === 0) {
    styleEl.textContent = "";
    return;
  }

  const cssRules = themes
    .map((theme) => {
      const harmonized = harmonizeCustomTheme(theme);
      const declarations = Object.entries(harmonized).map(([prop, val]) => `  ${prop}: ${val};`);
      if (theme.background?.image) {
        if (!isLocalImagePath(theme.background.image)) {
          const resolvedUrl = resolveChatImageSrc(theme.background.image);
          declarations.push(`  --theme-bg-image: url(${JSON.stringify(resolvedUrl)});`);
        }
        if (typeof theme.background.opacity === "number") {
          declarations.push(`  --theme-bg-opacity: ${theme.background.opacity};`);
        }
        if (typeof theme.background.blur === "number") {
          declarations.push(`  --theme-bg-blur: ${theme.background.blur}px;`);
        }
        if (theme.background.fit) {
          declarations.push(`  --theme-bg-fit: ${theme.background.fit};`);
        }
      }
      return `html[data-theme="${theme.id}"] {\n${declarations.join("\n")}\n}`;
    })
    .join("\n\n");

  styleEl.textContent = cssRules;
}

function applyDocumentColorScheme(colorScheme: ColorScheme, glass: boolean) {
  const root = rootElement();
  const body = document.body;
  const scheme = documentColorScheme(colorScheme);

  if (root.classList.contains("peek-window") || glass) {
    root.style.colorScheme = "normal";
    root.style.background = "transparent";
    if (body) {
      body.style.colorScheme = scheme;
      body.style.background = "transparent";
    }
    return;
  }

  root.style.colorScheme = scheme;
  root.style.removeProperty("background");
  if (body) {
    body.style.removeProperty("color-scheme");
    body.style.removeProperty("background");
  }
}

function colorSchemeInlineMatches(colorScheme: ColorScheme, glass: boolean): boolean {
  const root = rootElement();
  const scheme = documentColorScheme(colorScheme);
  if (root.classList.contains("peek-window") || glass) {
    return root.style.colorScheme === "normal" && document.body?.style.colorScheme === scheme;
  }
  return root.style.colorScheme === scheme && !document.body?.style.colorScheme;
}

function scheduleColorSchemeRebind(colorScheme: ColorScheme, glass: boolean) {
  const rebind = () => applyDocumentColorScheme(colorScheme, glass);
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      rebind();
      void rootElement().offsetHeight;
    });
  });
  window.setTimeout(rebind, 50);
}

/** Read resolved CSS custom property from the document root. */
export function readThemeToken(name: string, fallback = ""): string {
  const value = getComputedStyle(rootElement()).getPropertyValue(name).trim();
  return value || fallback;
}

export function readThemeState(): ThemeState {
  const root = rootElement();
  const themeId = normalizeColorScheme(root.dataset.theme);
  const customThemes = readCachedCustomThemes();
  return {
    themeId,
    language: (root.lang as AppLanguage) || "zh-CN",
    chromeFrostedGlass: root.classList.contains("chrome-frosted-glass"),
    isDark: isDarkTheme(themeId, customThemes),
  };
}

function dispatchThemeChange(detail: ThemeChangeDetail) {
  window.dispatchEvent(new CustomEvent(THEME_CHANGE_EVENT, { detail }));
}

/** Apply theme by setting html[data-theme]. Does not use a `.dark` class. */
export function applyThemeAppearance(input: ThemeAppearanceInput): ThemeState {
  const customThemes = input.customThemes ?? readCachedCustomThemes();
  cacheCustomThemes(input.customThemes);
  ensureCustomThemeStyleSheet(customThemes);

  const themeId = normalizeColorScheme(input.colorScheme);
  const language = input.language;
  const glass =
    input.chromeFrostedGlass ?? rootElement().classList.contains("chrome-frosted-glass");

  cacheColorScheme(themeId);

  const root = rootElement();
  const unchanged =
    root.dataset.theme === themeId &&
    root.lang === language &&
    root.classList.contains("chrome-frosted-glass") === glass &&
    colorSchemeInlineMatches(themeId, glass);

  applyDocumentColorScheme(themeId, glass);

  const isDark = isDarkTheme(themeId, customThemes);

  if (!unchanged) {
    root.lang = language;
    root.dataset.theme = themeId;
    root.dataset.themeMode = isDark ? "dark" : "light";
    root.classList.remove("dark");
  }

  applyDocumentColorScheme(themeId, glass);
  scheduleColorSchemeRebind(themeId, glass);
  void applyChromeFrostedGlass(glass);

  const state: ThemeState = {
    themeId,
    language,
    chromeFrostedGlass: glass,
    isDark,
  };
  dispatchThemeChange(state);
  return state;
}

/** Boot hint before settings IPC returns (matches index.html splash). */
export function bootstrapThemeAppearance(language: AppLanguage = "zh-CN"): ThemeAppearanceInput {
  return {
    colorScheme: readCachedColorScheme(),
    language,
    chromeFrostedGlass: false,
  };
}

export function isWorkbenchWindow(): boolean {
  try {
    return getCurrentWebviewWindow().label === "workbench";
  } catch {
    return false;
  }
}
