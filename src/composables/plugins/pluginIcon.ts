import { pluginIconUrl } from "@/services/plugins/ipc";

const ICON_URL_RE = /^(anya-plugin:|data:|https?:)/i;

export function isPluginIconUrl(value: string): boolean {
  return ICON_URL_RE.test(value.trim());
}

/**
 * Prefer a shipped image URL. A Lucide glyph (`terminal`) is kept only when
 * the plugin has no `plugin.json` icon file yet.
 */
export function resolveSlotIcon(
  pluginId: string,
  icon: string | undefined,
  manifestIcon?: string | null,
): string | undefined {
  const trimmed = icon?.trim();
  if (trimmed && isPluginIconUrl(trimmed)) return trimmed;
  const rel = manifestIcon?.trim();
  if (rel) return pluginIconUrl(pluginId, rel);
  if (trimmed) return trimmed;
  return pluginIconUrl(pluginId, "ui/icon.svg");
}

/** Icon for the plugins list / home header: manifest file, else a live tab glyph. */
export function displayPluginIcon(
  plugin: { id: string; icon?: string | null },
  tabIcon?: string,
): string | undefined {
  const rel = plugin.icon?.trim();
  if (rel) return pluginIconUrl(plugin.id, rel);
  const tab = tabIcon?.trim();
  if (tab) return tab;
  if (plugin.id === "terminal") return "terminal";
  return undefined;
}
