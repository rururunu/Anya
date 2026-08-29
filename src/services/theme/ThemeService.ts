import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

import { applyChromeFrostedGlass } from "@/services/overlay/appearance";
import type { AppLanguage, ColorScheme } from "@/types/setting";
import {
  COLOR_SCHEME_CACHE_KEY,
  isDarkTheme,
  normalizeColorScheme,
  readCachedColorScheme,
} from "./catalog";

import {
  THEME_CHANGE_EVENT,
  type ThemeAppearanceInput,
  type ThemeChangeDetail,
  type ThemeState,
} from "./types";

export { THEME_CHANGE_EVENT };
export type { ThemeAppearanceInput, ThemeChangeDetail, ThemeState, ThemeId } from "./types";

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

function applyDocumentColorScheme(colorScheme: ColorScheme, glass: boolean) {
  const root = rootElement();
  const body = document.body;
  const scheme = documentColorScheme(colorScheme);

  if (glass) {
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
  if (glass) {
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
  return {
    themeId,
    language: (root.lang as AppLanguage) || "zh-CN",
    chromeFrostedGlass: root.classList.contains("chrome-frosted-glass"),
    isDark: isDarkTheme(themeId),
  };
}

function dispatchThemeChange(detail: ThemeChangeDetail) {
  window.dispatchEvent(new CustomEvent(THEME_CHANGE_EVENT, { detail }));
}

/** Apply light/dark by setting html[data-theme]. Does not use a `.dark` class. */
export function applyThemeAppearance(input: ThemeAppearanceInput): ThemeState {
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

  if (!unchanged) {
    root.lang = language;
    root.dataset.theme = themeId;
    root.classList.remove("dark");
  }

  applyDocumentColorScheme(themeId, glass);
  scheduleColorSchemeRebind(themeId, glass);
  void applyChromeFrostedGlass(glass);

  const state: ThemeState = {
    themeId,
    language,
    chromeFrostedGlass: glass,
    isDark: isDarkTheme(themeId),
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
