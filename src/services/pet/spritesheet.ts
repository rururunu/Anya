/** ChatGPT/Codex-compatible pet spritesheet contract and helpers. */

export const SPRITESHEET_FRAME_WIDTH = 192;
export const SPRITESHEET_FRAME_HEIGHT = 208;
export const SPRITESHEET_COLUMNS = 8;
export const SPRITESHEET_ROWS = 9;
export const SPRITESHEET_WIDTH = SPRITESHEET_FRAME_WIDTH * SPRITESHEET_COLUMNS; // 1536
export const SPRITESHEET_HEIGHT = SPRITESHEET_FRAME_HEIGHT * SPRITESHEET_ROWS; // 1872

export type PetSpriteAnimationId =
  "idle" | "runRight" | "runLeft" | "wave" | "jump" | "fail" | "wait" | "work" | "review";

export type PetSpriteAnimationDef = {
  row: number;
  frames: number;
  durationsMs?: number[];
  loop?: boolean;
};

export type PetSpritesheetManifest = {
  id: string;
  displayName?: string | { en?: string; "zh-CN"?: string };
  spritesheet: string;
  frameWidth?: number;
  frameHeight?: number;
  columns?: number;
  rows?: number;
  animations?: Partial<Record<PetSpriteAnimationId, PetSpriteAnimationDef>>;
};

export const DEFAULT_ANIMATIONS: Record<PetSpriteAnimationId, PetSpriteAnimationDef> = {
  idle: { row: 0, frames: 6, durationsMs: [280, 110, 110, 140, 140, 320], loop: true },
  runRight: {
    row: 1,
    frames: 8,
    durationsMs: [120, 120, 120, 120, 120, 120, 120, 220],
    loop: true,
  },
  runLeft: { row: 2, frames: 8, durationsMs: [120, 120, 120, 120, 120, 120, 120, 220], loop: true },
  wave: { row: 3, frames: 4, durationsMs: [140, 140, 140, 280], loop: false },
  jump: { row: 4, frames: 5, durationsMs: [140, 140, 140, 140, 280], loop: false },
  fail: { row: 5, frames: 8, durationsMs: [140, 140, 140, 140, 140, 140, 140, 240], loop: true },
  wait: { row: 6, frames: 6, durationsMs: [180, 180, 180, 180, 180, 280], loop: true },
  work: { row: 7, frames: 6, durationsMs: [140, 140, 140, 140, 140, 220], loop: true },
  review: { row: 8, frames: 6, durationsMs: [180, 180, 180, 180, 180, 280], loop: true },
};

const ANIMATION_IDS = Object.keys(DEFAULT_ANIMATIONS) as PetSpriteAnimationId[];

/** Resolve a loaded JSON blob into a full manifest with defaults filled in. */
export function normalizeSpritesheetManifest(raw: unknown): PetSpritesheetManifest | null {
  if (!raw || typeof raw !== "object") return null;
  const value = raw as Record<string, unknown>;
  const id = typeof value.id === "string" ? value.id.trim() : "";
  const spritesheet =
    typeof value.spritesheet === "string"
      ? value.spritesheet.trim()
      : typeof value.spritesheetPath === "string"
        ? value.spritesheetPath.trim()
        : "";
  if (!id || !spritesheet) return null;

  const frameWidth =
    typeof value.frameWidth === "number" && value.frameWidth > 0
      ? value.frameWidth
      : SPRITESHEET_FRAME_WIDTH;
  const frameHeight =
    typeof value.frameHeight === "number" && value.frameHeight > 0
      ? value.frameHeight
      : SPRITESHEET_FRAME_HEIGHT;
  const columns =
    typeof value.columns === "number" && value.columns > 0 ? value.columns : SPRITESHEET_COLUMNS;
  const rows = typeof value.rows === "number" && value.rows > 0 ? value.rows : SPRITESHEET_ROWS;

  const animations: Partial<Record<PetSpriteAnimationId, PetSpriteAnimationDef>> = {
    ...DEFAULT_ANIMATIONS,
  };
  if (value.animations && typeof value.animations === "object") {
    const incoming = value.animations as Record<string, unknown>;
    for (const key of ANIMATION_IDS) {
      const item = incoming[key];
      if (!item || typeof item !== "object") continue;
      const def = item as Record<string, unknown>;
      const base = DEFAULT_ANIMATIONS[key];
      const row = typeof def.row === "number" ? def.row : base.row;
      const frames = typeof def.frames === "number" && def.frames > 0 ? def.frames : base.frames;
      const durationsMs = Array.isArray(def.durationsMs)
        ? def.durationsMs.filter((n): n is number => typeof n === "number" && n > 0)
        : base.durationsMs;
      const loop = typeof def.loop === "boolean" ? def.loop : base.loop;
      animations[key] = { row, frames, durationsMs, loop };
    }
  }

  return {
    id,
    displayName:
      typeof value.displayName === "string" ||
      (value.displayName && typeof value.displayName === "object")
        ? (value.displayName as PetSpritesheetManifest["displayName"])
        : undefined,
    spritesheet,
    frameWidth,
    frameHeight,
    columns,
    rows,
    animations,
  };
}

export function resolveAnimation(
  manifest: PetSpritesheetManifest,
  id: PetSpriteAnimationId,
): PetSpriteAnimationDef {
  return manifest.animations?.[id] ?? DEFAULT_ANIMATIONS[id];
}

/** Map Anya expression / transient gesture onto a spritesheet animation. */
export function expressionToSpriteAnimation(
  expression: string | undefined,
  gesture?: PetSpriteAnimationId | null,
): PetSpriteAnimationId {
  if (gesture) return gesture;
  switch (expression) {
    case "thinking":
    case "talking":
      return "review";
    case "working":
      return "work";
    case "waiting":
      return "wait";
    case "done":
      return "jump";
    case "error":
      return "fail";
    case "sleeping":
    case "idle":
    default:
      return "idle";
  }
}

export function frameSourceRect(
  manifest: PetSpritesheetManifest,
  animation: PetSpriteAnimationDef,
  frameIndex: number,
): { sx: number; sy: number; sw: number; sh: number } {
  const fw = manifest.frameWidth ?? SPRITESHEET_FRAME_WIDTH;
  const fh = manifest.frameHeight ?? SPRITESHEET_FRAME_HEIGHT;
  const columns = manifest.columns ?? SPRITESHEET_COLUMNS;
  const frame = ((frameIndex % animation.frames) + animation.frames) % animation.frames;
  const col = frame % columns;
  const row = animation.row;
  return { sx: col * fw, sy: row * fh, sw: fw, sh: fh };
}

export function frameDurationMs(animation: PetSpriteAnimationDef, frameIndex: number): number {
  const list = animation.durationsMs;
  if (list && list.length > 0) {
    return list[frameIndex % list.length] ?? list[0] ?? 160;
  }
  return 160;
}

/** Resolve spritesheet image URL relative to a pet.json URL. */
export function resolveSpritesheetUrl(manifestUrl: string, spritesheetRel: string): string {
  if (
    spritesheetRel.startsWith("http://") ||
    spritesheetRel.startsWith("https://") ||
    spritesheetRel.startsWith("data:") ||
    spritesheetRel.startsWith("anya-plugin:") ||
    spritesheetRel.startsWith("blob:")
  ) {
    return spritesheetRel;
  }
  try {
    const base = new URL(manifestUrl);
    const dir = base.pathname.replace(/[^/]+$/, "");
    base.pathname = `${dir}${spritesheetRel.replace(/^\//, "")}`;
    return base.toString();
  } catch {
    const trimmed = manifestUrl.replace(/[^/]+$/, "");
    return `${trimmed}${spritesheetRel.replace(/^\//, "")}`;
  }
}
