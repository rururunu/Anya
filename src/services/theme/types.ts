import type {
  AppLanguage,
  CustomThemeConfig,
  ThemeBackgroundConfig,
  ThemeId,
} from "@/types/setting";

export type { ThemeId, CustomThemeConfig, ThemeBackgroundConfig };

export interface ThemeAppearanceInput {
  colorScheme: ThemeId;
  language: AppLanguage;
  chromeFrostedGlass?: boolean;
  customThemes?: CustomThemeConfig[];
}

export interface ThemeState {
  themeId: ThemeId;
  language: AppLanguage;
  chromeFrostedGlass: boolean;
  isDark: boolean;
}

export type ThemeChangeDetail = ThemeState;

export const THEME_CHANGE_EVENT = "peek-theme-change";
