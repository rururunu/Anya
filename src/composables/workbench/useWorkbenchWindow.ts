import { ref, watch } from "vue";
import type { WebviewWindow } from "@tauri-apps/api/webviewWindow";

import { useAppStore } from "@/stores/app";
import { DEFAULT_SETTINGS_CATEGORY, type CategoryId } from "@/types/setting";

export interface UseWorkbenchWindowOptions {
  appWindow: WebviewWindow;
}

/**
 * Native window chrome (minimize / maximize / hide) plus the embedded
 * settings panel's open/category state. Also mirrors `appStore.settingsOpenSignal`
 * so other parts of the app (e.g. tray menu) can request settings to open.
 */
export function useWorkbenchWindow(options: UseWorkbenchWindowOptions) {
  const { appWindow } = options;
  const appStore = useAppStore();

  const isMaximized = ref(false);
  const settingsOpen = ref(false);
  const settingsCategory = ref<CategoryId>(DEFAULT_SETTINGS_CATEGORY);

  function minimizeWindow() {
    void appWindow.minimize();
  }

  async function syncMaximizedState() {
    isMaximized.value = await appWindow.isMaximized();
  }

  async function toggleMaximizeWindow() {
    if (await appWindow.isMaximized()) await appWindow.unmaximize();
    else await appWindow.maximize();
    await syncMaximizedState();
  }

  function hideWindow() {
    void appWindow.hide();
  }

  function openSettings(category?: CategoryId) {
    if (category) {
      settingsCategory.value = category;
    } else if (!settingsOpen.value) {
      settingsCategory.value = DEFAULT_SETTINGS_CATEGORY;
    }
    settingsOpen.value = true;
  }

  function closeSettings() {
    settingsOpen.value = false;
  }

  function toggleSettings() {
    if (settingsOpen.value) closeSettings();
    else openSettings();
  }

  watch(
    () => appStore.settingsOpenSignal,
    () => {
      openSettings(appStore.settingsCategory);
    },
  );

  return {
    isMaximized,
    settingsOpen,
    settingsCategory,
    minimizeWindow,
    syncMaximizedState,
    toggleMaximizeWindow,
    hideWindow,
    openSettings,
    closeSettings,
    toggleSettings,
  };
}
