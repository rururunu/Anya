import type { AppLanguage, ThemeId } from "@/types/setting";

export type { ThemeId };

export interface ThemeAppearanceInput {
  colorScheme: ThemeId;
  language: AppLanguage;
  chromeFrostedGlass?: boolean;
}

export interface ThemeState {
  themeId: ThemeId;
  language: AppLanguage;
  chromeFrostedGlass: boolean;
  isDark: boolean;
}

export type ThemeChangeDetail = ThemeState;

export const THEME_CHANGE_EVENT = "peek-theme-change";
