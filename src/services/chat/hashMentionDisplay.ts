/**
 * Display helpers for `#skill:` / `#mcp:` / `#plugin:` chips.
 * Wire tokens use install ids (`gmail`, legacy `sm-gmail`); UI shows title / vendor.
 */

import { peekInstallIcon } from "@/services/iconCache";
import { pluginIconUrl } from "@/services/plugins/ipc";
import type { McpServerConfig } from "@/types/setting";

/** Strip Smithery install-id prefixes for fallback labels. */
export function prettyHashInstallId(id: string): string {
  const trimmed = id.trim();
  if (trimmed.startsWith("sm-")) return trimmed.slice(3);
  if (trimmed.startsWith("smid-")) return trimmed.slice(5);
  return trimmed;
}

export function mcpMentionLabel(
  id: string,
  servers: readonly Pick<McpServerConfig, "id" | "title" | "qualifiedName">[],
): string {
  const server = servers.find((item) => item.id === id);
  return server?.title?.trim() || server?.qualifiedName?.trim() || prettyHashInstallId(id) || id;
}

export function mcpMentionIconUrl(
  id: string,
  servers: readonly Pick<McpServerConfig, "id" | "iconUrl">[],
): string | null {
  const peeked = peekInstallIcon("mcp", id);
  if (peeked) return peeked;
  const server = servers.find((item) => item.id === id);
  return server?.iconUrl?.trim() || null;
}

export function skillMentionLabel(
  id: string,
  skills?: readonly { name: string; title?: string; qualifiedName?: string | null }[],
): string {
  const skill = skills?.find((item) => item.name === id);
  return skill?.title?.trim() || skill?.qualifiedName?.trim() || prettyHashInstallId(id) || id;
}

export function skillMentionIconUrl(
  id: string,
  skills?: readonly { name: string; iconUrl?: string | null }[],
): string | null {
  const peeked = peekInstallIcon("skill", id);
  if (peeked) return peeked;
  const skill = skills?.find((item) => item.name === id);
  return skill?.iconUrl?.trim() || null;
}

export function pluginMentionLabel(
  id: string,
  plugins?: readonly { id: string; name?: string }[],
): string {
  const plugin = plugins?.find((item) => item.id === id);
  return plugin?.name?.trim() || prettyHashInstallId(id) || id;
}

export function pluginMentionIconUrl(
  id: string,
  plugins?: readonly { id: string; icon?: string | null }[],
): string | null {
  const plugin = plugins?.find((item) => item.id === id);
  const rel = plugin?.icon?.trim();
  return rel ? pluginIconUrl(id, rel) : null;
}
