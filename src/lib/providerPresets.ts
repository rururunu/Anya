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
 * Google's favicon service resolves a real favicon for any reachable domain.
 * Custom providers always use this (cached on disk) instead of a model-vendor logo.
 */
export function faviconUrlForBaseUrl(baseUrl: string, size = 64): string | null {
  const hostname = hostnameFromBaseUrl(baseUrl);
  if (!hostname) return null;
  return `https://www.google.com/s2/favicons?sz=${size}&domain=${encodeURIComponent(hostname)}`;
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
