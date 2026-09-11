import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type PluginContributes = {
  sidebar?: boolean;
  view?: boolean;
  composer?: boolean;
  window?: boolean;
  agent?: { tools?: boolean; hooks?: string[]; prompt?: string };
};

export type PluginCapabilityDecl = {
  id: string;
  scope: Record<string, unknown>;
};

export type UserPluginSummary = {
  id: string;
  name: string;
  version: string;
  apiVersion: string;
  apiSupported: boolean;
  /** Relative path inside the plugin to an svg/png/jpg/webp/gif; see `pluginIconUrl`. */
  icon?: string | null;
  /** Relative path to a `.md`/`.html` file rendered as the home page's "详情/About" tab. */
  about?: string | null;
  description: string;
  path: string;
  enabled: boolean;
  permissions: string[];
  capabilities: PluginCapabilityDecl[];
  granted: string[];
  hasUi: boolean;
  hasHost: boolean;
  /** `ui` | `service` | `agent` — what the plugin is for, not which chrome it happens to use. */
  role: string;
  contributes: PluginContributes;
  /** Bundled with Anya; cannot be uninstalled. */
  official: boolean;
};

export function listUserPlugins(): Promise<UserPluginSummary[]> {
  return invoke("list_user_plugins");
}

/** Builds the `anya-plugin://` URL for a file inside a plugin's own folder (icons, etc). */
export function pluginIconUrl(pluginId: string, relativePath: string): string {
  const rel = relativePath.replace(/^\/+/, "");
  return `anya-plugin://localhost/${pluginId}/${rel}`;
}

/**
 * WebView2 serves plugin files as `http://anya-plugin.localhost/...`.
 * `<video>` / lottie need that form; `anya-plugin://` only reliably loads images.
 */
export function pluginAssetUrl(source: string): string {
  const trimmed = source.trim();
  const match = trimmed.match(/^anya-plugin:\/\/localhost\/(.+)$/i);
  if (match) return `http://anya-plugin.localhost/${match[1]}`;
  return trimmed;
}

export function pluginPermissionCatalog(): Promise<[string, string][]> {
  return invoke("plugin_permission_catalog");
}

export function pluginCapabilityCatalog(): Promise<[string, string][]> {
  return invoke("plugin_capability_catalog");
}

/** Bridges a runtime fault to Rust so agent tool calls (`manage_plugin errors`) can see it. */
export function reportPluginRuntimeError(
  pluginId: string,
  phase: string,
  message: string,
): Promise<void> {
  return invoke("report_plugin_runtime_error", { pluginId, phase, message });
}

/** Drop mirrored Rust records so `manage_plugin errors` matches the diagnostics panel. */
export function clearPluginRuntimeErrors(pluginId?: string): Promise<void> {
  return invoke("clear_plugin_runtime_errors", { pluginId: pluginId ?? null });
}

export function listenPluginReloadRequests(
  handler: (pluginId: string) => void,
): Promise<UnlistenFn> {
  return listen<{ pluginId: string }>("plugin-reload-request", (event) => {
    const id = event.payload?.pluginId;
    if (id) handler(id);
  });
}

export function enableUserPlugin(id: string, permissions: string[]): Promise<void> {
  return invoke("enable_user_plugin", { id, permissions });
}

export function disableUserPlugin(id: string): Promise<void> {
  return invoke("disable_user_plugin", { id });
}

export function reloadUserPlugin(id: string): Promise<void> {
  return invoke("reload_user_plugin", { id });
}

export function exportUserPlugin(id: string, dest: string): Promise<string> {
  return invoke("export_user_plugin", { id, dest });
}

export function importUserPlugin(path: string, overwrite = false): Promise<string> {
  return invoke("import_user_plugin", { path, overwrite });
}

export function deleteUserPlugin(id: string): Promise<void> {
  return invoke("delete_user_plugin", { id });
}

export function openUserPlugin(id: string): Promise<void> {
  return invoke("open_user_plugin", { id });
}

export function closeUserPluginWindow(id: string): Promise<void> {
  return invoke("close_user_plugin_window", { id });
}

export function closeAllUserPluginWindows(): Promise<void> {
  return invoke("close_all_user_plugin_windows");
}

export function openPluginsDir(): Promise<void> {
  return invoke("open_plugins_dir");
}

export function getAnyaUserDir(): Promise<string> {
  return invoke("get_anya_user_dir");
}

export function getPluginUiSource(id: string): Promise<string> {
  return invoke("get_plugin_ui_source", { id });
}

export type PluginSafeModeStatus = {
  active: boolean;
  reason: string;
};

export function pluginSafeModeStatus(): Promise<PluginSafeModeStatus> {
  return invoke("plugin_safe_mode_status");
}

export function clearPluginSafeMode(): Promise<void> {
  return invoke("clear_plugin_safe_mode");
}

export type PluginPickedFile = {
  path: string;
  name: string;
  url: string;
};

export type PluginFsPickOptions = {
  multiple?: boolean;
  directory?: boolean;
  filters?: Array<{ name: string; extensions: string[] }>;
};

/** Native file/folder dialog. Requires the `fs.pick` permission. Cancel → `null`. */
export function pluginFsPick(
  pluginId: string,
  options: PluginFsPickOptions = {},
): Promise<PluginPickedFile[] | null> {
  return invoke("plugin_fs_pick", { pluginId, options });
}

export function pluginHostRpc(
  id: string,
  method: string,
  params: Record<string, unknown> = {},
): Promise<unknown> {
  return invoke("plugin_host_rpc", { id, method, params });
}

export function listenPluginHostEvents(
  handler: (payload: Record<string, unknown>) => void,
): Promise<UnlistenFn> {
  return listen<Record<string, unknown>>("plugin-host-event", (event) => {
    handler(event.payload ?? {});
  });
}
