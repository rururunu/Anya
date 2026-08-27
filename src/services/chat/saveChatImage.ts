/**
 * Save a chat image (generated, pasted, or remote) through the native
 * "Save as" dialog in Tauri, with an anchor-download fallback in the browser.
 *
 * Local files are copied with plugin-fs so the original bytes are preserved.
 * Data URLs and http(s) sources are decoded / fetched and written.
 */

import { dataUrlToBytes, isTauri } from "@/services/platform";
import {
  isLocalImagePath,
  resolveChatImageSrc,
  unwrapLocalImagePath,
} from "@/services/chat/localImageSrc";

export { unwrapLocalImagePath };

const IMAGE_EXT = /^(png|jpe?g|webp|gif|bmp|svg|avif)$/i;

export type SaveChatImageResult = "saved" | "cancelled" | "failed";

function sanitizeStem(name: string, fallback: string): string {
  const cleaned = name
    .trim()
    .replace(/[\\/:*?"<>|]/g, "_")
    .slice(0, 120);
  return cleaned || fallback;
}

function extensionFromMime(mime: string): string {
  const subtype = mime.split("/")[1]?.split("+")[0]?.trim().toLowerCase() || "png";
  if (subtype === "jpeg") return "jpg";
  return subtype || "png";
}

/** Suggested filename for the save dialog, including a sensible extension. */
export function suggestedImageFilename(source: string, fallback = "generated"): string {
  const value = source.trim();
  if (value.startsWith("data:")) {
    const comma = value.indexOf(",");
    const header = comma >= 0 ? value.slice(5, comma) : value.slice(5);
    const mime = header.split(";")[0] || "image/png";
    return `${sanitizeStem(fallback, "generated")}.${extensionFromMime(mime)}`;
  }

  const path = unwrapLocalImagePath(value).split(/[?#]/)[0] ?? "";
  const base = path.split(/[\\/]/).filter(Boolean).pop() || fallback;
  const dot = base.lastIndexOf(".");
  if (dot > 0) {
    const ext = base.slice(dot + 1);
    if (IMAGE_EXT.test(ext)) {
      const normalized = ext.toLowerCase() === "jpeg" ? "jpg" : ext.toLowerCase();
      return `${sanitizeStem(base.slice(0, dot), fallback)}.${normalized}`;
    }
  }
  return `${sanitizeStem(base, fallback)}.png`;
}

function dialogFilters(filename: string): { name: string; extensions: string[] }[] {
  const ext = filename.split(".").pop()?.toLowerCase() || "png";
  const normalized = ext === "jpeg" ? "jpg" : ext;
  return [{ name: `${normalized.toUpperCase()} Image`, extensions: [normalized] }];
}

async function bytesFromSource(source: string): Promise<Uint8Array> {
  const value = source.trim();
  if (value.startsWith("data:")) return dataUrlToBytes(value);
  const url = resolveChatImageSrc(value);
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Could not read image (${response.status})`);
  return new Uint8Array(await response.arrayBuffer());
}

async function writeBytes(dest: string, bytes: Uint8Array): Promise<void> {
  const { writeFile } = await import("@tauri-apps/plugin-fs");
  await writeFile(dest, bytes);
}

async function saveLocalFile(source: string, dest: string): Promise<void> {
  const from = unwrapLocalImagePath(source);
  const { copyFile, readFile } = await import("@tauri-apps/plugin-fs");
  try {
    await copyFile(from, dest);
    return;
  } catch {
    // copy-file may be denied; read + write still works with write-file scope.
  }
  try {
    await writeBytes(dest, await readFile(from));
    return;
  } catch {
    await writeBytes(dest, await bytesFromSource(source));
  }
}

function triggerBrowserDownload(bytes: Uint8Array, filename: string): void {
  const blob = new Blob([bytes], { type: "application/octet-stream" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}

/** Prompt the user to save `source`. Cancel is not a failure. */
export async function saveChatImage(
  source: string,
  fallbackName = "generated",
): Promise<SaveChatImageResult> {
  const value = source.trim();
  if (!value) return "failed";
  const filename = suggestedImageFilename(value, fallbackName);

  try {
    if (isTauri()) {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const dest = await save({
        defaultPath: filename,
        filters: dialogFilters(filename),
      });
      if (!dest) return "cancelled";
      if (isLocalImagePath(value)) {
        await saveLocalFile(value, dest);
      } else {
        await writeBytes(dest, await bytesFromSource(value));
      }
      return "saved";
    }

    triggerBrowserDownload(await bytesFromSource(value), filename);
    return "saved";
  } catch (error) {
    console.warn("save image failed:", error);
    return "failed";
  }
}
