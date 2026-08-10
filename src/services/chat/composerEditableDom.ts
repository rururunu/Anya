/**
 * Contenteditable composer DOM helpers.
 * Source of truth is plain text (`@path`, `#skill:id`, `#mcp:id`);
 * tokens render as atomic inline marks (icons + colored labels).
 */

import { COMPOSER_INLINE_TOKEN_RE } from "@/services/chat/composerSegments";
import { splitInlineTokenParts, type InlineTokenPart } from "@/services/chat/inlineTokenMarks";

export const CE_TOKEN_ATTR = "data-ce-token";
export const CE_KIND_ATTR = "data-ce-kind";

export type ComposerTokenKind = "mention" | "skill" | "mcp";

export type ComposerTokenMeta = {
  kind: ComposerTokenKind;
  /** Wire token written into the message, e.g. `@src/a.ts`. */
  token: string;
  /** Visible label inside the mark. */
  label: string;
  title?: string;
  iconUrl?: string | null;
  /** Lucide-style fallback name when iconUrl is missing. */
  fallback: "file" | "folder" | "zap" | "bot";
  className: string;
};

export type ResolveComposerTokenMeta = (
  part: Exclude<InlineTokenPart, { kind: "text" }>,
) => ComposerTokenMeta;

/** Serialize editor DOM back to plain composer text. */
export function serializeComposerEditable(root: HTMLElement): string {
  let out = "";
  const walk = (node: Node) => {
    if (node.nodeType === Node.TEXT_NODE) {
      out += node.nodeValue ?? "";
      return;
    }
    if (!(node instanceof HTMLElement)) return;
    if (node.dataset.ceToken != null) {
      out += node.dataset.ceToken;
      return;
    }
    if (node.tagName === "BR") {
      out += "\n";
      return;
    }
    for (const child of Array.from(node.childNodes)) walk(child);
  };
  for (const child of Array.from(root.childNodes)) walk(child);
  // Contenteditable often keeps a trailing <br>; trim only a lone final newline
  // introduced by that empty line marker when the visual field looks empty.
  if (out === "\n") return "";
  return out;
}

/** Plain-text caret offsets for the current selection inside root. */
export function getComposerSelectionOffsets(root: HTMLElement): { start: number; end: number } {
  const sel = window.getSelection();
  if (!sel || sel.rangeCount === 0) {
    const len = serializeComposerEditable(root).length;
    return { start: len, end: len };
  }
  const anchor = sel.getRangeAt(0);
  const start = offsetFromRoot(root, anchor.startContainer, anchor.startOffset);
  const end =
    sel.rangeCount && !anchor.collapsed
      ? offsetFromRoot(root, anchor.endContainer, anchor.endOffset)
      : start;
  return start <= end ? { start, end } : { start: end, end: start };
}

/** Place the caret (or selection) at plain-text offsets. */
export function setComposerSelectionOffsets(root: HTMLElement, start: number, end: number = start) {
  const sel = window.getSelection();
  if (!sel) return;
  const a = pointFromRoot(root, Math.max(0, start));
  const b = start === end ? a : pointFromRoot(root, Math.max(0, end));
  if (!a || !b) return;
  const range = document.createRange();
  range.setStart(a.node, a.offset);
  range.setEnd(b.node, b.offset);
  sel.removeAllRanges();
  sel.addRange(range);
}

function offsetFromRoot(root: HTMLElement, target: Node, targetOffset: number): number {
  let total = 0;
  let done = false;

  const walk = (node: Node) => {
    if (done) return;
    if (node === target) {
      if (node.nodeType === Node.TEXT_NODE) {
        total += Math.min(targetOffset, (node.nodeValue ?? "").length);
      } else if (node instanceof HTMLElement && node.dataset.ceToken != null) {
        // Caret on/inside atomic token counts as before (0) or after (1+) the token.
        total += targetOffset > 0 ? node.dataset.ceToken.length : 0;
      } else {
        // Element with child offset index.
        const children = Array.from(node.childNodes);
        for (let i = 0; i < targetOffset && i < children.length; i++) {
          total += nodeTextLength(children[i]!);
        }
      }
      done = true;
      return;
    }
    if (node.nodeType === Node.TEXT_NODE) {
      total += node.nodeValue?.length ?? 0;
      return;
    }
    if (node instanceof HTMLElement && node.dataset.ceToken != null) {
      if (node.contains(target)) {
        total += targetOffset > 0 ? node.dataset.ceToken.length : 0;
        done = true;
        return;
      }
      total += node.dataset.ceToken.length;
      return;
    }
    if (node instanceof HTMLElement && node.tagName === "BR") {
      total += 1;
      return;
    }
    for (const child of Array.from(node.childNodes)) {
      walk(child);
      if (done) return;
    }
  };

  walk(root);
  return total;
}

function nodeTextLength(node: Node): number {
  if (node.nodeType === Node.TEXT_NODE) return node.nodeValue?.length ?? 0;
  if (!(node instanceof HTMLElement)) return 0;
  if (node.dataset.ceToken != null) return node.dataset.ceToken.length;
  if (node.tagName === "BR") return 1;
  let n = 0;
  for (const child of Array.from(node.childNodes)) n += nodeTextLength(child);
  return n;
}

function pointFromRoot(root: HTMLElement, index: number): { node: Node; offset: number } | null {
  let remaining = index;

  const walk = (node: Node): { node: Node; offset: number } | null => {
    if (node.nodeType === Node.TEXT_NODE) {
      const len = node.nodeValue?.length ?? 0;
      if (remaining <= len) return { node, offset: remaining };
      remaining -= len;
      return null;
    }
    if (node instanceof HTMLElement && node.dataset.ceToken != null) {
      const len = node.dataset.ceToken.length;
      if (remaining <= 0) {
        // Before token: position at parent index of token.
        const parent = node.parentNode;
        if (!parent) return { node: root, offset: 0 };
        const idx = Array.from(parent.childNodes).indexOf(node);
        return { node: parent, offset: Math.max(0, idx) };
      }
      if (remaining < len) {
        // Snap to after token (atomic).
        remaining = 0;
        const parent = node.parentNode;
        if (!parent) return { node: root, offset: 0 };
        const idx = Array.from(parent.childNodes).indexOf(node);
        return { node: parent, offset: idx + 1 };
      }
      remaining -= len;
      return null;
    }
    if (node instanceof HTMLElement && node.tagName === "BR") {
      if (remaining <= 0) {
        const parent = node.parentNode;
        if (!parent) return { node: root, offset: 0 };
        const idx = Array.from(parent.childNodes).indexOf(node);
        return { node: parent, offset: idx };
      }
      remaining -= 1;
      return null;
    }
    if (node instanceof HTMLElement) {
      for (const child of Array.from(node.childNodes)) {
        const hit = walk(child);
        if (hit) return hit;
      }
    }
    return null;
  };

  const hit = walk(root);
  if (hit) return hit;
  // EOF
  if (root.lastChild?.nodeType === Node.TEXT_NODE) {
    const text = root.lastChild;
    return { node: text, offset: text.nodeValue?.length ?? 0 };
  }
  return { node: root, offset: root.childNodes.length };
}

/** Build a token mark element. */
export function createComposerTokenElement(meta: ComposerTokenMeta): HTMLSpanElement {
  const mark = document.createElement("span");
  mark.className = `ce-token ce-token-mark ${meta.className}`;
  mark.contentEditable = "false";
  mark.dataset.ceToken = meta.token;
  mark.dataset.ceKind = meta.kind;
  if (meta.title) mark.title = meta.title;

  if (meta.iconUrl) {
    const img = document.createElement("img");
    img.className = "ce-token-icon";
    img.src = meta.iconUrl;
    img.alt = "";
    img.draggable = false;
    img.referrerPolicy = "no-referrer";
    img.addEventListener("error", () => {
      img.remove();
      mark.insertBefore(createFallbackIcon(meta.fallback), mark.firstChild);
    });
    mark.appendChild(img);
  } else {
    mark.appendChild(createFallbackIcon(meta.fallback));
  }

  const label = document.createElement("span");
  label.className = "ce-token-label";
  label.textContent = meta.label;
  mark.appendChild(label);
  return mark;
}

function createFallbackIcon(kind: ComposerTokenMeta["fallback"]): HTMLElement {
  const span = document.createElement("span");
  span.className = `ce-token-fallback ce-token-fallback--${kind}`;
  span.setAttribute("aria-hidden", "true");
  // Compact geometric fallbacks (no Lucide dependency inside DOM builder).
  span.textContent = kind === "folder" ? "▣" : kind === "zap" ? "⚡" : kind === "bot" ? "◉" : "▤";
  return span;
}

/** Replace editor contents from plain text. */
export function renderComposerEditable(
  root: HTMLElement,
  text: string,
  resolveMeta: ResolveComposerTokenMeta,
) {
  const parts = splitInlineTokenParts(text);
  const frag = document.createDocumentFragment();
  for (const part of parts) {
    if (part.kind === "text") {
      if (part.text) frag.appendChild(document.createTextNode(part.text));
      continue;
    }
    frag.appendChild(createComposerTokenElement(resolveMeta(part)));
  }
  // Ensure the field is focusable / caret-friendly when empty.
  if (!frag.childNodes.length) {
    frag.appendChild(document.createTextNode(""));
  }
  root.replaceChildren(frag);
}

/**
 * After a free-form edit, rebuild marks for completed wire tokens while
 * preserving unfinished `@query` / `#partial` as plain text for pickers.
 */
export function normalizeComposerEditable(
  root: HTMLElement,
  resolveMeta: ResolveComposerTokenMeta,
): string {
  const text = serializeComposerEditable(root);
  const caret = getComposerSelectionOffsets(root);
  renderComposerEditable(root, text, resolveMeta);
  setComposerSelectionOffsets(root, caret.start, caret.end);
  return text;
}

/** True when `text` has at least one complete inline token. */
export function composerTextHasTokens(text: string): boolean {
  const re = new RegExp(COMPOSER_INLINE_TOKEN_RE.source, "g");
  return re.test(text);
}

export type { InlineTokenPart };
