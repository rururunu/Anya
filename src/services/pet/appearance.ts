export type PetExpression =
  "idle" | "thinking" | "working" | "talking" | "waiting" | "done" | "error" | "sleeping";

export type PetAppearanceMode = "mascot" | "media" | "companion" | "spritesheet";

export type PetMediaKind = "image" | "video" | "lottie" | "svg" | "html" | "spritesheet";

export type PetCompanionConfig = {
  accent?: string;
  variant?: "orb" | "pill";
  glow?: boolean;
};

export type PetAppearance = {
  mode: PetAppearanceMode;
  kind?: PetMediaKind;
  source?: string;
  config?: PetCompanionConfig | Record<string, unknown>;
};

export const PET_MEDIA_KINDS = new Set<PetMediaKind>([
  "image",
  "video",
  "lottie",
  "svg",
  "html",
  "spritesheet",
]);

export const DEFAULT_PET_APPEARANCE: PetAppearance = { mode: "mascot" };

/** Normalize RPC / event payloads into a PetAppearance. */
export function normalizePetAppearance(raw: unknown): PetAppearance | null {
  if (!raw || typeof raw !== "object") return null;
  const value = raw as Record<string, unknown>;

  if ("mode" in value) {
    const mode = value.mode;
    if (mode === "mascot") return { mode: "mascot" };
    if (mode === "companion") {
      return {
        mode: "companion",
        config: normalizeCompanionConfig(value.config),
      };
    }
    if (mode === "spritesheet") {
      const source = value.source;
      if (typeof source === "string" && source.trim()) {
        return {
          mode: "spritesheet",
          kind: "spritesheet",
          source: source.trim(),
          config:
            value.config && typeof value.config === "object"
              ? (value.config as Record<string, unknown>)
              : undefined,
        };
      }
      return null;
    }
    if (mode === "media") {
      const kind = value.kind;
      const source = value.source;
      if (
        typeof kind === "string" &&
        PET_MEDIA_KINDS.has(kind as PetMediaKind) &&
        kind !== "spritesheet" &&
        typeof source === "string" &&
        source.trim()
      ) {
        return { mode: "media", kind: kind as PetMediaKind, source: source.trim() };
      }
      return null;
    }
    return null;
  }

  // Legacy `{ kind, source }` skin payloads.
  const kind = value.kind;
  const source = value.source;
  if (
    typeof kind === "string" &&
    PET_MEDIA_KINDS.has(kind as PetMediaKind) &&
    typeof source === "string" &&
    source.trim()
  ) {
    return { mode: "media", kind: kind as PetMediaKind, source: source.trim() };
  }
  return null;
}

function normalizeCompanionConfig(raw: unknown): PetCompanionConfig | undefined {
  if (!raw || typeof raw !== "object") return undefined;
  const value = raw as Record<string, unknown>;
  const config: PetCompanionConfig = {};
  if (typeof value.accent === "string" && value.accent.trim()) {
    config.accent = value.accent.trim();
  }
  if (value.variant === "orb" || value.variant === "pill") {
    config.variant = value.variant;
  }
  if (typeof value.glow === "boolean") {
    config.glow = value.glow;
  }
  return Object.keys(config).length > 0 ? config : undefined;
}

/** Map mascot / agent expressions onto companion animation classes. */
export function normalizePetExpression(expression: string | undefined): PetExpression {
  if (expression === "icon") return "idle";
  if (
    expression === "thinking" ||
    expression === "working" ||
    expression === "talking" ||
    expression === "waiting" ||
    expression === "done" ||
    expression === "error" ||
    expression === "sleeping"
  ) {
    return expression;
  }
  return "idle";
}

export function companionExpressionClass(expression: PetExpression): string {
  switch (expression) {
    case "thinking":
      return "companion-thinking";
    case "working":
      return "companion-working";
    case "talking":
      return "companion-talking";
    case "waiting":
      return "companion-waiting";
    case "done":
      return "companion-done";
    case "error":
      return "companion-error";
    case "sleeping":
      return "companion-sleeping";
    default:
      return "companion-idle";
  }
}
