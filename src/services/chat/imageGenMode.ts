import { parseProviderModels } from "@/lib/providerPresets";
import type { ImageStyleTemplate } from "@/types/setting";

export type ImageGenRatio =
  "auto" | "21:9" | "16:9" | "3:2" | "4:3" | "1:1" | "3:4" | "2:3" | "9:16";
export type ImageGenResolution = "1.5k" | "2k" | "4k";
export type ImageGenCount = 1 | 2 | 3 | 4;
export type ImageGenFieldId = "ratio" | "resolution" | "style" | "count" | "model";

export type ImageGenFieldOption = {
  id: string;
  labelKey?: string;
  label?: string;
  labelParams?: Record<string, string>;
  hint?: string;
};

export type ImageGenFieldDef = {
  id: ImageGenFieldId;
  titleKey: string;
  selectedId: string;
  valueLabelKey: string;
  valueLabelParams?: Record<string, string>;
  options: ImageGenFieldOption[];
};

export interface ImageGenCompose {
  ratio: ImageGenRatio;
  resolution: ImageGenResolution;
  width: number;
  height: number;
  sizeLocked: boolean;
  styleId: string;
  count: ImageGenCount;
}

export interface ImageGenStylePreset {
  id: string;
  /** i18n key under chat locales, e.g. imageGen.style.anime */
  labelKey: string;
  /** Appended to the model's generate_image prompt. Empty = no extra style. */
  prompt: string;
}

/** Pixel bounds for Images API size (`WxH`).
 * Keep in sync with Rust `IMAGE_SIZE_MIN` / `IMAGE_SIZE_MAX` / `IMAGE_SIZE_STEP`
 * in `src-tauri/src/core/ai/image_gen.rs`. */
export const IMAGE_GEN_MIN_PX = 256;
export const IMAGE_GEN_MAX_PX = 4096;
/** Aggregators (and some official hosts) reject sizes that are not multiples of 16. */
export const IMAGE_GEN_SIZE_STEP = 16;

/** Popular styles used as prompt prefixes. Each one names a distinct medium and forbids lookalikes. */
export const IMAGE_GEN_STYLE_PRESETS: ImageGenStylePreset[] = [
  { id: "none", labelKey: "imageGen.style.none", prompt: "" },
  {
    id: "photo",
    labelKey: "imageGen.style.photo",
    prompt:
      "Photograph captured with a real camera, not generated CGI. 35mm stills camera, 50mm f/1.8 lens, Kodak Portra 400, optical bokeh, accurate skin pores, fabric weave, and micro-contrast. Available light, slight film grain. Must look like a photograph from a physical camera: no illustration, no painterly edges, no 3D render, no anime.",
  },
  {
    id: "cinematic",
    labelKey: "imageGen.style.cinematic",
    prompt:
      "A single frame from a theatrical feature film. Super 35 / anamorphic 2x, widescreen blocking, motivated practical lights, teal-orange colorist grade, heavy 35mm grain, horizontal anamorphic flares, shallow focus on the subject. Movie production still, not a phone snapshot, not a digital painting, not anime, not product CGI.",
  },
  {
    id: "anime",
    labelKey: "imageGen.style.anime",
    prompt:
      "Japanese 2D TV anime cel. Crisp ink lineart, hard cel-shaded shadows in 2–3 bands, flat color fills, screen-tone textures, anime face proportions (large eyes, small nose, simplified mouth). 1990s–2000s Kyoto Animation / Gainax look. Strictly 2D: no photoreal skin, no 3D Blender look, no western cartoon, no painterly oil.",
  },
  {
    id: "ghibli",
    labelKey: "imageGen.style.ghibli",
    prompt:
      "Hand-painted Studio Ghibli background and character art. Gouache and watercolor skies, lush naive trees, warm pastoral daylight, Ghibli character design (round simple faces, gentle expressions). Storybook atmosphere like My Neighbor Totoro or Kiki's Delivery Service. Not photoreal, not modern digital anime, not dark cyberpunk, not 3D.",
  },
  {
    id: "oil",
    labelKey: "imageGen.style.oil",
    prompt:
      "Classical oil painting on linen canvas. Thick impasto, visible hog-bristle brushstrokes, layered glazes, Rembrandt/Sargent museum lighting, canvas weave and varnish sheen. Must read as paint on canvas from a meter away. Not a photo, not smooth digital airbrush, not watercolor, not anime.",
  },
  {
    id: "watercolor",
    labelKey: "imageGen.style.watercolor",
    prompt:
      "Transparent watercolor on cold-press paper. Wet-on-wet blooms, granulating pigment, unpainted paper whites, soft bleeding edges, faint pencil underdrawing. Light, airy, unfinished in the corners. Not opaque gouache, not oil impasto, not a photograph, not crisp vector, not anime cel.",
  },
  {
    id: "cyberpunk",
    labelKey: "imageGen.style.cyberpunk",
    prompt:
      "Nighttime neo-noir cyberpunk city. Saturated magenta and cyan neon signage (CJK glyphs), rain-slick asphalt with neon reflections, dense cables, holograms, smog, wet concrete, high-contrast rim light. Blade Runner / Ghost in the Shell production design. Night only: no pastoral daylight, no Ghibli warmth, no oil-painting look.",
  },
  {
    id: "render3d",
    labelKey: "imageGen.style.render3d",
    prompt:
      "Path-traced 3D CGI still (Octane or Cycles). Physically based materials, HDRI studio lighting, crisp ray-traced reflections, subsurface skin or product shaders, clean geometry, no film grain. Looks like a 3D software beauty render. Not 2D illustration, not anime lineart, not a camera photo, not painterly concept art.",
  },
  {
    id: "pixel",
    labelKey: "imageGen.style.pixel",
    prompt:
      "Strict pixel art sprite scene. 32–64 px character scale, 16-color NES/SNES palette, chunky pixels, dithering, no anti-aliasing, no smooth gradients. 1990s JRPG / arcade look. If you can see real-world photographic detail, it failed. Not high-resolution, not photoreal, not vector, not anime cel.",
  },
  {
    id: "flat",
    labelKey: "imageGen.style.flat",
    prompt:
      "Flat vector poster illustration. Hard geometric shapes, 4–6 solid brand colors, no gradients, no photoreal shading, no outlines or a single even hairline, Swiss/Bauhaus graphic design. Poster on a wall, not a 3D scene, not a painting, not a photo, not anime.",
  },
  {
    id: "concept",
    labelKey: "imageGen.style.concept",
    prompt:
      "Film/game production concept art. Painterly digital environment, strong silhouette, atmospheric perspective, single key-light mood, unfinished prop detail, previs for a director. ArtStation film-concept look. Not a finished photograph, not anime, not pixel art, not a clean 3D product shot.",
  },
];

export const IMAGE_GEN_RATIOS: { id: ImageGenRatio; labelKey: string; w: number; h: number }[] = [
  { id: "auto", labelKey: "imageGen.ratio.auto", w: 1, h: 1 },
  { id: "21:9", labelKey: "imageGen.ratio.ultrawide", w: 21, h: 9 },
  { id: "16:9", labelKey: "imageGen.ratio.widescreen", w: 16, h: 9 },
  { id: "3:2", labelKey: "imageGen.ratio.photo", w: 3, h: 2 },
  { id: "4:3", labelKey: "imageGen.ratio.landscape", w: 4, h: 3 },
  { id: "1:1", labelKey: "imageGen.ratio.square", w: 1, h: 1 },
  { id: "3:4", labelKey: "imageGen.ratio.portrait", w: 3, h: 4 },
  { id: "2:3", labelKey: "imageGen.ratio.tall", w: 2, h: 3 },
  { id: "9:16", labelKey: "imageGen.ratio.story", w: 9, h: 16 },
];

export const IMAGE_GEN_RESOLUTIONS: {
  id: ImageGenResolution;
  labelKey: string;
  shortLabel: string;
  longEdge: number;
}[] = [
  { id: "1.5k", labelKey: "imageGen.resolution.sd", shortLabel: "1.5K", longEdge: 1536 },
  { id: "2k", labelKey: "imageGen.resolution.hd", shortLabel: "2K", longEdge: 2048 },
  { id: "4k", labelKey: "imageGen.resolution.uhd", shortLabel: "4K", longEdge: 3840 },
];

export const IMAGE_GEN_COUNTS: ImageGenCount[] = [1, 2, 3, 4];

export function isImageGenSettingsField(id: ImageGenFieldId | null | undefined): boolean {
  return id === "ratio" || id === "resolution" || id === "count";
}

export function isImageGenListField(id: ImageGenFieldId | null | undefined): boolean {
  return id === "style" || id === "model";
}

export function encodeImageModelSelection(provider: string, model: string): string {
  return JSON.stringify([provider, model]);
}

export function decodeImageModelSelection(
  value: string,
): { provider: string; model: string } | null {
  try {
    const parsed = JSON.parse(value) as unknown;
    if (!Array.isArray(parsed) || parsed.length < 2) return null;
    const provider = typeof parsed[0] === "string" ? parsed[0].trim() : "";
    const model = typeof parsed[1] === "string" ? parsed[1].trim() : "";
    if (!provider || !model) return null;
    return { provider, model };
  } catch {
    return null;
  }
}

export function listImageModelChoices(
  providers: Array<{
    id: string;
    name: string;
    models: string;
    disabledModels?: string;
  }>,
): ImageGenFieldOption[] {
  const options: ImageGenFieldOption[] = [];
  for (const provider of providers) {
    const name = provider.name.trim() || provider.id;
    const disabled = new Set(parseProviderModels(provider.disabledModels ?? ""));
    for (const id of parseProviderModels(provider.models)) {
      if (disabled.has(id)) continue;
      options.push({
        id: encodeImageModelSelection(provider.id, id),
        label: name ? `${id} · ${name}` : id,
      });
    }
  }
  return options;
}

export function selectedImageModelChoiceId(
  provider: string,
  model: string,
  choices: ImageGenFieldOption[],
): string {
  const value = encodeImageModelSelection(provider.trim(), model.trim());
  return choices.some((item) => item.id === value) ? value : "";
}

export function defaultImageGenCompose(): ImageGenCompose {
  const resolution: ImageGenResolution = "2k";
  const dims = dimensionsFor("1:1", resolution);
  return {
    ratio: "auto",
    resolution,
    width: dims.width,
    height: dims.height,
    sizeLocked: false,
    styleId: "none",
    count: 1,
  };
}

export function clampImagePx(value: number): number {
  if (!Number.isFinite(value)) return IMAGE_GEN_MIN_PX;
  const clamped = Math.min(IMAGE_GEN_MAX_PX, Math.max(IMAGE_GEN_MIN_PX, value));
  const snapped = Math.round(clamped / IMAGE_GEN_SIZE_STEP) * IMAGE_GEN_SIZE_STEP;
  return Math.min(IMAGE_GEN_MAX_PX, Math.max(IMAGE_GEN_MIN_PX, snapped));
}

export function ratioParts(ratio: ImageGenRatio): { w: number; h: number } {
  const item = IMAGE_GEN_RATIOS.find((entry) => entry.id === ratio);
  return { w: item?.w ?? 1, h: item?.h ?? 1 };
}

export function dimensionsFor(
  ratio: ImageGenRatio,
  resolution: ImageGenResolution,
): { width: number; height: number } {
  const { w, h } = ratioParts(ratio === "auto" ? "1:1" : ratio);
  const long = IMAGE_GEN_RESOLUTIONS.find((item) => item.id === resolution)?.longEdge ?? 2048;
  let width: number;
  let height: number;
  if (w >= h) {
    width = long;
    height = Math.round((long * h) / w);
  } else {
    height = long;
    width = Math.round((long * w) / h);
  }
  return { width: clampImagePx(width), height: clampImagePx(height) };
}

function migrateResolution(raw: unknown): ImageGenResolution {
  if (raw === "1.5k" || raw === "2k" || raw === "4k") return raw;
  if (raw === "low") return "1.5k";
  if (raw === "high") return "4k";
  return "2k";
}

export function normalizeImageGenCompose(raw: unknown): ImageGenCompose {
  const base = defaultImageGenCompose();
  if (!raw || typeof raw !== "object") return base;
  const value = raw as Partial<ImageGenCompose> & { quality?: unknown };
  const ratio = IMAGE_GEN_RATIOS.some((item) => item.id === value.ratio)
    ? value.ratio!
    : base.ratio;
  const resolution = migrateResolution(value.resolution ?? value.quality);
  const rawStyle = typeof value.styleId === "string" ? value.styleId.trim() : "";
  const styleId = rawStyle || base.styleId;
  const count = IMAGE_GEN_COUNTS.includes(value.count as ImageGenCount)
    ? (value.count as ImageGenCount)
    : base.count;
  const fallback = dimensionsFor(ratio, resolution);
  const width = typeof value.width === "number" ? clampImagePx(value.width) : fallback.width;
  const height = typeof value.height === "number" ? clampImagePx(value.height) : fallback.height;
  const sizeLocked = typeof value.sizeLocked === "boolean" ? value.sizeLocked : ratio !== "auto";
  return { ratio, resolution, width, height, sizeLocked, styleId, count };
}

export function normalizeImageStyleTemplates(raw: unknown): ImageStyleTemplate[] {
  if (!Array.isArray(raw)) return [];
  const out: ImageStyleTemplate[] = [];
  for (const item of raw) {
    if (!item || typeof item !== "object") continue;
    const value = item as Partial<ImageStyleTemplate>;
    const id = typeof value.id === "string" ? value.id.trim() : "";
    const name = typeof value.name === "string" ? value.name.trim() : "";
    if (!id || !name) continue;
    const prompt = typeof value.prompt === "string" ? value.prompt.trim() : "";
    const exampleImage =
      typeof value.exampleImage === "string" && value.exampleImage.startsWith("data:image/")
        ? value.exampleImage
        : undefined;
    out.push({ id, name, prompt, exampleImage });
  }
  return out;
}

export function newImageStyleTemplateId(): string {
  const rand =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
  return `custom-${rand}`;
}

export function stylePromptForId(styleId: string, templates: ImageStyleTemplate[] = []): string {
  const builtin = IMAGE_GEN_STYLE_PRESETS.find((item) => item.id === styleId);
  if (builtin) return builtin.prompt;
  return templates.find((item) => item.id === styleId)?.prompt ?? "";
}

export function exampleImageForStyle(
  styleId: string,
  templates: ImageStyleTemplate[] = [],
): string | undefined {
  return templates.find((item) => item.id === styleId)?.exampleImage;
}

/**
 * Map UI resolution tier → Images API `quality` (low/medium/high/auto).
 * Pixel size still comes from `width`×`height` / `size`; this only sets render quality.
 */
export function imagesApiQualityForResolution(resolution: ImageGenResolution): "medium" | "high" {
  return resolution === "1.5k" ? "medium" : "high";
}

export function imageGenPayload(
  options: ImageGenCompose,
  templates: ImageStyleTemplate[] = [],
): {
  size: string;
  /** Images API quality — distinct from UI `resolution` (1.5k/2k/4k). */
  quality: "low" | "medium" | "high" | "auto";
  n: ImageGenCount;
  stylePrompt: string;
  exampleImage?: string;
} {
  const exampleImage = exampleImageForStyle(options.styleId, templates);
  return {
    size: `${clampImagePx(options.width)}x${clampImagePx(options.height)}`,
    quality: imagesApiQualityForResolution(options.resolution),
    n: options.count,
    stylePrompt: stylePromptForId(options.styleId, templates),
    exampleImage,
  };
}

export function imageGenFieldDefs(
  value: ImageGenCompose,
  templates: ImageStyleTemplate[] = [],
): ImageGenFieldDef[] {
  const builtinStyle = IMAGE_GEN_STYLE_PRESETS.find((item) => item.id === value.styleId);
  const ratio = IMAGE_GEN_RATIOS.find((item) => item.id === value.ratio);
  const resolution = IMAGE_GEN_RESOLUTIONS.find((item) => item.id === value.resolution);
  const styleOptions: ImageGenFieldOption[] = [
    ...IMAGE_GEN_STYLE_PRESETS.map((item) => ({
      id: item.id,
      labelKey: item.labelKey,
      hint: item.prompt || undefined,
    })),
    ...templates.map((item) => ({
      id: item.id,
      label: item.name,
      hint: item.prompt || undefined,
    })),
  ];
  return [
    {
      id: "ratio",
      titleKey: "imageGen.ratio",
      selectedId: value.ratio,
      valueLabelKey: ratio?.labelKey ?? "imageGen.ratio.auto",
      options: IMAGE_GEN_RATIOS.map((item) => ({ id: item.id, labelKey: item.labelKey })),
    },
    {
      id: "resolution",
      // UI field is long-edge resolution (1.5k/2k/4k), not Images API `quality`.
      titleKey: "imageGen.resolutionTitle",
      selectedId: value.resolution,
      valueLabelKey: resolution?.labelKey ?? "imageGen.resolution.hd",
      options: IMAGE_GEN_RESOLUTIONS.map((item) => ({ id: item.id, labelKey: item.labelKey })),
    },
    {
      id: "style",
      titleKey: "imageGen.style",
      selectedId: value.styleId,
      valueLabelKey: builtinStyle?.labelKey ?? "imageGen.style.none",
      options: styleOptions,
    },
    {
      id: "count",
      titleKey: "imageGen.count",
      selectedId: String(value.count),
      valueLabelKey: "imageGen.countValue",
      valueLabelParams: { count: String(value.count) },
      options: IMAGE_GEN_COUNTS.map((count) => ({
        id: String(count),
        labelKey: "imageGen.countValue",
        labelParams: { count: String(count) },
      })),
    },
  ];
}

export function applyImageGenField(
  current: ImageGenCompose,
  field: ImageGenFieldId,
  id: string,
): ImageGenCompose {
  if (field === "ratio") return applyImageGenRatio(current, id as ImageGenRatio);
  if (field === "resolution") return applyImageGenResolution(current, id as ImageGenResolution);
  if (field === "style") return normalizeImageGenCompose({ ...current, styleId: id });
  if (field === "count") {
    return normalizeImageGenCompose({ ...current, count: Number(id) as ImageGenCount });
  }
  if (field === "model") return current;
  return normalizeImageGenCompose(current);
}

export function applyImageGenRatio(
  current: ImageGenCompose,
  ratio: ImageGenRatio,
): ImageGenCompose {
  const nextRatio = IMAGE_GEN_RATIOS.some((item) => item.id === ratio) ? ratio : current.ratio;
  const dims = dimensionsFor(nextRatio, current.resolution);
  return normalizeImageGenCompose({
    ...current,
    ratio: nextRatio,
    sizeLocked: nextRatio !== "auto",
    width: dims.width,
    height: dims.height,
  });
}

export function applyImageGenResolution(
  current: ImageGenCompose,
  resolution: ImageGenResolution,
): ImageGenCompose {
  const nextResolution = IMAGE_GEN_RESOLUTIONS.some((item) => item.id === resolution)
    ? resolution
    : current.resolution;
  const dims = dimensionsFor(current.ratio, nextResolution);
  return normalizeImageGenCompose({
    ...current,
    resolution: nextResolution,
    width: dims.width,
    height: dims.height,
  });
}

export function applyImageGenWidth(current: ImageGenCompose, width: number): ImageGenCompose {
  const nextWidth = clampImagePx(width);
  if (!current.sizeLocked) {
    return normalizeImageGenCompose({ ...current, width: nextWidth });
  }
  const { w, h } = ratioParts(current.ratio === "auto" ? "1:1" : current.ratio);
  return normalizeImageGenCompose({
    ...current,
    width: nextWidth,
    height: clampImagePx(Math.round((nextWidth * h) / w)),
  });
}

export function applyImageGenHeight(current: ImageGenCompose, height: number): ImageGenCompose {
  const nextHeight = clampImagePx(height);
  if (!current.sizeLocked) {
    return normalizeImageGenCompose({ ...current, height: nextHeight });
  }
  const { w, h } = ratioParts(current.ratio === "auto" ? "1:1" : current.ratio);
  return normalizeImageGenCompose({
    ...current,
    height: nextHeight,
    width: clampImagePx(Math.round((nextHeight * w) / h)),
  });
}

export function applyImageGenSizeLock(current: ImageGenCompose, locked: boolean): ImageGenCompose {
  return normalizeImageGenCompose({ ...current, sizeLocked: locked });
}

export function imageGenPickerWidth(field: ImageGenFieldId): number {
  if (field === "style") return 200;
  if (field === "model") return 240;
  return 560;
}

export function imageGenComposeEqual(a: ImageGenCompose, b: ImageGenCompose): boolean {
  return (
    a.ratio === b.ratio &&
    a.resolution === b.resolution &&
    a.width === b.width &&
    a.height === b.height &&
    a.sizeLocked === b.sizeLocked &&
    a.styleId === b.styleId &&
    a.count === b.count
  );
}
