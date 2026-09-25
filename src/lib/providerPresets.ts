import type { ProviderModelEntry } from "@/types/setting";

export function looksLikeHttpUrl(value: string): boolean {
  return /^https?:\/\//i.test(value.trim());
}

export function hostnameFromBaseUrl(baseUrl: string): string | null {
  const trimmed = baseUrl.trim();
  if (!looksLikeHttpUrl(trimmed)) return null;
  try {
    const { hostname } = new URL(trimmed);
    return hostname || null;
  } catch {
    return null;
  }
}

/**
 * Google's favicon service resolves an icon for a website hostname.
 * The result is cached on disk instead of using a model-vendor logo.
 */
export function faviconUrlForBaseUrl(baseUrl: string, size = 64): string | null {
  const hostname = hostnameFromBaseUrl(baseUrl);
  if (!hostname) return null;
  return `https://www.google.com/s2/favicons?sz=${size}&domain=${encodeURIComponent(hostname)}`;
}

export function faviconUrlsForWebsite(websiteUrl: string): string[] {
  const favicon = faviconUrlForBaseUrl(websiteUrl);
  if (!favicon) return [];
  try {
    const origin = new URL(websiteUrl.trim()).origin;
    return [favicon, `${origin}/favicon.ico`];
  } catch {
    return [];
  }
}

export function isCustomProviderConfigured(provider: { baseUrl: string; apiKey: string }): boolean {
  return looksLikeHttpUrl(provider.baseUrl) && provider.apiKey.trim().length > 0;
}

export function parseProviderModels(raw: string): string[] {
  const seen = new Set<string>();
  const models: string[] = [];
  for (const part of raw.split(/[,，\n]/)) {
    const id = part.trim();
    if (!id || seen.has(id)) continue;
    seen.add(id);
    models.push(id);
  }
  return models;
}

export function serializeProviderModels(models: string[]): string {
  return models.join("\n");
}

/** 将线上拉取的模型列表合并进当前条目：保留自定义条目，丢弃消失的非自定义条目，新远程模型默认启用。 */
export function syncRemoteModels(
  entries: ProviderModelEntry[],
  remoteIds: string[],
): ProviderModelEntry[] {
  const remoteSet = new Set(remoteIds);
  const kept = entries.filter((entry) => entry.custom || remoteSet.has(entry.id));
  const keptIds = new Set(kept.map((entry) => entry.id));
  const added = remoteIds
    .filter((id) => !keptIds.has(id))
    .map((id) => ({ id, disabled: false, custom: false }));
  return [...kept, ...added];
}
