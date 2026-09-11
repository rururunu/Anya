import { reactive } from "vue";

export type AssetKind = "image" | "video" | "lottie";

export type AssetOverride = {
  pluginId: string;
  kind: AssetKind;
  source: string;
};

export type AssetKeyConstraint = { allowedKinds: AssetKind[] };

export type AssetConflict = { key: string; ownerPluginId: string; rejectedPluginId: string };

/**
 * Resource keys Anya renders through a shared, controlled container (mascot,
 * tray icon, workbench backdrop...). Plugins register a key + source instead
 * of touching the DOM; Anya decides how each kind actually plays (loop,
 * pause when minimized, respect reduced motion). New "support video
 * background" style requests are just widening `allowedKinds` for a key, not
 * a new feature.
 */
const KEY_CONSTRAINTS: Record<string, AssetKeyConstraint> = {
  "mascot.idle": { allowedKinds: ["image", "video", "lottie"] },
  "tray.icon": { allowedKinds: ["image"] },
  "workbench.backdrop": { allowedKinds: ["image", "video", "lottie"] },
  "pet.stage.skin": { allowedKinds: ["image", "video", "lottie"] },
};

const overrides = reactive(new Map<string, AssetOverride>());
const conflicts = reactive<AssetConflict[]>([]);

export function listAssetKeys(): string[] {
  return Object.keys(KEY_CONSTRAINTS);
}

export function getAssetConstraint(key: string): AssetKeyConstraint | undefined {
  return KEY_CONSTRAINTS[key];
}

export function getAssetOverride(key: string): AssetOverride | undefined {
  return overrides.get(key);
}

export function getAssetConflicts(): AssetConflict[] {
  return conflicts;
}

/** Registers `asset` for `key`; false if another plugin already owns it. */
export function registerAsset(
  key: string,
  pluginId: string,
  asset: { kind: AssetKind; source: string },
): boolean {
  const constraint = KEY_CONSTRAINTS[key];
  if (!constraint) throw new Error(`unknown asset key \`${key}\``);
  if (!constraint.allowedKinds.includes(asset.kind)) {
    throw new Error(`asset key \`${key}\` does not allow kind \`${asset.kind}\``);
  }
  const current = overrides.get(key);
  if (current && current.pluginId !== pluginId) {
    conflicts.push({ key, ownerPluginId: current.pluginId, rejectedPluginId: pluginId });
    return false;
  }
  overrides.set(key, { pluginId, kind: asset.kind, source: asset.source });
  return true;
}

export function unregisterAsset(key: string, pluginId: string): void {
  const current = overrides.get(key);
  if (current && current.pluginId === pluginId) overrides.delete(key);
}

export function unregisterPluginAssets(pluginId: string): void {
  for (const [key, value] of overrides.entries()) {
    if (value.pluginId === pluginId) overrides.delete(key);
  }
  const remaining = conflicts.filter(
    (item) => item.ownerPluginId !== pluginId && item.rejectedPluginId !== pluginId,
  );
  conflicts.splice(0, conflicts.length, ...remaining);
}
