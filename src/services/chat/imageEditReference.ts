/**
 * Helpers for "edit from this image" / region reference images.
 * Loads chat image sources into data URLs for the Images edits API.
 * Region edits attach a dimmed spotlight for the message list, plus a separate
 * full-brightness original for the edits API (never send the dimmed composite
 * as the edits reference).
 */

import { compressImageDataUrl } from "@/services/chat/compressImage";
import {
  isLocalImagePath,
  resolveChatImageSrc,
  unwrapLocalImagePath,
} from "@/services/chat/localImageSrc";

export type NormalizedRect = {
  /** 0–1 relative to natural image width */
  x: number;
  /** 0–1 relative to natural image height */
  y: number;
  w: number;
  h: number;
};

/** One freehand stroke in normalized image coordinates (0–1). */
export type NormalizedStroke = {
  /** Brush radius as a fraction of the shorter image edge (0–1). */
  radius: number;
  points: Array<{ x: number; y: number }>;
};

export type ImageEditReferencePayload = {
  /**
   * Data URLs shown in the composer / message list.
   * For region edits this is the dimmed spotlight preview so the paint is visible.
   */
  images: string[];
  /**
   * Full-brightness originals used as Images edits references.
   * Parallel to `images` when set; omitted entries fall back to `images[i]`.
   * Never send the dimmed preview to the edits API — it makes the next image darker.
   */
  editSources?: string[];
  /** Optional draft text to leave in the input (user completes / sends). */
  draftText?: string;
  /** True when a region was marked. */
  region?: boolean;
  /** Normalized painted/selected bounds, when available. */
  regionBounds?: NormalizedRect;
};

/** Markdown alt for display-only paint previews (skipped by the edits API extractor). */
export const EDIT_REGION_IMAGE_ALT = "edit-region";

function bytesToDataUrl(bytes: Uint8Array, mime: string): string {
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return `data:${mime};base64,${btoa(binary)}`;
}

function mimeFromPath(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase() ?? "png";
  if (ext === "jpg" || ext === "jpeg") return "image/jpeg";
  if (ext === "webp") return "image/webp";
  if (ext === "gif") return "image/gif";
  return "image/png";
}

/** Load a chat image source (`path:`, data URL, http) into a data URL. */
export async function loadImageSourceAsDataUrl(source: string): Promise<string> {
  const value = source.trim();
  if (!value) throw new Error("empty image source");
  if (value.startsWith("data:")) return value;

  if (isLocalImagePath(value)) {
    try {
      const { readFile } = await import("@tauri-apps/plugin-fs");
      const path = unwrapLocalImagePath(value);
      const bytes = await readFile(path);
      return bytesToDataUrl(bytes, mimeFromPath(path));
    } catch {
      // Fall through to fetch via convertFileSrc.
    }
  }

  const url = resolveChatImageSrc(value);
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Could not read image (${response.status})`);
  const blob = await response.blob();
  return await new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ""));
    reader.onerror = () => reject(new Error("Could not decode image"));
    reader.readAsDataURL(blob);
  });
}

function loadHtmlImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error("Could not decode image"));
    image.src = src;
  });
}

function clamp01(value: number): number {
  return Math.min(1, Math.max(0, value));
}

export function normalizeRect(rect: NormalizedRect): NormalizedRect | null {
  const x = clamp01(rect.x);
  const y = clamp01(rect.y);
  const right = clamp01(rect.x + rect.w);
  const bottom = clamp01(rect.y + rect.h);
  const w = Math.round((right - x) * 1e6) / 1e6;
  const h = Math.round((bottom - y) * 1e6) / 1e6;
  if (w < 0.02 || h < 0.02) return null;
  return { x, y, w, h };
}

export function strokePointCount(strokes: NormalizedStroke[]): number {
  return strokes.reduce((sum, stroke) => sum + stroke.points.length, 0);
}

export function hasUsableStrokes(strokes: NormalizedStroke[]): boolean {
  return strokes.some((stroke) => stroke.points.length > 0 && stroke.radius > 0);
}

function paintStrokesOnMask(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  strokes: NormalizedStroke[],
) {
  const shortEdge = Math.min(width, height);
  ctx.fillStyle = "#fff";
  ctx.strokeStyle = "#fff";
  ctx.lineCap = "round";
  ctx.lineJoin = "round";

  for (const stroke of strokes) {
    if (stroke.points.length === 0) continue;
    const radiusPx = Math.max(2, stroke.radius * shortEdge);
    ctx.lineWidth = radiusPx * 2;

    if (stroke.points.length === 1) {
      const point = stroke.points[0]!;
      ctx.beginPath();
      ctx.arc(point.x * width, point.y * height, radiusPx, 0, Math.PI * 2);
      ctx.fill();
      continue;
    }

    ctx.beginPath();
    stroke.points.forEach((point, index) => {
      const x = point.x * width;
      const y = point.y * height;
      if (index === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.stroke();
  }
}

/**
 * Dim everything outside the freehand strokes; keep the painted area bright
 * (and slightly lifted) so the image model sees a spotlight, not a red box.
 */
export async function buildStrokeHighlightReferenceDataUrl(
  sourceDataUrl: string,
  strokes: NormalizedStroke[],
): Promise<string> {
  if (!hasUsableStrokes(strokes)) throw new Error("region too small");

  const image = await loadHtmlImage(sourceDataUrl);
  const width = image.naturalWidth || image.width;
  const height = image.naturalHeight || image.height;
  if (!width || !height) throw new Error("invalid image size");

  const mask = document.createElement("canvas");
  mask.width = width;
  mask.height = height;
  const maskCtx = mask.getContext("2d");
  if (!maskCtx) throw new Error("canvas unavailable");
  maskCtx.clearRect(0, 0, width, height);
  paintStrokesOnMask(maskCtx, width, height, strokes);

  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("canvas unavailable");

  ctx.drawImage(image, 0, 0, width, height);

  // Soft brighten inside the painted region.
  const lift = document.createElement("canvas");
  lift.width = width;
  lift.height = height;
  const liftCtx = lift.getContext("2d");
  if (!liftCtx) throw new Error("canvas unavailable");
  liftCtx.fillStyle = "rgba(255, 255, 255, 0.18)";
  liftCtx.fillRect(0, 0, width, height);
  liftCtx.globalCompositeOperation = "destination-in";
  liftCtx.drawImage(mask, 0, 0);
  ctx.drawImage(lift, 0, 0);

  // Dim outside the painted mask.
  const dim = document.createElement("canvas");
  dim.width = width;
  dim.height = height;
  const dimCtx = dim.getContext("2d");
  if (!dimCtx) throw new Error("canvas unavailable");
  dimCtx.fillStyle = "rgba(0, 0, 0, 0.52)";
  dimCtx.fillRect(0, 0, width, height);
  dimCtx.globalCompositeOperation = "destination-out";
  dimCtx.drawImage(mask, 0, 0);
  ctx.drawImage(dim, 0, 0);

  return canvas.toDataURL("image/png");
}

/**
 * Darken everything outside the region and draw a bright border so the
 * image model can see what to change.
 */
export async function buildHighlightReferenceDataUrl(
  sourceDataUrl: string,
  rect: NormalizedRect,
): Promise<string> {
  const normalized = normalizeRect(rect);
  if (!normalized) throw new Error("region too small");

  const image = await loadHtmlImage(sourceDataUrl);
  const width = image.naturalWidth || image.width;
  const height = image.naturalHeight || image.height;
  if (!width || !height) throw new Error("invalid image size");

  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("canvas unavailable");

  ctx.drawImage(image, 0, 0, width, height);

  const rx = Math.round(normalized.x * width);
  const ry = Math.round(normalized.y * height);
  const rw = Math.max(1, Math.round(normalized.w * width));
  const rh = Math.max(1, Math.round(normalized.h * height));

  ctx.save();
  ctx.fillStyle = "rgba(0, 0, 0, 0.45)";
  ctx.beginPath();
  ctx.rect(0, 0, width, height);
  ctx.rect(rx, ry, rw, rh);
  ctx.fill("evenodd");
  ctx.restore();

  const stroke = Math.max(2, Math.round(Math.min(width, height) * 0.006));
  ctx.strokeStyle = "rgba(255, 70, 70, 0.95)";
  ctx.lineWidth = stroke;
  ctx.setLineDash([]);
  ctx.strokeRect(rx + stroke / 2, ry + stroke / 2, rw - stroke, rh - stroke);

  ctx.strokeStyle = "rgba(255, 255, 255, 0.85)";
  ctx.lineWidth = Math.max(1, stroke * 0.4);
  ctx.strokeRect(rx + stroke * 1.2, ry + stroke * 1.2, rw - stroke * 2.4, rh - stroke * 2.4);

  return canvas.toDataURL("image/png");
}

/** Prepare attachments for a full-image edit (no region). */
export async function prepareFullImageEdit(source: string): Promise<ImageEditReferencePayload> {
  const dataUrl = await loadImageSourceAsDataUrl(source);
  const compressed = await compressImageDataUrl(dataUrl);
  return { images: [compressed], region: false };
}

/**
 * Prepare attachments for a freehand region edit.
 * Composer/message show the dimmed spotlight so the paint is visible;
 * edits API gets the original full-brightness image only.
 */
export async function prepareStrokeImageEdit(
  source: string,
  strokes: NormalizedStroke[],
): Promise<ImageEditReferencePayload> {
  if (!hasUsableStrokes(strokes)) throw new Error("region too small");
  const dataUrl = await loadImageSourceAsDataUrl(source);
  const highlight = await buildStrokeHighlightReferenceDataUrl(dataUrl, strokes);
  const [preview, original] = await Promise.all([
    compressImageDataUrl(highlight),
    compressImageDataUrl(dataUrl),
  ]);
  const bounds = strokeBounds(strokes);
  return {
    images: [preview],
    editSources: [original],
    region: true,
    draftText: "",
    regionBounds: bounds ?? undefined,
  };
}

/**
 * Prepare attachments for a rectangular region edit.
 * Same split as stroke edit: dimmed preview for humans, original for the API.
 */
export async function prepareRegionImageEdit(
  source: string,
  rect: NormalizedRect,
): Promise<ImageEditReferencePayload> {
  const normalized = normalizeRect(rect);
  if (!normalized) throw new Error("region too small");
  const dataUrl = await loadImageSourceAsDataUrl(source);
  const highlight = await buildHighlightReferenceDataUrl(dataUrl, normalized);
  const [preview, original] = await Promise.all([
    compressImageDataUrl(highlight),
    compressImageDataUrl(dataUrl),
  ]);
  return {
    images: [preview],
    editSources: [original],
    region: true,
    draftText: "",
    regionBounds: normalized,
  };
}

/** Bounding box of freehand strokes (including brush radius padding). */
export function strokeBounds(strokes: NormalizedStroke[]): NormalizedRect | null {
  let minX = 1;
  let minY = 1;
  let maxX = 0;
  let maxY = 0;
  let any = false;
  for (const stroke of strokes) {
    const pad = Math.max(0, stroke.radius);
    for (const point of stroke.points) {
      any = true;
      minX = Math.min(minX, point.x - pad);
      minY = Math.min(minY, point.y - pad);
      maxX = Math.max(maxX, point.x + pad);
      maxY = Math.max(maxY, point.y + pad);
    }
  }
  if (!any) return null;
  return normalizeRect({ x: minX, y: minY, w: maxX - minX, h: maxY - minY });
}

/** Rough natural-language placement for prompts, e.g. "upper-left". */
export function describeRegionPlacement(bounds: NormalizedRect): string {
  const cx = bounds.x + bounds.w / 2;
  const cy = bounds.y + bounds.h / 2;
  const vertical = cy < 0.33 ? "upper" : cy > 0.66 ? "lower" : "middle";
  const horizontal = cx < 0.33 ? "left" : cx > 0.66 ? "right" : "center";
  if (vertical === "middle" && horizontal === "center") return "center";
  if (horizontal === "center") return vertical;
  if (vertical === "middle") return horizontal;
  return `${vertical}-${horizontal}`;
}
