/**
 * Composer chip / segment model used by the chat input bar.
 *
 * Segments sit ahead of the live textarea value and preserve mid-sentence
 * order for mentions, pasted blocks, and selection chips.
 */

export type ComposerSegment =
  | { kind: "text"; text: string }
  | { kind: "mention"; path: string; isDir?: boolean }
  | { kind: "skill"; id: string }
  | { kind: "mcp"; id: string }
  | { kind: "paste"; text: string }
  | { kind: "selection"; lines: number };

/** Count lines in pasted text (treats empty as zero). */
export function pasteLineCount(text: string): number {
  return text ? text.split(/\r\n|\r|\n/).length : 0;
}

/** Normalize separators and strip a trailing slash. */
export function normalizeMentionPath(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "").replace(/^\/+/, "").trim();
}

/** True when the mention refers to a directory (flag or trailing slash). */
export function isDirMention(path: string, isDir?: boolean): boolean {
  if (typeof isDir === "boolean") return isDir;
  return /[/\\]$/.test(path.trim());
}

/**
 * Canonical storage path for a mention.
 * Directories keep a trailing `/` so chips survive serialize → parse.
 */
export function mentionStoragePath(path: string, isDir?: boolean): string {
  const normalized = normalizeMentionPath(path);
  if (!normalized) return "";
  return isDirMention(path, isDir) ? `${normalized}/` : normalized;
}

/** Basename of a mention path (ignores trailing slash). */
export function mentionBasename(path: string): string {
  const normalized = normalizeMentionPath(path);
  return normalized.split("/").pop() || normalized;
}

/**
 * Match `@file`, `#skill:id`, and `#mcp:id` tokens in a serialized composer message.
 * Keep in sync with MessageList inline chip parsing.
 */
export const COMPOSER_INLINE_TOKEN_RE = /@(?:"([^"]+)"|([^\s@#]+))|#(skill|mcp):([A-Za-z0-9_.-]+)/g;

/**
 * Restore composer chips from serialized message text (e.g. after rewind).
 * Chip tokens become frozen segments; trailing plain text stays editable.
 */
export function parseComposerTextToSegments(text: string): {
  segments: ComposerSegment[];
  liveMessage: string;
} {
  const input = text ?? "";
  if (!input) {
    return { segments: [], liveMessage: "" };
  }

  const segments: ComposerSegment[] = [];
  const re = new RegExp(COMPOSER_INLINE_TOKEN_RE.source, "g");
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  let sawChip = false;

  while ((match = re.exec(input)) !== null) {
    if (match.index > lastIndex) {
      const between = input.slice(lastIndex, match.index);
      if (between.trim()) {
        segments.push({ kind: "text", text: between });
      }
    }
    if (match[3] && match[4]) {
      sawChip = true;
      segments.push({ kind: match[3] as "skill" | "mcp", id: match[4] });
    } else {
      const path = match[1] || match[2] || "";
      if (path) {
        sawChip = true;
        const storage = mentionStoragePath(path);
        if (isDirMention(storage)) {
          segments.push({ kind: "mention", path: storage, isDir: true });
        } else {
          segments.push({ kind: "mention", path: storage });
        }
      }
    }
    lastIndex = match.index + match[0].length;
  }

  if (!sawChip) {
    return { segments: [], liveMessage: input };
  }

  return {
    segments,
    liveMessage: input.slice(lastIndex).replace(/^\s+/, ""),
  };
}

/**
 * Label shown on a mention chip.
 * - Directories: always the workspace-relative path (`src/main`)
 * - Files: basename, or full path when another file shares that name
 */
export function mentionDisplayLabel(
  path: string,
  options?: { isDir?: boolean; catalog?: readonly string[] },
): string {
  const normalized = normalizeMentionPath(path);
  if (!normalized) return path;
  if (isDirMention(path, options?.isDir)) {
    return normalized;
  }
  const base = mentionBasename(normalized);
  const catalog = options?.catalog ?? [normalized];
  let sameNameFiles = 0;
  for (const entry of catalog) {
    if (isDirMention(entry)) continue;
    if (mentionBasename(entry) === base) {
      sameNameFiles += 1;
      if (sameNameFiles > 1) return normalized;
    }
  }
  return base;
}

/** Serialize a file/folder path as an @-mention, quoting when it contains spaces. */
export function formatMentionPath(path: string, isDir?: boolean): string {
  const storage = mentionStoragePath(path, isDir);
  return /\s/.test(storage) ? `@"${storage}"` : `@${storage}`;
}

/** Serialize a skill/MCP chip as a `#kind:id` token. */
export function formatResourceMention(kind: "skill" | "mcp", id: string): string {
  return `#${kind}:${id.trim()}`;
}

/**
 * Join composer parts while preserving mid-sentence tag order
 * (no forced blank lines between adjacent chips/text).
 */
export function joinInlineParts(parts: string[]): string {
  let out = "";
  for (const part of parts) {
    if (!part) continue;
    if (!out) {
      out = part;
      continue;
    }
    if (/\s$/.test(out) || /^\s/.test(part)) {
      out += part;
    } else {
      out += ` ${part}`;
    }
  }
  return out;
}

/**
 * Flatten frozen segments plus the live textarea into a single sendable string.
 * Selection chips are omitted — the parent attaches selection separately.
 */
export function serializeComposerSegments(
  segments: readonly ComposerSegment[],
  liveMessage: string,
): string {
  const parts: string[] = [];
  for (const seg of segments) {
    if (seg.kind === "text") {
      parts.push(seg.text);
    } else if (seg.kind === "mention") {
      parts.push(formatMentionPath(seg.path, seg.isDir));
    } else if (seg.kind === "skill") {
      parts.push(formatResourceMention("skill", seg.id));
    } else if (seg.kind === "mcp") {
      parts.push(formatResourceMention("mcp", seg.id));
    } else if (seg.kind === "paste") {
      parts.push(seg.text);
    }
  }
  if (liveMessage) {
    parts.push(liveMessage);
  }
  return joinInlineParts(parts);
}

/**
 * Append a segment, merging adjacent text/paste chips when possible.
 * Returns the next segment list (immutable-friendly for callers that prefer it).
 */
export function appendComposerSegment(
  segments: ComposerSegment[],
  segment: ComposerSegment,
): ComposerSegment[] {
  if (segment.kind === "mention") {
    const path = mentionStoragePath(segment.path, segment.isDir);
    if (!path) return segments;
    segments.push({
      kind: "mention",
      path,
      isDir: isDirMention(path, segment.isDir) || undefined,
    });
    return segments;
  }
  if (segment.kind === "text") {
    if (!segment.text) return segments;
    const last = segments[segments.length - 1];
    if (last?.kind === "text") {
      last.text = joinInlineParts([last.text, segment.text]);
      return segments;
    }
  }
  if (segment.kind === "paste") {
    const last = segments[segments.length - 1];
    if (last?.kind === "paste") {
      last.text = `${last.text}\n${segment.text}`;
      return segments;
    }
  }
  segments.push(segment);
  return segments;
}

/**
 * Move trailing typed text into a frozen text segment so a new chip can sit after it.
 * Mutates `segments` and clears `liveMessage` via the returned value.
 */
export function flushLiveMessageToSegments(
  segments: ComposerSegment[],
  liveMessage: string,
): { segments: ComposerSegment[]; liveMessage: string } {
  if (!liveMessage) {
    return { segments, liveMessage: "" };
  }
  const last = segments[segments.length - 1];
  if (last?.kind === "text") {
    last.text = joinInlineParts([last.text, liveMessage]);
  } else {
    segments.push({ kind: "text", text: liveMessage });
  }
  return { segments, liveMessage: "" };
}

/** Plain text (or short paste) that can be merged back into the live textarea. */
export function isEditableTextSegment(seg: ComposerSegment): boolean {
  if (seg.kind === "text") return Boolean(seg.text);
  if (seg.kind === "paste") return pasteLineCount(seg.text) <= 5 && Boolean(seg.text);
  return false;
}

export function editableTextOf(seg: ComposerSegment): string {
  if (seg.kind === "text" || seg.kind === "paste") return seg.text;
  return "";
}
