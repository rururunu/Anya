/**
 * Local usage habits for Skills and MCP servers.
 * Used to rank `#` mention suggestions and settings lists.
 */

export type ResourceKind = "skill" | "mcp";

export type ResourceUsageEntry = {
  count: number;
  /** Unix ms of last use. */
  lastUsedAt: number;
};

export type ResourceUsageStore = {
  skill: Record<string, ResourceUsageEntry>;
  mcp: Record<string, ResourceUsageEntry>;
};

const STORAGE_KEY = "anya.resourceUsage.v1";

/** Built-in skill tools whose name equals the skill id (or maps to one). */
const SKILL_TOOL_IDS = new Set([
  "explore",
  "research",
  "review",
  "security_review",
  "review_security",
  "generate_word",
  "docx",
  "pandoc",
]);

const SKILL_TOOL_ALIASES: Record<string, string> = {
  review_security: "security_review",
};

function emptyStore(): ResourceUsageStore {
  return { skill: {}, mcp: {} };
}

function canUseStorage(): boolean {
  return typeof localStorage !== "undefined";
}

export function loadResourceUsage(): ResourceUsageStore {
  if (!canUseStorage()) return emptyStore();
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return emptyStore();
    const parsed = JSON.parse(raw) as Partial<ResourceUsageStore>;
    return {
      skill: sanitizeMap(parsed.skill),
      mcp: sanitizeMap(parsed.mcp),
    };
  } catch {
    return emptyStore();
  }
}

function sanitizeMap(value: unknown): Record<string, ResourceUsageEntry> {
  if (!value || typeof value !== "object") return {};
  const out: Record<string, ResourceUsageEntry> = {};
  for (const [key, entry] of Object.entries(value as Record<string, unknown>)) {
    const id = key.trim();
    if (!id || !entry || typeof entry !== "object") continue;
    const count = Number((entry as ResourceUsageEntry).count);
    const lastUsedAt = Number((entry as ResourceUsageEntry).lastUsedAt);
    if (!Number.isFinite(count) || count <= 0) continue;
    out[id] = {
      count: Math.floor(count),
      lastUsedAt: Number.isFinite(lastUsedAt) ? Math.floor(lastUsedAt) : 0,
    };
  }
  return out;
}

export function saveResourceUsage(store: ResourceUsageStore): void {
  if (!canUseStorage()) return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(store));
  } catch {
    // Quota / private mode — ignore.
  }
}

export function recordResourceUsage(kind: ResourceKind, id: string, at = Date.now()): void {
  const cleaned = id.trim();
  if (!cleaned) return;
  const store = loadResourceUsage();
  const bucket = store[kind];
  const prev = bucket[cleaned];
  bucket[cleaned] = {
    count: (prev?.count ?? 0) + 1,
    lastUsedAt: at,
  };
  saveResourceUsage(store);
}

export function recordResourceUsages(
  items: ReadonlyArray<{ kind: ResourceKind; id: string }>,
  at = Date.now(),
): void {
  if (items.length === 0) return;
  const store = loadResourceUsage();
  let changed = false;
  for (const item of items) {
    const id = item.id.trim();
    if (!id) continue;
    const bucket = store[item.kind];
    const prev = bucket[id];
    bucket[id] = {
      count: (prev?.count ?? 0) + 1,
      lastUsedAt: at,
    };
    changed = true;
  }
  if (changed) saveResourceUsage(store);
}

/**
 * Score for ranking: prefer higher frequency, then more recent.
 * Recency adds a small boost so a rarely used but just-used item can surface.
 */
export function resourceUsageScore(
  kind: ResourceKind,
  id: string,
  store: ResourceUsageStore = loadResourceUsage(),
  now = Date.now(),
): number {
  const entry = store[kind][id.trim()];
  if (!entry) return 0;
  const ageMs = Math.max(0, now - entry.lastUsedAt);
  const dayMs = 86_400_000;
  const recencyBoost = Math.max(0, 14 - ageMs / dayMs);
  return entry.count * 10 + recencyBoost;
}

export function compareByResourceUsage(
  kind: ResourceKind,
  leftId: string,
  rightId: string,
  store: ResourceUsageStore = loadResourceUsage(),
  now = Date.now(),
): number {
  const left = resourceUsageScore(kind, leftId, store, now);
  const right = resourceUsageScore(kind, rightId, store, now);
  if (left !== right) return right - left;
  return leftId.localeCompare(rightId);
}

export function sortByResourceUsage<T>(
  items: readonly T[],
  kind: ResourceKind,
  idOf: (item: T) => string,
  store: ResourceUsageStore = loadResourceUsage(),
  now = Date.now(),
): T[] {
  return [...items].sort((left, right) => {
    const byUsage = compareByResourceUsage(kind, idOf(left), idOf(right), store, now);
    if (byUsage !== 0) return byUsage;
    return idOf(left).localeCompare(idOf(right));
  });
}

/** Map a tool activity to a skill/MCP usage event when applicable. */
export function resourceFromToolActivity(
  toolName: string,
  args?: Record<string, unknown> | null,
): { kind: ResourceKind; id: string } | null {
  const name = toolName.trim();
  if (!name) return null;

  if (name.startsWith("mcp__")) {
    const rest = name.slice("mcp__".length);
    const sep = rest.indexOf("__");
    const id = (sep >= 0 ? rest.slice(0, sep) : rest).trim();
    return id ? { kind: "mcp", id } : null;
  }

  if (name === "load_skill" || name === "run_skill") {
    const skill =
      typeof args?.name === "string"
        ? args.name.trim()
        : typeof args?.skill === "string"
          ? args.skill.trim()
          : "";
    return skill ? { kind: "skill", id: skill } : null;
  }

  if (SKILL_TOOL_IDS.has(name)) {
    return { kind: "skill", id: SKILL_TOOL_ALIASES[name] ?? name };
  }

  return null;
}

export function recordToolActivityUsage(
  toolName: string,
  args?: Record<string, unknown> | null,
): void {
  const hit = resourceFromToolActivity(toolName, args);
  if (hit) recordResourceUsage(hit.kind, hit.id);
}
