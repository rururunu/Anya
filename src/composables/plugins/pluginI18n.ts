import { useSettingStore } from "@/stores/setting";

/**
 * Minimal i18n contract for plugin-contributed text: plugins hand Anya a
 * per-locale dict and a fallback, Anya picks the active locale. Plugins are
 * not required to build their own i18n system, but Anya doesn't own their
 * strings either.
 */
export function createPluginI18n() {
  return {
    t(_key: string, fallback: string, dict?: Record<string, string>): string {
      if (!dict) return fallback;
      const locale = useSettingStore().language ?? "zh-CN";
      return dict[locale] ?? dict[locale.split("-")[0]] ?? fallback;
    },
  };
}
