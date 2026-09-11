import { reactive } from "vue";

export type PluginMount = (el: HTMLElement) => void | (() => void);

/** How multiple plugins mounting the same anchor are reconciled. */
export type SlotMultiplicity = "exclusive" | "stack" | "user-choice";

/** Where a sidebar tab's launcher is drawn. The plugin chooses; Anya does not fan out. */
export const TAB_CHROME = ["nav", "header", "views"] as const;
export type PluginChromeSurface = (typeof TAB_CHROME)[number];

export type PluginTabClick = () => void | Promise<void>;

export type SlotEntry = {
  id: string;
  pluginId: string;
  title?: string;
  icon?: string;
  mount: PluginMount;
  /** Launcher chrome. Empty = no button. Omitted at mount defaults to `["views"]`. */
  chrome?: PluginChromeSurface[];
  /** Per-surface content override; a surface not listed here falls back to `mount`. */
  content?: Partial<Record<PluginChromeSurface, PluginMount>>;
  /** Header-only tabs run this instead of opening a pane. */
  onClick?: PluginTabClick;
};

export type AnchorDescriptor = {
  id: string;
  /** Surfaces (workbench/peek/pet/settings) this anchor is meaningful on. */
  surfaces: string[];
  multiplicity: SlotMultiplicity;
};

/**
 * Named mount points that plugins target by id instead of a bespoke API per
 * feature. Anya declares anchors once; new anchors are data, not new SDK
 * methods. Backs `sidebar.addTab` / `workbench.setView` / `composer.addAccessory`
 * so all surfaces read from the same source of truth (chrome.v1 contract).
 * Host chrome renders holes with `PluginSlotOutlet`; do not add per-anchor Vue files.
 */
const ANCHORS: Record<string, AnchorDescriptor> = {
  "sidebar.tabs": { id: "sidebar.tabs", surfaces: ["workbench", "peek"], multiplicity: "stack" },
  "workbench.main": { id: "workbench.main", surfaces: ["workbench"], multiplicity: "exclusive" },
  "composer.accessory": {
    id: "composer.accessory",
    surfaces: ["workbench"],
    multiplicity: "stack",
  },
  "conversation.materials": {
    id: "conversation.materials",
    surfaces: ["workbench"],
    multiplicity: "stack",
  },
};

export type SlotConflict = { anchorId: string; ownerPluginId: string; rejectedPluginId: string };

const entries = reactive(new Map<string, SlotEntry[]>());
const conflicts = reactive<SlotConflict[]>([]);

export function listAnchors(): AnchorDescriptor[] {
  return Object.values(ANCHORS);
}

export function getAnchor(anchorId: string): AnchorDescriptor | undefined {
  return ANCHORS[anchorId];
}

export function getSlotEntries(anchorId: string): SlotEntry[] {
  return entries.get(anchorId) ?? [];
}

export function getSlotConflicts(): SlotConflict[] {
  return conflicts;
}

/**
 * `undefined` → `["views"]` (one launcher, where the pane actually lives).
 * `[]` → no launcher. Unknown ids are dropped.
 */
export function normalizeTabChrome(chrome?: readonly string[] | null): PluginChromeSurface[] {
  if (chrome == null) return ["views"];
  const allowed = new Set<string>(TAB_CHROME);
  const out: PluginChromeSurface[] = [];
  for (const item of chrome) {
    if (allowed.has(item) && !out.includes(item as PluginChromeSurface)) {
      out.push(item as PluginChromeSurface);
    }
  }
  return out;
}

export function tabShowsOn(
  chrome: readonly PluginChromeSurface[] | undefined,
  surface: PluginChromeSurface,
): boolean {
  return (chrome ?? ["views"]).includes(surface);
}

/**
 * Left-nav launcher with no `"views"` entry: same center region as 插件 /
 * Connect phone (replaces the chat). `"nav"+"views"` still opens the right
 * review pane — nav is only a shortcut then, not 主视图.
 */
export function tabOpensMainPane(chrome: readonly PluginChromeSurface[] | undefined): boolean {
  return tabShowsOn(chrome, "nav") && !tabShowsOn(chrome, "views");
}

/**
 * Header icon with no pane: click runs `onClick` and must not open 主视图
 * or the review strip. `"header"` plus `nav`/`views` still opens a pane.
 */
export function tabIsChromeAction(chrome: readonly PluginChromeSurface[] | undefined): boolean {
  return tabShowsOn(chrome, "header") && !tabShowsOn(chrome, "nav") && !tabShowsOn(chrome, "views");
}

/**
 * Content for one surface of a multi-surface tab. A plugin can mount a
 * compact widget in `header`/`nav` chrome and a full pane for `views` by
 * passing `content`; surfaces not listed there just reuse `mount`.
 */
export function contentForSurface(
  entry: { mount: PluginMount; content?: Partial<Record<PluginChromeSurface, PluginMount>> },
  surface: PluginChromeSurface,
): PluginMount {
  return entry.content?.[surface] ?? entry.mount;
}

/**
 * Mounts `entry` into `anchorId`, applying the anchor's multiplicity rule.
 * Returns false (and records a conflict) if an `exclusive` anchor is already
 * occupied by a different plugin.
 */
export function mountSlot(anchorId: string, entry: SlotEntry): boolean {
  const anchor = ANCHORS[anchorId];
  if (!anchor) throw new Error(`unknown slot anchor \`${anchorId}\``);
  const list = entries.get(anchorId) ?? [];
  const withoutSelf = list.filter((item) => item.id !== entry.id);
  if (
    anchor.multiplicity === "exclusive" &&
    withoutSelf.some((item) => item.pluginId !== entry.pluginId)
  ) {
    const owner = withoutSelf[0];
    conflicts.push({ anchorId, ownerPluginId: owner.pluginId, rejectedPluginId: entry.pluginId });
    return false;
  }
  entries.set(anchorId, [...withoutSelf, { ...entry, chrome: normalizeTabChrome(entry.chrome) }]);
  return true;
}

export function unmountSlot(anchorId: string, id: string): void {
  const list = entries.get(anchorId);
  if (!list) return;
  entries.set(
    anchorId,
    list.filter((item) => item.id !== id),
  );
}

export function unmountPlugin(anchorId: string, pluginId: string): void {
  const list = entries.get(anchorId);
  if (!list) return;
  entries.set(
    anchorId,
    list.filter((item) => item.pluginId !== pluginId),
  );
}

export function unmountPluginFromAllAnchors(pluginId: string): void {
  for (const anchorId of Object.keys(ANCHORS)) unmountPlugin(anchorId, pluginId);
  const remaining = conflicts.filter(
    (c) => c.ownerPluginId !== pluginId && c.rejectedPluginId !== pluginId,
  );
  conflicts.splice(0, conflicts.length, ...remaining);
}
