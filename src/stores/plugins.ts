import { defineStore } from "pinia";
import { computed, reactive, ref } from "vue";
import {
  listUserPlugins,
  pluginSafeModeStatus,
  reportPluginRuntimeError,
  clearPluginRuntimeErrors,
  type UserPluginSummary,
} from "@/services/plugins/ipc";
import {
  getSlotConflicts,
  getSlotEntries,
  mountSlot,
  unmountPluginFromAllAnchors,
  unmountSlot,
  type PluginChromeSurface,
  type PluginMount,
  type PluginTabClick,
  type SlotConflict,
} from "@/composables/plugins/slotRegistry";
import {
  unregisterPluginAssets,
  getAssetConflicts,
  type AssetConflict,
} from "@/composables/plugins/assetRegistry";
import { unsubscribeAll } from "@/composables/plugins/eventBus";

export type { PluginChromeSurface, PluginMount };

export type PluginSidebarTab = {
  id: string;
  pluginId: string;
  title: string;
  icon?: string;
  mount: PluginMount;
  /** Launcher chrome. `nav` without `views` opens the center pane; `views` is the right review strip. */
  surfaces: PluginChromeSurface[];
  /** Per-surface content override; a surface not listed here falls back to `mount`. */
  content?: Partial<Record<PluginChromeSurface, PluginMount>>;
  /** Header-only tabs fire this instead of opening a pane. */
  onClick?: PluginTabClick;
};

export type PluginMainView = {
  id: string;
  pluginId: string;
  title: string;
  mount: PluginMount;
};

export type PluginErrorEntry = {
  pluginId: string;
  phase: "activate" | "deactivate" | "mount" | "host";
  message: string;
  at: number;
};

const MAX_ERROR_ENTRIES = 100;
const HOME_ID_KEY = "anya.plugin.homeId";

function readPersistedHomeId(): string | null {
  try {
    return localStorage.getItem(HOME_ID_KEY)?.trim() || null;
  } catch {
    return null;
  }
}

export const usePluginsStore = defineStore("plugins", () => {
  const plugins = ref<UserPluginSummary[]>([]);
  const pluginsLoaded = ref(false);
  const selectedPluginId = ref<string | null>(readPersistedHomeId());
  const activeSidebarTabId = ref<string | null>(null);
  /** Plugin whose 资料区 sits above the live conversation (cleared when leaving that workspace). */
  const conversationWorkspacePluginId = ref<string | null>(null);
  const warningAcknowledged = ref(false);
  const safeMode = ref(false);
  const safeModeReason = ref("");
  const errors = ref<PluginErrorEntry[]>([]);

  /**
   * Settings view for a plugin's home page, keyed by plugin id. Not an
   * anchor/slot (no launcher, no multiplicity) — just direct storage set via
   * `ctx.home.setSettingsView(mount)`.
   */
  const homeSettingsViews = reactive(new Map<string, PluginMount>());

  // Sidebar tabs / main view stay as thin wrappers (click + exclusive pane).
  // Composer, materials, and review panes read SlotRegistry via PluginSlotOutlet.
  const sidebarTabs = computed<PluginSidebarTab[]>(() =>
    getSlotEntries("sidebar.tabs").map((entry) => ({
      id: entry.id,
      pluginId: entry.pluginId,
      title: entry.title ?? entry.id,
      icon: entry.icon,
      mount: entry.mount,
      surfaces: entry.chrome ?? ["views"],
      content: entry.content,
      onClick: entry.onClick,
    })),
  );

  const mainView = computed<PluginMainView | null>(() => {
    const entry = getSlotEntries("workbench.main")[0];
    if (!entry) return null;
    return {
      id: entry.id,
      pluginId: entry.pluginId,
      title: entry.title ?? entry.id,
      mount: entry.mount,
    };
  });

  const slotConflicts = computed<SlotConflict[]>(() => getSlotConflicts());
  const assetConflicts = computed<AssetConflict[]>(() => getAssetConflicts());

  function addSidebarTab(tab: PluginSidebarTab) {
    mountSlot("sidebar.tabs", {
      id: tab.id,
      pluginId: tab.pluginId,
      title: tab.title,
      icon: tab.icon,
      mount: tab.mount,
      chrome: tab.surfaces,
      content: tab.content,
      onClick: tab.onClick,
    });
  }

  function removeSidebarTab(id: string) {
    unmountSlot("sidebar.tabs", id);
    if (activeSidebarTabId.value === id) {
      const remaining = getSlotEntries("sidebar.tabs");
      activeSidebarTabId.value = remaining[remaining.length - 1]?.id ?? null;
    }
  }

  function setMainView(view: PluginMainView) {
    mountSlot("workbench.main", view);
  }

  function clearMainView() {
    const current = getSlotEntries("workbench.main")[0];
    if (current) unmountSlot("workbench.main", current.id);
  }

  function persistHomeId(id: string | null) {
    try {
      if (id) localStorage.setItem(HOME_ID_KEY, id);
      else localStorage.removeItem(HOME_ID_KEY);
    } catch {
      /* ignore quota / private-mode */
    }
  }

  function setSelectedPluginId(id: string | null) {
    selectedPluginId.value = id;
    persistHomeId(id);
  }

  function pruneSelectedPluginId() {
    const id = selectedPluginId.value;
    if (id && !plugins.value.some((plugin) => plugin.id === id)) setSelectedPluginId(null);
  }

  function removePluginSurface(pluginId: string) {
    unmountPluginFromAllAnchors(pluginId);
    unregisterPluginAssets(pluginId);
    unsubscribeAll(pluginId);
    homeSettingsViews.delete(pluginId);
    if (conversationWorkspacePluginId.value === pluginId)
      conversationWorkspacePluginId.value = null;
    if (selectedPluginId.value === pluginId) setSelectedPluginId(null);
  }

  function setHomeSettingsView(pluginId: string, mount: PluginMount) {
    homeSettingsViews.set(pluginId, mount);
  }

  function clearHomeSettingsView(pluginId: string) {
    homeSettingsViews.delete(pluginId);
  }

  /**
   * Ring-buffered plugin faults so one crash is visible instead of a silent
   * console.warn. Also mirrored to Rust (`reportPluginRuntimeError`) so the
   * agent's `manage_plugin errors`/`reload` tool calls can see faults that
   * only happen inside this JS heap.
   */
  function recordError(entry: Omit<PluginErrorEntry, "at">) {
    errors.value = [...errors.value, { ...entry, at: Date.now() }].slice(-MAX_ERROR_ENTRIES);
    void reportPluginRuntimeError(entry.pluginId, entry.phase, entry.message).catch(() => {});
  }

  function clearErrors(pluginId?: string) {
    errors.value = pluginId ? errors.value.filter((e) => e.pluginId !== pluginId) : [];
    void clearPluginRuntimeErrors(pluginId).catch(() => {});
  }

  async function refresh() {
    const [items, status] = await Promise.all([listUserPlugins(), pluginSafeModeStatus()]);
    plugins.value = items;
    safeMode.value = status.active;
    safeModeReason.value = status.reason;
    pruneSelectedPluginId();
    pluginsLoaded.value = true;
  }

  return {
    plugins,
    pluginsLoaded,
    selectedPluginId,
    setSelectedPluginId,
    sidebarTabs,
    activeSidebarTabId,
    conversationWorkspacePluginId,
    mainView,
    homeSettingsViews,
    slotConflicts,
    assetConflicts,
    errors,
    warningAcknowledged,
    safeMode,
    safeModeReason,
    addSidebarTab,
    removeSidebarTab,
    setMainView,
    clearMainView,
    removePluginSurface,
    setHomeSettingsView,
    clearHomeSettingsView,
    recordError,
    clearErrors,
    refresh,
  };
});
