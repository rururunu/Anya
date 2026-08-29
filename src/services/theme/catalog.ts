import type { AppLanguage, SelectOption, ThemeId } from "@/types/setting";

export type { ThemeId };
export type { ColorScheme } from "@/types/setting";

export const COLOR_SCHEME_CACHE_KEY = "anya-color-scheme";

export const THEME_IDS = ["light", "dark"] as const satisfies readonly ThemeId[];

const LEGACY_ALIASES: Record<string, ThemeId> = {
  system: "dark",
  auto: "dark",
  default: "dark",
  frost: "light",
  cream: "light",
  paper: "light",
  midnight: "dark",
  "blue-black": "dark",
  ocean: "dark",
  forest: "dark",
  rose: "dark",
  "ghost-pastel": "dark",
  graphite: "dark",
  ember: "dark",
  nocturne: "dark",
  teal: "dark",
};

function themeLabel(en: string, zh: string): SelectOption<ThemeId>["label"] {
  return { "en-US": en, "zh-CN": zh };
}

export const colorSchemeOptions: SelectOption<ThemeId>[] = [
  { value: "light", label: themeLabel("Light", "浅色") },
  { value: "dark", label: themeLabel("Dark", "深色") },
];

export const themeOptionGroups: Array<{
  id: string;
  label: Partial<Record<AppLanguage, string>> & Pick<Record<AppLanguage, string>, "en-US">;
  options: SelectOption<ThemeId>[];
}> = [
  {
    id: "appearance",
    label: themeLabel("Theme", "主题"),
    options: colorSchemeOptions,
  },
];

/** Returns whether the scheme uses the dark palette. */
export function isDarkTheme(scheme: ThemeId): boolean {
  return scheme === "dark";
}

export function isLightColorScheme(scheme: ThemeId): boolean {
  return scheme === "light";
}

export function isThemeId(value: string): value is ThemeId {
  return (THEME_IDS as readonly string[]).includes(value);
}

/** Coerce unknown / legacy palette ids to light or dark. */
export function normalizeThemeId(value: unknown): ThemeId {
  if (typeof value !== "string") {
    return "light";
  }
  const trimmed = value.trim().toLowerCase();
  const aliased = LEGACY_ALIASES[trimmed] ?? trimmed;
  return isThemeId(aliased) ? aliased : "light";
}

export const normalizeColorScheme = normalizeThemeId;

/** Read cached color scheme from localStorage for boot splash. */
export function readCachedColorScheme(): ThemeId {
  try {
    const cached = localStorage.getItem(COLOR_SCHEME_CACHE_KEY);
    if (cached) {
      return normalizeThemeId(cached);
    }
  } catch {
    // ignore private mode / quota
  }
  return "light";
}

export const readCachedThemeId = readCachedColorScheme;
