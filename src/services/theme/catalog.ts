import type {
  AppLanguage,
  BuiltinThemeId,
  CustomThemeConfig,
  SelectOption,
  ThemeId,
} from "@/types/setting";

export type { ThemeId, BuiltinThemeId };
export type { ColorScheme } from "@/types/setting";

export const COLOR_SCHEME_CACHE_KEY = "anya-color-scheme";
export const CUSTOM_THEMES_CACHE_KEY = "anya-custom-themes";

export const BUILTIN_THEME_IDS = ["light", "dark"] as const satisfies readonly BuiltinThemeId[];

export const THEME_IDS = BUILTIN_THEME_IDS;

const LEGACY_ALIASES: Record<string, BuiltinThemeId> = {
  system: "dark",
  auto: "dark",
  default: "dark",
  "blue-black": "dark",
  "ghost-pastel": "dark",
  graphite: "dark",
  nocturne: "dark",
  teal: "dark",
  frost: "light",
  cream: "light",
  paper: "light",
  midnight: "dark",
  ocean: "dark",
  forest: "dark",
  rose: "dark",
  ember: "dark",
};

function themeLabel(en: string, zh: string): SelectOption<ThemeId>["label"] {
  return { "en-US": en, "zh-CN": zh };
}

export const lightBuiltinOptions: SelectOption<ThemeId>[] = [
  { value: "light", label: themeLabel("Light", "浅色") },
];

export const darkBuiltinOptions: SelectOption<ThemeId>[] = [
  { value: "dark", label: themeLabel("Dark", "深色") },
];

export const colorSchemeOptions: SelectOption<ThemeId>[] = [
  ...lightBuiltinOptions,
  ...darkBuiltinOptions,
];

export const themeOptionGroups: Array<{
  id: string;
  label: Partial<Record<AppLanguage, string>> & Pick<Record<AppLanguage, string>, "en-US">;
  options: SelectOption<ThemeId>[];
}> = [
  {
    id: "builtin-presets",
    label: themeLabel("System Presets", "系统预设"),
    options: colorSchemeOptions,
  },
];

export function buildFullThemeGroups(customThemes: CustomThemeConfig[] = []): Array<{
  id: string;
  label: Partial<Record<AppLanguage, string>> & Pick<Record<AppLanguage, string>, "en-US">;
  options: SelectOption<ThemeId>[];
}> {
  const groups: Array<{
    id: string;
    label: Partial<Record<AppLanguage, string>> & Pick<Record<AppLanguage, string>, "en-US">;
    options: SelectOption<ThemeId>[];
  }> = [
    {
      id: "builtin-presets",
      label: themeLabel("System Presets", "系统预设"),
      options: colorSchemeOptions,
    },
  ];
  if (customThemes.length > 0) {
    groups.push({
      id: "custom-themes",
      label: themeLabel("Custom Themes", "自定义主题"),
      options: customThemes.map((theme) => ({
        value: theme.id,
        label: { "en-US": theme.name, "zh-CN": theme.name },
      })),
    });
  }
  return groups;
}

/** Returns whether the scheme uses the dark palette. */
export function isDarkTheme(scheme: ThemeId, customThemes?: CustomThemeConfig[]): boolean {
  if (scheme === "dark") {
    return true;
  }
  if (scheme === "light") {
    return false;
  }
  if (LEGACY_ALIASES[scheme]) {
    return LEGACY_ALIASES[scheme] === "dark";
  }
  if (customThemes) {
    const found = customThemes.find((t) => t.id === scheme);
    if (found) {
      return found.mode === "dark";
    }
  }
  return scheme.includes("dark") || scheme.includes("night");
}

export function isLightColorScheme(scheme: ThemeId, customThemes?: CustomThemeConfig[]): boolean {
  return !isDarkTheme(scheme, customThemes);
}

export function isBuiltinThemeId(value: string): value is BuiltinThemeId {
  return (BUILTIN_THEME_IDS as readonly string[]).includes(value);
}

export const isThemeId = isBuiltinThemeId;

/** Coerce unknown / legacy palette ids or return custom id. */
export function normalizeThemeId(value: unknown): ThemeId {
  if (typeof value !== "string") {
    return "light";
  }
  const trimmed = value.trim();
  const lower = trimmed.toLowerCase();
  if (LEGACY_ALIASES[lower]) {
    return LEGACY_ALIASES[lower];
  }
  if (isBuiltinThemeId(lower)) {
    return lower;
  }
  return trimmed || "light";
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
