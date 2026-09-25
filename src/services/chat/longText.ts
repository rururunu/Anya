/**
 * Thresholds for long pasted / displayed user text.
 * Oversized paste becomes a .txt attachment; long bubbles fold in the thread.
 */

import { isTauri } from "@/services/platform";

/** Paste becomes a file when either limit is met. */
export const PASTE_AS_FILE_MIN_LINES = 40;
export const PASTE_AS_FILE_MIN_CHARS = 4000;

/** User bubble folds when either limit is exceeded. */
export const USER_MESSAGE_FOLD_MIN_LINES = 12;
export const USER_MESSAGE_FOLD_MIN_CHARS = 900;

/** Visible lines while a user bubble is collapsed. */
export const USER_MESSAGE_FOLD_VISIBLE_LINES = 10;

/** Soft-inject cards stay short; fold earlier than user questions. */
export const SOFT_INJECT_FOLD_MIN_LINES = 4;
export const SOFT_INJECT_FOLD_MIN_CHARS = 220;
export const SOFT_INJECT_FOLD_VISIBLE_LINES = 3;

/** Tool bodies fold later: expanding the activity is meant to show its output. */
export const TOOL_BODY_FOLD_MIN_LINES = 16;
export const TOOL_BODY_FOLD_MIN_CHARS = 1400;

/** Visible lines while a tool body is folded. */
export const TOOL_BODY_FOLD_VISIBLE_LINES = 12;

/** Count lines after normalizing newlines. */
export function countTextLines(text: string): number {
  if (!text) return 0;
  return text.replace(/\r\n|\r/g, "\n").split("\n").length;
}

/** Normalize clipboard / message newlines to `\n`. */
export function normalizeMultilineText(text: string): string {
  return text.replace(/\r\n|\r/g, "\n");
}

/** True when pasted text should become a .txt attachment instead of composer text. */
export function shouldAttachPasteAsFile(text: string): boolean {
  const normalized = normalizeMultilineText(text);
  if (!normalized) return false;
  return (
    countTextLines(normalized) >= PASTE_AS_FILE_MIN_LINES ||
    normalized.length >= PASTE_AS_FILE_MIN_CHARS
  );
}

/** True when a sent user message should start collapsed in the thread. */
export function shouldFoldUserMessage(text: string): boolean {
  return exceedsFoldThreshold(text, USER_MESSAGE_FOLD_MIN_LINES, USER_MESSAGE_FOLD_MIN_CHARS);
}

/** True when a mid-turn inject card should start collapsed. */
export function shouldFoldSoftInject(text: string): boolean {
  return exceedsFoldThreshold(text, SOFT_INJECT_FOLD_MIN_LINES, SOFT_INJECT_FOLD_MIN_CHARS);
}

/** True when an expanded tool body needs its own fold toggle. */
export function shouldFoldToolBody(text: string): boolean {
  return exceedsFoldThreshold(text, TOOL_BODY_FOLD_MIN_LINES, TOOL_BODY_FOLD_MIN_CHARS);
}

function exceedsFoldThreshold(text: string, minLines: number, minChars: number): boolean {
  const normalized = normalizeMultilineText(text);
  if (!normalized) return false;
  return countTextLines(normalized) > minLines || normalized.length > minChars;
}

/** Suggested filename for a pasted text attachment. */
export function pastedTextFilename(text: string, now = new Date()): string {
  const lines = countTextLines(text);
  const stamp = now.toISOString().replace(/[:.]/g, "-").slice(0, 19);
  return `pasted-${lines}lines-${stamp}.txt`;
}

/**
 * Build a File for composer attachment; on Tauri also write a temp path so
 * `filePathHint` / the model see a real absolute path.
 */
export async function createPastedTextFile(text: string): Promise<File> {
  const normalized = normalizeMultilineText(text);
  const name = pastedTextFilename(normalized);
  const file = new File([normalized], name, { type: "text/plain" });

  if (!isTauri()) return file;

  try {
    const { tempDir, join } = await import("@tauri-apps/api/path");
    const { writeFile } = await import("@tauri-apps/plugin-fs");
    const path = await join(await tempDir(), name);
    await writeFile(path, new TextEncoder().encode(normalized));
    Object.defineProperty(file, "path", { value: path, configurable: true });
  } catch {
    // In-memory File still inlines via attachFiles when content fits the budget.
  }

  return file;
}
