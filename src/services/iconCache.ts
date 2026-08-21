/**
 * Install-scoped icon cache.
 * Browse catalogs use remote URLs directly; only installs call these helpers.
 */

import { convertFileSrc, invoke } from "@tauri-apps/api/core";

export type IconInstallKind = "mcp" | "skill" | "provider";

export type IconWarmEntry = {
  kind: IconInstallKind;
  cacheKey: string;
  /** Remote URL used to backfill disk cache when missing. */
  url?: string | null;
};

/** kind+key → local asset URL */
const memory = new Map<string, string>();
const inflight = new Map<string, Promise<string | null>>();

function memKey(kind: IconInstallKind, cacheKey: string) {
  return `${kind}:${cacheKey}`;
}

export function peekInstallIcon(kind: IconInstallKind, cacheKey: string): string | null {
  return memory.get(memKey(kind, cacheKey)) ?? null;
}

function rememberPath(kind: IconInstallKind, cacheKey: string, path: string): string {
  const local = convertFileSrc(path);
  memory.set(memKey(kind, cacheKey), local);
  return local;
}

/** Disk lookup only (no network). */
export async function lookupInstallIcon(
  kind: IconInstallKind,
  cacheKey: string,
): Promise<string | null> {
  const key = memKey(kind, cacheKey);
  const hit = memory.get(key);
  if (hit) return hit;
  try {
    const path = await invoke<string | null>("lookup_install_icon", { kind, cacheKey });
    if (!path) return null;
    return rememberPath(kind, cacheKey, path);
  } catch {
    return null;
  }
}

/**
 * Batch-warm install icons into memory (one IPC). Missing disk entries with a
 * remote URL are downloaded in the background.
 */
export async function warmInstallIcons(entries: IconWarmEntry[]): Promise<void> {
  const unique = new Map<string, IconWarmEntry>();
  for (const entry of entries) {
    const cacheKey = entry.cacheKey.trim();
    if (!cacheKey) continue;
    unique.set(memKey(entry.kind, cacheKey), { ...entry, cacheKey });
  }
  if (unique.size === 0) return;

  const missing = [...unique.values()].filter(
    (entry) => !memory.has(memKey(entry.kind, entry.cacheKey)),
  );
  if (missing.length === 0) return;

  try {
    const found = await invoke<Record<string, string>>("lookup_install_icons", {
      entries: missing.map((entry) => ({
        kind: entry.kind,
        cacheKey: entry.cacheKey,
      })),
    });
    for (const [key, path] of Object.entries(found ?? {})) {
      const sep = key.indexOf(":");
      if (sep <= 0 || !path) continue;
      const kind = key.slice(0, sep) as IconInstallKind;
      const cacheKey = key.slice(sep + 1);
      if (kind !== "mcp" && kind !== "skill" && kind !== "provider") continue;
      rememberPath(kind, cacheKey, path);
    }
  } catch {
    // fall through to per-item / remote backfill
  }

  for (const entry of missing) {
    const key = memKey(entry.kind, entry.cacheKey);
    if (memory.has(key)) continue;
    const remote = entry.url?.trim();
    if (!remote || !/^https?:\/\//i.test(remote)) continue;
    void cacheInstallIcon(entry.kind, entry.cacheKey, remote);
  }
}

/**
 * After install: if not on disk, download `url` and store under the install identity.
 * Returns local asset URL, or null on failure.
 */
export async function cacheInstallIcon(
  kind: IconInstallKind,
  cacheKey: string,
  url: string,
): Promise<string | null> {
  const remote = url.trim();
  if (!remote || !/^https?:\/\//i.test(remote)) return lookupInstallIcon(kind, cacheKey);

  const key = memKey(kind, cacheKey);
  const pending = inflight.get(key);
  if (pending) return pending;

  const task = (async (): Promise<string | null> => {
    try {
      const path = await invoke<string>("cache_install_icon", {
        kind,
        cacheKey,
        url: remote,
      });
      return rememberPath(kind, cacheKey, path);
    } catch {
      return null;
    } finally {
      inflight.delete(key);
    }
  })();

  inflight.set(key, task);
  return task;
}

export async function clearInstallIcon(kind: IconInstallKind, cacheKey: string): Promise<void> {
  memory.delete(memKey(kind, cacheKey));
  try {
    await invoke("clear_install_icon", { kind, cacheKey });
  } catch {
    // best-effort
  }
}
