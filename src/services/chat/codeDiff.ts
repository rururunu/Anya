import { ipcInvoke } from "@/services/ipc/commands";
import { IPC_COMMANDS } from "@/types/ipc";

export type CodeDiffLineKind = "context" | "addition" | "deletion" | "skip";

export interface CodeDiffInlineRange {
  from: number;
  to: number;
}

export interface CodeDiffLine {
  lineNumber: number;
  text: string;
  kind: CodeDiffLineKind;
  skipped?: number;
}

export interface CodeDiffRow {
  left: CodeDiffLine | null;
  right: CodeDiffLine | null;
}

export interface CodeDiffDocument {
  rows: CodeDiffRow[];
}

export interface CodeDiffRequest {
  oldText?: string | null;
  newText?: string | null;
  unifiedDiff: string;
}

export async function buildCodeDiff(request: CodeDiffRequest): Promise<CodeDiffDocument> {
  let document: CodeDiffDocument | null = null;
  try {
    document = await ipcInvoke<CodeDiffDocument>(IPC_COMMANDS.buildCodeDiff, { request });
    if (!document.rows.length && request.unifiedDiff.trim()) {
      document = null;
    }
  } catch (error) {
    console.warn("Rust code diff unavailable; rendering unified diff locally.", error);
  }

  const rows = document?.rows ?? parseUnifiedDiff(request.unifiedDiff).rows;
  return { rows: collapseUnchangedRows(rows) };
}

function parseUnifiedDiff(diff: string): CodeDiffDocument {
  const rows: CodeDiffRow[] = [];
  const deletions: CodeDiffLine[] = [];
  const additions: CodeDiffLine[] = [];
  let oldLine = 0;
  let newLine = 0;

  const flushChanges = () => {
    const count = Math.max(deletions.length, additions.length);
    for (let index = 0; index < count; index += 1) {
      rows.push({ left: deletions[index] ?? null, right: additions[index] ?? null });
    }
    deletions.length = 0;
    additions.length = 0;
  };

  for (const raw of diff.replace(/\r\n/g, "\n").split("\n")) {
    const hunk = raw.match(/^@@\s+-(\d+)(?:,\d+)?\s+\+(\d+)(?:,\d+)?\s+@@/);
    if (hunk) {
      flushChanges();
      oldLine = Number(hunk[1]);
      newLine = Number(hunk[2]);
      continue;
    }
    if (
      raw.startsWith("--- ") ||
      raw.startsWith("+++ ") ||
      raw.startsWith("diff ") ||
      raw.startsWith("index ")
    ) {
      continue;
    }
    if (raw.startsWith("-")) {
      deletions.push({ lineNumber: oldLine, text: raw.slice(1), kind: "deletion" });
      oldLine += 1;
      continue;
    }
    if (raw.startsWith("+")) {
      additions.push({ lineNumber: newLine, text: raw.slice(1), kind: "addition" });
      newLine += 1;
      continue;
    }
    if (raw.startsWith(" ")) {
      flushChanges();
      const text = raw.slice(1);
      rows.push({
        left: { lineNumber: oldLine, text, kind: "context" },
        right: { lineNumber: newLine, text, kind: "context" },
      });
      oldLine += 1;
      newLine += 1;
    }
  }

  flushChanges();
  return { rows };
}

const CONTEXT_LINES = 3;
const MIN_SKIP = 2;

function isChangedRow(row: CodeDiffRow): boolean {
  return row.left?.kind === "deletion" || row.right?.kind === "addition";
}

function skipRow(count: number): CodeDiffRow {
  const line: CodeDiffLine = { lineNumber: 0, text: "", kind: "skip", skipped: count };
  return { left: { ...line }, right: { ...line } };
}

/** Fold long unchanged stretches so the review shows hunks, not the whole file. */
export function collapseUnchangedRows(rows: CodeDiffRow[], context = CONTEXT_LINES): CodeDiffRow[] {
  if (!rows.some(isChangedRow)) return rows;
  const out: CodeDiffRow[] = [];
  let index = 0;
  while (index < rows.length) {
    if (isChangedRow(rows[index]!)) {
      out.push(rows[index]!);
      index += 1;
      continue;
    }
    let end = index;
    while (end < rows.length && !isChangedRow(rows[end]!)) end += 1;
    const run = rows.slice(index, end);
    const leading = index === 0;
    const trailing = end === rows.length;
    const keepStart = leading ? 0 : Math.min(context, run.length);
    const keepEnd = trailing ? 0 : Math.min(context, run.length - keepStart);
    const skipped = run.length - keepStart - keepEnd;
    if (skipped >= MIN_SKIP) {
      out.push(...run.slice(0, keepStart));
      out.push(skipRow(skipped));
      out.push(...run.slice(run.length - keepEnd));
    } else {
      out.push(...run);
    }
    index = end;
  }
  return out;
}

/** Highlight the middle of a replaced line; skip when the whole line changed. */
export function inlineEditRange(
  before: string,
  after: string,
): { before: CodeDiffInlineRange | null; after: CodeDiffInlineRange | null } {
  if (!before || !after || before === after) {
    return { before: null, after: null };
  }
  let prefix = 0;
  const limit = Math.min(before.length, after.length);
  while (prefix < limit && before[prefix] === after[prefix]) prefix += 1;
  let suffix = 0;
  while (
    suffix < limit - prefix &&
    before[before.length - 1 - suffix] === after[after.length - 1 - suffix]
  ) {
    suffix += 1;
  }
  const beforeTo = before.length - suffix;
  const afterTo = after.length - suffix;
  if (prefix >= beforeTo && prefix >= afterTo) {
    return { before: null, after: null };
  }
  return {
    before:
      prefix < beforeTo && beforeTo - prefix < before.length
        ? { from: prefix, to: beforeTo }
        : null,
    after:
      prefix < afterTo && afterTo - prefix < after.length ? { from: prefix, to: afterTo } : null,
  };
}

/** Intra-line ranges for a replacement row (deletion on the left, addition on the right). */
export function inlineRangesForRow(row: CodeDiffRow): {
  left?: CodeDiffInlineRange;
  right?: CodeDiffInlineRange;
} {
  if (row.left?.kind !== "deletion" || row.right?.kind !== "addition") return {};
  const ranges = inlineEditRange(row.left.text, row.right.text);
  return {
    left: ranges.before ?? undefined,
    right: ranges.after ?? undefined,
  };
}
