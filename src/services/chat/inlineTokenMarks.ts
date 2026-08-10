/**
 * Markdown-like inline marks for `@file`, `#skill:id`, and `#mcp:id`.
 * Typography only (weight / color) — not chip / pill chrome.
 */

import {
  COMPOSER_INLINE_TOKEN_RE,
  isDirMention,
  mentionDisplayLabel,
} from "@/services/chat/composerSegments";

export type InlineTokenPart =
  | { kind: "text"; text: string }
  | { kind: "mention"; path: string; name: string; isDir: boolean; raw: string }
  | { kind: "skill"; id: string; raw: string }
  | { kind: "mcp"; id: string; raw: string };

/** Escape text for safe use inside highlight HTML. */
export function escapeInlineHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/** Split serialized composer / user-message text into plain + token parts. */
export function splitInlineTokenParts(text: string): InlineTokenPart[] {
  const input = text ?? "";
  if (!input) return [{ kind: "text", text: "" }];

  const parts: InlineTokenPart[] = [];
  const re = new RegExp(COMPOSER_INLINE_TOKEN_RE.source, "g");
  const mentionPaths: string[] = [];
  let match: RegExpExecArray | null;

  while ((match = re.exec(input)) !== null) {
    if (!(match[3] && match[4])) {
      mentionPaths.push(match[1] || match[2] || "");
    }
  }

  re.lastIndex = 0;
  let lastIndex = 0;
  while ((match = re.exec(input)) !== null) {
    if (match.index > lastIndex) {
      parts.push({ kind: "text", text: input.slice(lastIndex, match.index) });
    }
    const raw = match[0];
    if (match[3] && match[4]) {
      const kind = match[3] as "skill" | "mcp";
      parts.push({ kind, id: match[4], raw });
    } else {
      const path = match[1] || match[2] || "";
      const isDir = isDirMention(path);
      const name = mentionDisplayLabel(path, { isDir, catalog: mentionPaths });
      parts.push({ kind: "mention", path, name, isDir, raw });
    }
    lastIndex = match.index + match[0].length;
  }
  if (lastIndex < input.length) {
    parts.push({ kind: "text", text: input.slice(lastIndex) });
  }
  return parts.length > 0 ? parts : [{ kind: "text", text: input }];
}

/**
 * Mirror-layer HTML for a transparent textarea.
 * Character lengths must match the source text (no logos) so caret stays aligned.
 */
export function renderInlineTokenHighlightHtml(text: string): string {
  const parts = splitInlineTokenParts(text);
  return parts
    .map((part) => {
      if (part.kind === "text") return escapeInlineHtml(part.text);
      if (part.kind === "mention") {
        const cls = part.isDir
          ? "inline-token inline-token-file is-dir"
          : "inline-token inline-token-file";
        return `<span class="${cls}">${escapeInlineHtml(part.raw)}</span>`;
      }
      if (part.kind === "skill") {
        return `<span class="inline-token inline-token-skill">${escapeInlineHtml(part.raw)}</span>`;
      }
      return `<span class="inline-token inline-token-mcp">${escapeInlineHtml(part.raw)}</span>`;
    })
    .join("");
}
