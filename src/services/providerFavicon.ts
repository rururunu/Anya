/**
 * Domain favicons for custom model providers.
 *
 * Icons are keyed by hostname on disk so the same API host is only downloaded
 * once, then mapped in memory to each provider id for the UI.
 */

import { reactive } from "vue";
import { faviconUrlForBaseUrl, hostnameFromBaseUrl } from "@/lib/providerPresets";
import { cacheInstallIcon, lookupInstallIcon, peekInstallIcon } from "@/services/iconCache";

const srcById = reactive<Record<string, string | null>>({});
const hostnameById = new Map<string, string>();
const inflight = new Set<string>();

export function peekProviderFavicon(id: string | null | undefined): string | null {
  if (!id) return null;
  return srcById[id] ?? null;
}

export function markProviderFaviconBroken(id: string | null | undefined): void {
  if (!id) return;
  srcById[id] = null;
}

export function ensureProviderFavicon(id: string | null | undefined, baseUrl: string): void {
  if (!id) return;
  const hostname = hostnameFromBaseUrl(baseUrl);
  const remote = faviconUrlForBaseUrl(baseUrl);
  if (!hostname || !remote) return;

  if (hostnameById.get(id) === hostname && srcById[id]) return;
  hostnameById.set(id, hostname);

  const cached = peekInstallIcon("provider", hostname);
  if (cached) {
    srcById[id] = cached;
    return;
  }

  const attemptKey = `${id}:${hostname}`;
  if (inflight.has(attemptKey)) return;
  inflight.add(attemptKey);

  void (async () => {
    try {
      const existing = await lookupInstallIcon("provider", hostname);
      if (existing) {
        srcById[id] = existing;
        return;
      }
      srcById[id] = await cacheInstallIcon("provider", hostname, remote);
    } catch {
      srcById[id] = null;
    }
  })();
}

export function warmProviderFavicons(providers: Array<{ id: string; baseUrl: string }>): void {
  for (const provider of providers) {
    ensureProviderFavicon(provider.id, provider.baseUrl);
  }
}
