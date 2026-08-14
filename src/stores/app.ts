import { defineStore } from "pinia";

import { DEFAULT_SETTINGS_CATEGORY, type CategoryId } from "@/types/setting";

export const useAppStore = defineStore("app", {
  state: () => ({
    dark: true,
    /** Bumped to request the workbench open settings (with category). */
    settingsOpenSignal: 0,
    settingsCategory: DEFAULT_SETTINGS_CATEGORY,
  }),
  actions: {
    openSettings(category: CategoryId = DEFAULT_SETTINGS_CATEGORY) {
      this.settingsCategory = category;
      this.settingsOpenSignal += 1;
    },
  },
});
