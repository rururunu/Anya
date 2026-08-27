import { convertFileSrc } from "@tauri-apps/api/core";

/** True when `value` looks like a local filesystem path (or `path:` wrapper). */
export function isLocalImagePath(value: string): boolean {
  const trimmed = value.trim();
  if (!trimmed) return false;
  if (trimmed.startsWith("path:")) return true;
  if (/^[a-zA-Z]:[\\/]/.test(trimmed)) return true;
  if (trimmed.startsWith("\\\\")) return true;
  if (trimmed.startsWith("/") && !trimmed.startsWith("//")) return true;
  return false;
}

/** Strip a `path:` wrapper so plugin-fs / native APIs see a real filesystem path. */
export function unwrapLocalImagePath(source: string): string {
  const value = source.trim();
  return value.startsWith("path:") ? value.slice(5) : value;
}

/** Convert a chat image source into something `<img src>` can load in WebView. */
export function resolveChatImageSrc(source: string): string {
  const value = source.trim();
  if (!value || value.startsWith("data:") || /^https?:\/\//i.test(value)) {
    return value;
  }
  if (!isLocalImagePath(value)) {
    return value;
  }
  const path = unwrapLocalImagePath(value);
  try {
    return convertFileSrc(path);
  } catch {
    return value;
  }
}

const MARKDOWN_IMAGE = /!\[[^\]]*]\((path:[^)\s]+|(?:[a-zA-Z]:[\\/]|\/|\\\\)[^)\s]+)\)/g;
const ANYA_IMAGES_FENCE = /```anya-images\s*([\s\S]*?)```/i;

type StructuredGeneratedImage = {
  path: string;
  revisedPrompt?: string;
};

function normalizeStructuredPath(raw: string): string {
  const value = raw.trim();
  if (!value) return "";
  return value.startsWith("path:") ? value : `path:${value}`;
}

/** Prefer the trailing `anya-images` JSON fence when present. */
export function parseStructuredGeneratedImages(
  result: string | undefined,
): StructuredGeneratedImage[] {
  if (!result?.trim()) return [];
  const match = result.match(ANYA_IMAGES_FENCE);
  const raw = match?.[1]?.trim();
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw) as {
      images?: Array<{ path?: unknown; revised_prompt?: unknown; revisedPrompt?: unknown }>;
    };
    if (!Array.isArray(parsed.images)) return [];
    const out: StructuredGeneratedImage[] = [];
    for (const item of parsed.images) {
      if (typeof item?.path !== "string" || !item.path.trim()) continue;
      const path = normalizeStructuredPath(item.path);
      if (!isLocalImagePath(path)) continue;
      const revised =
        typeof item.revised_prompt === "string"
          ? item.revised_prompt.trim()
          : typeof item.revisedPrompt === "string"
            ? item.revisedPrompt.trim()
            : "";
      out.push(revised ? { path, revisedPrompt: revised } : { path });
    }
    return out;
  } catch {
    return [];
  }
}

/** Absolute / `path:` image targets from generate_image tool results. */
export function parseGeneratedImageSources(result: string | undefined): string[] {
  if (!result?.trim()) return [];
  const structured = parseStructuredGeneratedImages(result);
  if (structured.length > 0) {
    return structured.map((item) => item.path);
  }

  const sources: string[] = [];
  const seen = new Set<string>();
  for (const match of result.matchAll(MARKDOWN_IMAGE)) {
    const raw = match[1]?.trim();
    if (!raw || seen.has(raw)) continue;
    seen.add(raw);
    sources.push(raw);
  }
  if (sources.length > 0) return sources;

  for (const line of result.split(/\r?\n/)) {
    const saved = line.match(/^Saved:\s+(.+)$/i)?.[1]?.trim();
    if (saved && isLocalImagePath(saved) && !seen.has(saved)) {
      seen.add(saved);
      sources.push(saved.startsWith("path:") ? saved : `path:${saved}`);
    }
  }
  return sources;
}

function argumentPrompt(args?: Record<string, unknown>): string {
  const value = args?.prompt;
  return typeof value === "string" ? value.trim() : "";
}

function parseRevisedPrompts(result: string | undefined): string[] {
  if (!result) return [];
  const structured = parseStructuredGeneratedImages(result);
  if (structured.length > 0) {
    return structured.map((item) => item.revisedPrompt?.trim() ?? "").filter(Boolean);
  }
  const out: string[] = [];
  for (const match of result.matchAll(/^Revised prompt:\s*(.+)$/gim)) {
    const text = match[1]?.trim();
    if (text) out.push(text);
  }
  return out;
}

/** Prompt used to generate an image: API revision when present, else tool `prompt`. */
export function generatedImagePrompt(
  result: string | undefined,
  args?: Record<string, unknown>,
  source?: string,
): string {
  const original = argumentPrompt(args);
  const structured = parseStructuredGeneratedImages(result);
  if (structured.length > 0) {
    if (source) {
      const match = structured.find((item) => item.path === source);
      if (match?.revisedPrompt) return match.revisedPrompt;
    }
    return structured.find((item) => item.revisedPrompt)?.revisedPrompt || original;
  }
  const revised = parseRevisedPrompts(result);
  if (source && revised.length > 1) {
    const sources = parseGeneratedImageSources(result);
    const index = sources.findIndex((item) => item === source);
    if (index >= 0 && revised[index]) return revised[index];
  }
  return revised[0] || original;
}
