/**
 * Chart PNG export for ChartCard.
 *
 * Uses echarts' getDataURL, which renders every zrender layer into one
 * canvas: 2D layers are drawn directly, and the echarts-gl GL layer is
 * composited through LayerGL.renderToCanvas (its WebGL canvas keeps
 * preserveDrawingBuffer: true, so the content is readable). This works for
 * both 2D and 3D charts.
 *
 * One caveat: zrender's getRenderedCanvas only composites layers when
 * `pixelRatio <= devicePixelRatio`; above that it re-brushes the 2D display
 * list and GL layers are lost. Callers exporting 3D charts must therefore
 * leave pixelRatio undefined (echarts then falls back to the device ratio).
 *
 * Delivery:
 * - In the Tauri desktop app, WebView2 silently drops `<a download>` clicks
 *   on data: URLs, so the export goes through the native save dialog
 *   (plugin-dialog) and writes the file with plugin-fs.
 * - In a plain browser context (dev server without Tauri), it falls back to
 *   an anchor download.
 */

export interface ChartLike {
  getDataURL(opts?: { type?: string; pixelRatio?: number; backgroundColor?: unknown }): string;
}

export interface ExportImageOptions {
  pixelRatio?: number;
  backgroundColor?: string;
}

import { dataUrlToBytes, isTauri } from "@/services/platform";

export { dataUrlToBytes, isTauri };
/** Strips characters that are illegal in filenames on Windows/macOS/Linux
 *  and caps the length, so a model-supplied title can be used as-is. */
export function safeFilename(name: string): string {
  const cleaned = name
    .trim()
    .replace(/[\\/:*?"<>|]/g, "_")
    .slice(0, 120);
  return cleaned || "chart";
}

/** Exports the chart as a PNG file. Returns false (without throwing) when
 *  the export fails, so callers can surface an inline error. Returns true
 *  when the user cancels the save dialog (that is not a failure). */
export async function exportChartPng(
  chart: ChartLike,
  filename: string,
  opts: ExportImageOptions = {},
): Promise<boolean> {
  try {
    const url = chart.getDataURL({
      type: "png",
      pixelRatio: opts.pixelRatio,
      backgroundColor: opts.backgroundColor,
    });
    if (!url) return false;
    const name = `${safeFilename(filename)}.png`;

    if (isTauri()) {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const { writeFile } = await import("@tauri-apps/plugin-fs");
      const path = await save({
        defaultPath: name,
        filters: [{ name: "PNG Image", extensions: ["png"] }],
      });
      if (!path) return true; // cancelled by the user
      await writeFile(path, dataUrlToBytes(url));
      return true;
    }

    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = name;
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    return true;
  } catch (error) {
    console.warn("chart export failed:", error);
    return false;
  }
}
