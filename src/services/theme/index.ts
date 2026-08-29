export type { ThemeId, ColorScheme } from "./catalog";

export type { ThemeAppearanceInput, ThemeChangeDetail, ThemeState } from "./types";

export {
  applyThemeAppearance,
  bootstrapThemeAppearance,
  documentColorScheme,
  readThemeState,
  readThemeToken,
  THEME_CHANGE_EVENT,
} from "./ThemeService";

export { isDarkTheme, normalizeThemeId, themeOptionGroups } from "./catalog";
