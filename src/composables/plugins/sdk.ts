import type { App } from "vue";
import type { Pinia } from "pinia";
import {
  listenPluginHostEvents,
  listenPluginReloadRequests,
  pluginFsPick,
  pluginHostRpc,
  getPluginUiSource,
  reportPluginRuntimeError,
  type PluginFsPickOptions,
  type PluginPickedFile,
} from "@/services/plugins/ipc";
import { usePluginsStore, type PluginSidebarTab } from "@/stores/plugins";
import { useChatStore } from "@/stores/chat";
import { useSettingStore } from "@/stores/setting";
import {
  listAnchors,
  mountSlot,
  normalizeTabChrome,
  unmountSlot,
  type AnchorDescriptor,
  type PluginChromeSurface,
  type PluginTabClick,
} from "@/composables/plugins/slotRegistry";
import {
  listAssetKeys,
  registerAsset,
  unregisterAsset,
  type AssetKind,
} from "@/composables/plugins/assetRegistry";
import { publish, subscribe } from "@/composables/plugins/eventBus";
import { createPluginI18n } from "@/composables/plugins/pluginI18n";
import { resolveSlotIcon } from "@/composables/plugins/pluginIcon";
import {
  createPluginAgentSurface,
  type PluginAgentSurface,
} from "@/composables/plugins/conversationSend";
import {
  createPluginWorkspaceSurface,
  type PluginWorkspaceSurface,
} from "@/composables/plugins/workspace";

export type { PluginAgentSurface, PluginWorkspaceSurface };

export type PluginMount = (el: HTMLElement) => void | (() => void);

const noopMount: PluginMount = () => {};

export type PluginActivateContext = {
  pluginId: string;
  app: App;
  pinia: Pinia;
  sidebar: {
    addTab: (tab: {
      id: string;
      title: string;
      icon?: string;
      mount?: PluginMount;
      /** Header-only (`["header"]`, no nav/views): run this and do not open a pane. */
      onClick?: PluginTabClick;
      /** Launcher location. Pane: `nav` without `views` = center 主视图; `views` = right review strip. */
      surfaces?: PluginChromeSurface[];
      /** Per-surface content override, e.g. a compact `header` widget vs. a full `views` pane. Falls back to `mount`. */
      content?: Partial<Record<PluginChromeSurface, PluginMount>>;
    }) => void;
    removeTab: (id: string) => void;
  };
  workbench: {
    setView: (view: { id: string; title: string; mount: PluginMount } | null) => void;
  };
  /**
   * The plugin's home page — reached from the installed-plugins list, not
   * from Anya's chrome. The "详情/About" tab reads the manifest's `about`
   * file; this is the "设置/Settings" tab, for parameters the plugin owns.
   */
  home: {
    setSettingsView: (mount: PluginMount) => void;
    clearSettingsView: () => void;
  };
  composer: {
    addAccessory: (item: { id: string; mount: PluginMount }) => void;
    removeAccessory: (id: string) => void;
  };
  /** Anya's agent: `run` = isolated plugin session; `mount`/`send` wrap the open chat. */
  agent: PluginAgentSurface;
  /** Open the OS terminal at a workspace root (same as workbench「在终端中打开」). */
  workspace: PluginWorkspaceSurface;
  /** Aliases of `agent` plus compat `addMaterials` (prefer composing `workbench.main`). */
  conversation: PluginAgentSurface & {
    addMaterials: (item: { id: string; mount: PluginMount }) => void;
    removeMaterials: (id: string) => void;
  };
  /** Generic named-anchor mounting; `sidebar`/`workbench`/`composer` above are thin wrappers over this. */
  slots: {
    list: () => AnchorDescriptor[];
    mount: (
      anchorId: string,
      view: {
        id: string;
        title?: string;
        icon?: string;
        mount?: PluginMount;
        onClick?: PluginTabClick;
        surfaces?: PluginChromeSurface[];
        content?: Partial<Record<PluginChromeSurface, PluginMount>>;
      },
    ) => boolean;
    unmount: (anchorId: string, id: string) => void;
  };
  /** Resource-key overrides (mascot/tray/backdrop...) rendered by Anya's own container. */
  assets: {
    list: () => string[];
    register: (key: string, asset: { kind: AssetKind; source: string }) => boolean;
    unregister: (key: string) => void;
  };
  /** Namespaced pub/sub for plugin-to-plugin or agent-to-agent messaging. */
  bus: {
    publish: (topic: string, payload: unknown) => void;
    subscribe: (topic: string, fn: (payload: unknown) => void) => () => void;
  };
  i18n: {
    t: (key: string, fallback: string, dict?: Record<string, string>) => string;
  };
  stores: {
    chat: ReturnType<typeof useChatStore>;
    setting: ReturnType<typeof useSettingStore>;
  };
  host: {
    rpc: (method: string, params?: Record<string, unknown>) => Promise<unknown>;
    on: (event: string, fn: (payload: Record<string, unknown>) => void) => () => void;
  };
  /** Native file dialog. Requires `fs.pick`. Cancel → `null`. Use `url` as an asset/`<img>`/`<video>` source. */
  fs: {
    pick: (options?: PluginFsPickOptions) => Promise<PluginPickedFile[] | null>;
  };
  onDeactivate: (fn: () => void) => void;
};

type LoadedPlugin = {
  deactivate?: () => void;
  cleanups: Array<() => void>;
  blobUrl?: string;
};

const loaded = new Map<string, LoadedPlugin>();
let hostEventsStarted = false;
let reloadListenerStarted = false;
const hostListeners = new Set<(payload: Record<string, unknown>) => void>();
let bootApp: App | null = null;
let bootPinia: Pinia | null = null;

async function ensureHostEvents() {
  if (hostEventsStarted) return;
  hostEventsStarted = true;
  await listenPluginHostEvents((payload) => {
    for (const fn of hostListeners) fn(payload);
  });
}

/** Re-run `activate()` when Rust enable/reload/disable notifies this webview. */
async function ensureReloadListener() {
  if (reloadListenerStarted) return;
  reloadListenerStarted = true;
  await listenPluginReloadRequests(async (pluginId) => {
    if (!bootApp || !bootPinia) return;
    const pluginsStore = usePluginsStore(bootPinia);
    await pluginsStore.refresh();
    const enabled = pluginsStore.plugins.some((item) => item.id === pluginId && item.enabled);
    if (!enabled) {
      await deactivatePlugin(pluginId);
      return;
    }
    try {
      const source = await getPluginUiSource(pluginId);
      await activatePlugin(pluginId, source, bootApp, bootPinia);
      await reportPluginRuntimeError(pluginId, "reload-ok", "activated without error").catch(
        () => {},
      );
    } catch {
      // activatePlugin already reported the failure via recordError.
    }
  });
}

/** Install `Anya.plugins` on the workbench page (same JS heap as plugins). */
export function installPluginSdk(app: App, pinia: Pinia) {
  bootApp = app;
  bootPinia = pinia;
  const api = { _app: app, _pinia: pinia };
  (window as Window & { Anya?: { plugins: typeof api } }).Anya = { plugins: api };
  void ensureHostEvents();
  void ensureReloadListener();
}

export function createPluginContext(
  pluginId: string,
  app: App,
  pinia: Pinia,
): PluginActivateContext {
  const pluginsStore = usePluginsStore(pinia);
  const cleanups: Array<() => void> = [];
  const record = loaded.get(pluginId) ?? { cleanups };
  loaded.set(pluginId, record);
  const agent = createPluginAgentSurface(pluginId, (fn) => record.cleanups.push(fn));

  return {
    pluginId,
    app,
    pinia,
    sidebar: {
      addTab(tab) {
        const entry: PluginSidebarTab = {
          id: `${pluginId}:${tab.id}`,
          pluginId,
          title: tab.title,
          icon: resolveSlotIcon(
            pluginId,
            tab.icon,
            pluginsStore.plugins.find((p) => p.id === pluginId)?.icon,
          ),
          mount: tab.mount ?? noopMount,
          surfaces: normalizeTabChrome(tab.surfaces),
          content: tab.content,
          onClick: tab.onClick,
        };
        pluginsStore.addSidebarTab(entry);
        record.cleanups.push(() => pluginsStore.removeSidebarTab(entry.id));
      },
      removeTab(id) {
        pluginsStore.removeSidebarTab(`${pluginId}:${id}`);
      },
    },
    home: {
      setSettingsView(mount) {
        pluginsStore.setHomeSettingsView(pluginId, mount);
        record.cleanups.push(() => pluginsStore.clearHomeSettingsView(pluginId));
      },
      clearSettingsView() {
        pluginsStore.clearHomeSettingsView(pluginId);
      },
    },
    workbench: {
      setView(view) {
        if (!view) {
          pluginsStore.clearMainView();
          return;
        }
        pluginsStore.setMainView({
          id: `plugin:${pluginId}:${view.id}`,
          pluginId,
          title: view.title,
          mount: view.mount,
        });
      },
    },
    composer: {
      addAccessory(item) {
        const id = `${pluginId}:${item.id}`;
        const ok = mountSlot("composer.accessory", { id, pluginId, mount: item.mount });
        if (ok) record.cleanups.push(() => unmountSlot("composer.accessory", id));
      },
      removeAccessory(id) {
        unmountSlot("composer.accessory", `${pluginId}:${id}`);
      },
    },
    agent,
    workspace: createPluginWorkspaceSurface(),
    conversation: {
      ...agent,
      addMaterials(item) {
        const id = `${pluginId}:${item.id}`;
        const ok = mountSlot("conversation.materials", { id, pluginId, mount: item.mount });
        if (ok) record.cleanups.push(() => unmountSlot("conversation.materials", id));
      },
      removeMaterials(id) {
        unmountSlot("conversation.materials", `${pluginId}:${id}`);
      },
    },
    slots: {
      list: () => listAnchors(),
      mount(anchorId, view) {
        const ok = mountSlot(anchorId, {
          ...view,
          id: `${pluginId}:${view.id}`,
          pluginId,
          icon: resolveSlotIcon(
            pluginId,
            view.icon,
            pluginsStore.plugins.find((p) => p.id === pluginId)?.icon,
          ),
          chrome: view.surfaces,
          onClick: view.onClick,
          mount: view.mount ?? noopMount,
        });
        if (ok) record.cleanups.push(() => unmountSlot(anchorId, `${pluginId}:${view.id}`));
        return ok;
      },
      unmount(anchorId, id) {
        unmountSlot(anchorId, `${pluginId}:${id}`);
      },
    },
    assets: {
      list: () => listAssetKeys(),
      register(key, asset) {
        const ok = registerAsset(key, pluginId, asset);
        if (ok) {
          record.cleanups.push(() => unregisterAsset(key, pluginId));
          if (key === "pet.stage.skin" || key === "pet.stage.atlas") {
            const mode = asset.kind === "spritesheet" ? "spritesheet" : "media";
            void pluginHostRpc(pluginId, "pet.setAppearance", {
              mode,
              kind: asset.kind,
              source: asset.source,
            }).catch(() => {
              /* pet permission may be missing — asset still registered for diagnostics */
            });
            record.cleanups.push(() => {
              void pluginHostRpc(pluginId, "pet.clearAppearance", {}).catch(() => {});
            });
          }
        }
        return ok;
      },
      unregister(key) {
        unregisterAsset(key, pluginId);
        if (key === "pet.stage.skin" || key === "pet.stage.atlas") {
          void pluginHostRpc(pluginId, "pet.clearAppearance", {}).catch(() => {});
        }
      },
    },
    bus: {
      publish: (topic, payload) => publish(topic, payload),
      subscribe(topic, fn) {
        const off = subscribe(topic, pluginId, fn);
        record.cleanups.push(off);
        return off;
      },
    },
    i18n: createPluginI18n(),
    stores: {
      chat: useChatStore(pinia),
      setting: useSettingStore(pinia),
    },
    host: {
      rpc: (method, params) => pluginHostRpc(pluginId, method, params),
      on(event, fn) {
        const wrapped = (payload: Record<string, unknown>) => {
          if (payload.event === event && payload.pluginId === pluginId) fn(payload);
        };
        hostListeners.add(wrapped);
        const off = () => hostListeners.delete(wrapped);
        record.cleanups.push(off);
        return off;
      },
    },
    fs: {
      pick: (options) => pluginFsPick(pluginId, options),
    },
    onDeactivate(fn) {
      record.cleanups.push(fn);
    },
  };
}

export async function activatePlugin(
  pluginId: string,
  source: string,
  app: App,
  pinia: Pinia,
): Promise<void> {
  await deactivatePlugin(pluginId);
  const { mod, blobUrl } = await importActivateModule(pluginId, source);
  const ctx = createPluginContext(pluginId, app, pinia);
  const record = loaded.get(pluginId);
  if (record && blobUrl) {
    record.blobUrl = blobUrl;
    record.cleanups.push(() => URL.revokeObjectURL(blobUrl));
  }
  try {
    await mod.activate?.(ctx);
  } catch (error) {
    usePluginsStore().recordError({
      pluginId,
      phase: "activate",
      message: error instanceof Error ? error.message : String(error),
    });
    throw error;
  }
  const after = loaded.get(pluginId);
  if (after) after.deactivate = mod.deactivate;
}

async function importActivateModule(
  _pluginId: string,
  source: string,
): Promise<{
  mod: {
    activate?: (ctx: PluginActivateContext) => void | Promise<void>;
    deactivate?: () => void;
  };
  blobUrl?: string;
}> {
  const blob = new Blob([source], { type: "text/javascript" });
  const blobUrl = URL.createObjectURL(blob);
  const mod = (await import(/* @vite-ignore */ blobUrl)) as {
    activate?: (ctx: PluginActivateContext) => void | Promise<void>;
    deactivate?: () => void;
  };
  return { mod, blobUrl };
}

export async function deactivatePlugin(pluginId: string): Promise<void> {
  const record = loaded.get(pluginId);
  if (!record) return;
  for (const fn of record.cleanups.splice(0)) {
    try {
      fn();
    } catch {
      /* ignore */
    }
  }
  try {
    record.deactivate?.();
  } catch (error) {
    usePluginsStore().recordError({
      pluginId,
      phase: "deactivate",
      message: error instanceof Error ? error.message : String(error),
    });
  }
  loaded.delete(pluginId);
  usePluginsStore().removePluginSurface(pluginId);
}

/** Load activate.js for every enabled plugin. */
export async function syncEnabledPluginUi(): Promise<void> {
  if (!bootApp || !bootPinia) return;
  const pluginsStore = usePluginsStore(bootPinia);
  await pluginsStore.refresh();
  if (pluginsStore.safeMode) return;
  const enabled = pluginsStore.plugins.filter((item) => item.enabled && item.hasUi);
  for (const plugin of enabled) {
    if (!plugin.apiSupported) {
      // Contract version this plugin targets isn't in Anya's supported list yet
      // still activated; flagged so the plugin panel can warn instead of failing hard.
      pluginsStore.recordError({
        pluginId: plugin.id,
        phase: "activate",
        message: `apiVersion \`${plugin.apiVersion}\` is not in Anya's supported contract versions`,
      });
    }
    try {
      const source = await getPluginUiSource(plugin.id);
      await activatePlugin(plugin.id, source, bootApp, bootPinia);
    } catch (error) {
      console.warn("plugin activate failed", plugin.id, error);
    }
  }
}

export async function afterPluginDisabled(id: string): Promise<void> {
  await deactivatePlugin(id);
  await usePluginsStore().refresh();
}
