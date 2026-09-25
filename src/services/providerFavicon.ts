/**
 * Domain favicons for custom model providers.
 *
 * Icons are keyed by the selected website hostname on disk and mapped in
 * memory to each provider id for the UI.
 */

import { reactive } from "vue";
import { faviconUrlsForWebsite, hostnameFromBaseUrl } from "@/lib/providerPresets";
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

export function ensureProviderFavicon(
  id: string | null | undefined,
  baseUrl: string,
  websiteUrl?: string,
): void {
  if (!id) return;
  const sourceUrl = websiteUrl?.trim() || baseUrl;
  const hostname = hostnameFromBaseUrl(sourceUrl);
  const remotes = faviconUrlsForWebsite(sourceUrl);
  if (!hostname || remotes.length === 0) {
    hostnameById.delete(id);
    srcById[id] = null;
    return;
  }

  if (hostnameById.get(id) === hostname && srcById[id]) return;
  hostnameById.set(id, hostname);
  srcById[id] = null;

  const setCurrent = (src: string | null) => {
    if (hostnameById.get(id) === hostname) srcById[id] = src;
  };

  const cached = peekInstallIcon("provider", hostname);
  if (cached) {
    setCurrent(cached);
    return;
  }

  const attemptKey = `${id}:${hostname}`;
  if (inflight.has(attemptKey)) return;
  inflight.add(attemptKey);

  void (async () => {
    try {
      const existing = await lookupInstallIcon("provider", hostname);
      if (existing) {
        setCurrent(existing);
        return;
      }
      for (const remote of remotes) {
        const cached = await cacheInstallIcon("provider", hostname, remote);
        if (cached) {
          setCurrent(cached);
          return;
        }
      }
      setCurrent(null);
    } catch {
      setCurrent(null);
    } finally {
      inflight.delete(attemptKey);
    }
  })();
}

export function warmProviderFavicons(
  providers: Array<{ id: string; baseUrl: string; websiteUrl?: string }>,
): void {
  for (const provider of providers) {
    ensureProviderFavicon(provider.id, provider.baseUrl, provider.websiteUrl);
  }
}
