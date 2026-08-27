export type DiffLineKind = "context" | "addition" | "deletion";

export type DiffLine = {
  kind: DiffLineKind;
  oldNo?: number;
  newNo?: number;
  text: string;
};

export type DiffHunk = {
  oldStart: number;
  newStart: number;
  lines: DiffLine[];
  added: number;
  removed: number;
};

/** Parse a unified diff into interleaved hunks with line numbers. */
export function parseUnifiedDiffHunks(diff: string): DiffHunk[] {
  const hunks: DiffHunk[] = [];
  let current: DiffHunk | null = null;
  let oldNo = 0;
  let newNo = 0;

  const flush = () => {
    if (current && current.lines.length) {
      hunks.push(current);
    }
    current = null;
  };

  for (const raw of diff.replace(/\r\n/g, "\n").split("\n")) {
    const header = /^@@\s+-(\d+)(?:,\d+)?\s+\+(\d+)(?:,\d+)?\s+@@/.exec(raw);
    if (header) {
      flush();
      oldNo = Number(header[1]);
      newNo = Number(header[2]);
      current = {
        oldStart: oldNo,
        newStart: newNo,
        lines: [],
        added: 0,
        removed: 0,
      };
      continue;
    }
    if (
      raw.startsWith("--- ") ||
      raw.startsWith("+++ ") ||
      raw.startsWith("diff ") ||
      raw.startsWith("index ") ||
      raw.startsWith("\\")
    ) {
      continue;
    }
    if (!current) {
      current = { oldStart: 1, newStart: 1, lines: [], added: 0, removed: 0 };
      oldNo = 1;
      newNo = 1;
    }
    if (raw.startsWith("-")) {
      current.lines.push({ kind: "deletion", oldNo, text: raw.slice(1) });
      current.removed += 1;
      oldNo += 1;
      continue;
    }
    if (raw.startsWith("+")) {
      current.lines.push({ kind: "addition", newNo, text: raw.slice(1) });
      current.added += 1;
      newNo += 1;
      continue;
    }
    if (raw.startsWith(" ") || raw === "") {
      const text = raw.startsWith(" ") ? raw.slice(1) : raw;
      current.lines.push({ kind: "context", oldNo, newNo, text });
      oldNo += 1;
      newNo += 1;
    }
  }
  flush();
  return hunks;
}

/** Build a single hunk from plain old/new text (no context). */
export function hunkFromPlainEdit(oldText: string, newText: string): DiffHunk {
  const oldLines = splitLines(oldText);
  const newLines = splitLines(newText);
  const lines: DiffLine[] = [
    ...oldLines.map((text, index) => ({
      kind: "deletion" as const,
      oldNo: index + 1,
      text,
    })),
    ...newLines.map((text, index) => ({
      kind: "addition" as const,
      newNo: index + 1,
      text,
    })),
  ];
  return {
    oldStart: 1,
    newStart: 1,
    lines,
    added: newLines.length,
    removed: oldLines.length,
  };
}

function splitLines(value: string): string[] {
  if (!value.length) return [];
  const lines = value.replace(/\r\n/g, "\n").split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  return lines;
}

export function fileBasename(path: string | undefined | null): string {
  const normalized = (path ?? "").replace(/\\/g, "/");
  return normalized.split("/").pop() || path || "file";
}

export function fileParentDir(path: string | undefined | null): string {
  const normalized = (path ?? "").replace(/\\/g, "/");
  const separator = normalized.lastIndexOf("/");
  return separator > 0 ? normalized.slice(0, separator) : "";
}
