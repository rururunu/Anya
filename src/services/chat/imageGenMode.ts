import { parseProviderModels } from "@/lib/providerPresets";

export type ImageGenRatio =
  "auto" | "21:9" | "16:9" | "3:2" | "4:3" | "1:1" | "3:4" | "2:3" | "9:16";
export type ImageGenResolution = "1.5k" | "2k" | "4k";
export type ImageGenFieldId = "ratio" | "resolution" | "model";

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
}

/** Pixel bounds for Images API size (`WxH`).
 * Keep in sync with Rust `IMAGE_SIZE_MIN` / `IMAGE_SIZE_MAX` / `IMAGE_SIZE_STEP`
 * in `src-tauri/src/core/ai/image_gen.rs`. */
export const IMAGE_GEN_MIN_PX = 256;
export const IMAGE_GEN_MAX_PX = 4096;
/** Aggregators (and some official hosts) reject sizes that are not multiples of 16. */
export const IMAGE_GEN_SIZE_STEP = 16;

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

export function isImageGenSettingsField(id: ImageGenFieldId | null | undefined): boolean {
  return id === "ratio" || id === "resolution";
}

export function isImageGenListField(id: ImageGenFieldId | null | undefined): boolean {
  return id === "model";
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
  const fallback = dimensionsFor(ratio, resolution);
  const width = typeof value.width === "number" ? clampImagePx(value.width) : fallback.width;
  const height = typeof value.height === "number" ? clampImagePx(value.height) : fallback.height;
  const sizeLocked = typeof value.sizeLocked === "boolean" ? value.sizeLocked : ratio !== "auto";
  return { ratio, resolution, width, height, sizeLocked };
}

/**
 * Map UI resolution tier → Images API `quality` (low/medium/high/auto).
 * Pixel size still comes from `width`×`height` / `size`; this only sets render quality.
 */
export function imagesApiQualityForResolution(resolution: ImageGenResolution): "medium" | "high" {
  return resolution === "1.5k" ? "medium" : "high";
}

export function imageGenPayload(options: ImageGenCompose): {
  size: string;
  /** Images API quality — distinct from UI `resolution` (1.5k/2k/4k). */
  quality: "low" | "medium" | "high" | "auto";
  n: 1;
} {
  return {
    size: `${clampImagePx(options.width)}x${clampImagePx(options.height)}`,
    quality: imagesApiQualityForResolution(options.resolution),
    n: 1,
  };
}

export function imageGenFieldDefs(value: ImageGenCompose): ImageGenFieldDef[] {
  const ratio = IMAGE_GEN_RATIOS.find((item) => item.id === value.ratio);
  const resolution = IMAGE_GEN_RESOLUTIONS.find((item) => item.id === value.resolution);
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
  ];
}

export function applyImageGenField(
  current: ImageGenCompose,
  field: ImageGenFieldId,
  id: string,
): ImageGenCompose {
  if (field === "ratio") return applyImageGenRatio(current, id as ImageGenRatio);
  if (field === "resolution") return applyImageGenResolution(current, id as ImageGenResolution);
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
  if (field === "model") return 240;
  return 560;
}

export function imageGenComposeEqual(a: ImageGenCompose, b: ImageGenCompose): boolean {
  return (
    a.ratio === b.ratio &&
    a.resolution === b.resolution &&
    a.width === b.width &&
    a.height === b.height &&
    a.sizeLocked === b.sizeLocked
  );
}
