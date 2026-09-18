import type { AppLanguage } from "@/types/setting";

export type FontKind = "latin" | "cjk" | "mono";
export type FontSettingKey = "fontLatin" | "fontCjk" | "fontMono";

export const FONT_DEFAULT = "default";
export const FONT_CUSTOM = "__custom__";

export const LATIN_FALLBACK = [
  "Source Sans 3 Variable",
  "Source Sans 3",
  "Calibri",
  "Aptos",
  "Segoe UI",
  "Helvetica Neue",
  "Arial",
  "sans-serif",
] as const;

export const CJK_FALLBACK = [
  "Microsoft YaHei",
  "微软雅黑",
  "Microsoft YaHei UI",
  "PingFang SC",
  "Hiragino Sans GB",
  "Noto Sans SC",
  "sans-serif",
] as const;

export const MONO_FALLBACK = [
  "Cascadia Code",
  "Cascadia Mono",
  "JetBrains Mono Variable",
  "Consolas",
  "SF Mono",
  "Menlo",
  "ui-monospace",
  "monospace",
] as const;

const GENERIC_FAMILY =
  /^(sans-serif|serif|monospace|system-ui|ui-sans-serif|ui-serif|ui-monospace|ui-rounded|-apple-system|BlinkMacSystemFont)$/i;

const KIND_FALLBACK: Record<FontKind, readonly string[]> = {
  latin: LATIN_FALLBACK,
  cjk: CJK_FALLBACK,
  mono: MONO_FALLBACK,
};

const KIND_SETTING: Record<FontKind, FontSettingKey> = {
  latin: "fontLatin",
  cjk: "fontCjk",
  mono: "fontMono",
};

/** Strip CSS-breaking characters from a user-entered family name. */
export function sanitizeFontName(value: string): string {
  return value
    .replace(/[;{}<>]/g, "")
    .trim()
    .slice(0, 64);
}

/** Setting key for a font kind. */
export function fontSettingKey(kind: FontKind): FontSettingKey {
  return KIND_SETTING[kind];
}

/** Quote a family name for a CSS font-family list. */
export function quoteFontFamily(name: string): string {
  const trimmed = name.trim();
  if (!trimmed) return "";
  if (GENERIC_FAMILY.test(trimmed) || /^[A-Za-z0-9-]+$/.test(trimmed)) return trimmed;
  return `"${trimmed.replace(/"/g, "")}"`;
}

/** CSS font-family stack for a kind, with an optional user-chosen primary. */
export function fontStack(kind: FontKind, primary?: string): string {
  const fallback = KIND_FALLBACK[kind];
  const name = sanitizeFontName(primary ?? "");
  if (!name || name === FONT_DEFAULT) {
    return fallback.map(quoteFontFamily).join(", ");
  }
  const rest = fallback.filter((item) => item.toLowerCase() !== name.toLowerCase());
  return [quoteFontFamily(name), ...rest.map(quoteFontFamily)].join(", ");
}

/** Combined UI sans: Latin first so English is not YaHei's Latin glyphs. */
export function uiSansStack(settings: { fontLatin?: string; fontCjk?: string }): string {
  return `${fontStack("latin", settings.fontLatin)}, ${fontStack("cjk", settings.fontCjk)}`;
}

/** Apply UI / code font CSS variables from stored family names (empty = default stack). */
export function applyUiFonts(settings: {
  fontLatin?: string;
  fontCjk?: string;
  fontMono?: string;
}) {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  root.style.setProperty("--font-sans-latin", fontStack("latin", settings.fontLatin));
  root.style.setProperty("--font-sans-cjk", fontStack("cjk", settings.fontCjk));
  root.style.setProperty("--peek-font-sans", uiSansStack(settings));
  root.style.setProperty("--font-mono", fontStack("mono", settings.fontMono));
}

export function isFontPreset(kind: FontKind, value: string): boolean {
  return fontPresetFamilies(kind).some((item) => item.toLowerCase() === value.toLowerCase());
}

export function fontPresetFamilies(kind: FontKind): string[] {
  if (kind === "latin") {
    return ["Source Sans 3 Variable", "Calibri", "Aptos", "Segoe UI", "Inter Variable", "Arial"];
  }
  if (kind === "cjk") {
    return [
      "Microsoft YaHei",
      "Microsoft YaHei UI",
      "PingFang SC",
      "Noto Sans SC",
      "Source Han Sans SC",
      "SimHei",
      "SimSun",
      "KaiTi",
      "FangSong",
    ];
  }
  return [
    "Cascadia Code",
    "Cascadia Mono",
    "JetBrains Mono Variable",
    "Consolas",
    "Fira Code",
    "Source Code Pro",
    "IBM Plex Mono",
    "SF Mono",
  ];
}

export function fontSelectValue(stored: string, kind: FontKind): string {
  const name = sanitizeFontName(stored);
  if (!name || name === FONT_DEFAULT) return FONT_DEFAULT;
  if (isFontPreset(kind, name)) return name;
  return FONT_CUSTOM;
}

export function defaultFontLabel(kind: FontKind, language: AppLanguage): string {
  if (language === "zh-CN") {
    if (kind === "cjk") return "默认（微软雅黑）";
    if (kind === "latin") return "默认（Source Sans 3）";
    return "默认（Cascadia Code）";
  }
  if (kind === "cjk") return "Default (Microsoft YaHei)";
  if (kind === "latin") return "Default (Source Sans 3)";
  return "Default (Cascadia Code)";
}

export function customFontLabel(language: AppLanguage): string {
  return language === "zh-CN" ? "自定义…" : "Custom…";
}

export function customFontPlaceholder(language: AppLanguage): string {
  return language === "zh-CN" ? "输入字体名称，例如 Microsoft YaHei" : "Font family name";
}
