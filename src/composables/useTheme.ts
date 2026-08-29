import { onMounted, onUnmounted, readonly, ref, computed, type Ref } from "vue";

import {
  readThemeState,
  THEME_CHANGE_EVENT,
  type ThemeChangeDetail,
  type ThemeState,
} from "@/services/theme";
import { isDarkTheme } from "@/services/theme/catalog";
import type { ColorScheme } from "@/types/setting";

const colorSchemeRef = ref<ColorScheme>(readThemeState().themeId);

function syncFromDom() {
  colorSchemeRef.value = readThemeState().themeId;
}

/** Reactive app theme — reads html[data-theme], updates on peek-theme-change. */
export function useTheme(): {
  colorScheme: Readonly<Ref<ColorScheme>>;
  isDark: Ref<boolean>;
} {
  onMounted(() => {
    syncFromDom();
    window.addEventListener(THEME_CHANGE_EVENT, onThemeChange);
  });

  onUnmounted(() => {
    window.removeEventListener(THEME_CHANGE_EVENT, onThemeChange);
  });

  function onThemeChange(event: Event) {
    const detail = (event as CustomEvent<ThemeChangeDetail>).detail;
    if (detail?.themeId) {
      colorSchemeRef.value = detail.themeId;
      return;
    }
    syncFromDom();
  }

  const isDark = computed(() => isDarkTheme(colorSchemeRef.value));

  return {
    colorScheme: readonly(colorSchemeRef),
    isDark,
  };
}

/** Subscribe to theme changes outside of component setup (e.g. canvas re-render). */
export function onThemeChange(listener: (state: ThemeState) => void): () => void {
  const handler = (event: Event) => {
    const detail = (event as CustomEvent<ThemeChangeDetail>).detail;
    listener(detail ?? readThemeState());
  };
  window.addEventListener(THEME_CHANGE_EVENT, handler);
  return () => window.removeEventListener(THEME_CHANGE_EVENT, handler);
}
