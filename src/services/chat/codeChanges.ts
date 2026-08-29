import type { ChatMessage, ToolActivity, ToolPreviewPayload } from "@/types/chat";

export interface CodeChangeEntry {
  id: string;
  path: string;
  diff: string;
  oldText?: string | null;
  newText?: string | null;
  added: number;
  removed: number;
}

export function extractCodeChanges(messages: ChatMessage[]): CodeChangeEntry[] {
  const changes: CodeChangeEntry[] = [];
  for (const message of messages) {
    for (const activity of message.toolActivities ?? []) {
      if (activity.status !== "done" || !activity.success) continue;
      changes.push(...changesForActivity(message.id, activity));
    }
  }
  return mergeChangesByPath(changes);
}

function changesForActivity(messageId: string, activity: ToolActivity): CodeChangeEntry[] {
  const preview = activity.preview;
  if (!preview) return changesFromArguments(messageId, activity);
  const diff = previewDiff(preview);
  if (!diff.trim()) return [];

  const sections = splitUnifiedDiff(diff, preview.path || sanitizeDiffPath(activity.title));
  return sections.map((section, index) => ({
    id: `${messageId}:${activity.id}:${index}`,
    path: section.path,
    diff: section.diff,
    // A preview describes one file. Multi-file patches intentionally use the
    // unified-diff fallback in Rust so no unrelated text is paired together.
    oldText: sections.length === 1 ? preview.oldText : undefined,
    newText: sections.length === 1 ? preview.newText : undefined,
    ...countChanges(section.diff),
  }));
}

function changesFromArguments(messageId: string, activity: ToolActivity): CodeChangeEntry[] {
  const args = activity.arguments ?? {};
  const path = typeof args.path === "string" ? args.path : "";
  if (activity.toolName === "apply_patch") {
    const input = typeof args.input === "string" ? args.input : "";
    return changesFromPatchInput(messageId, activity.id, input);
  }
  if (activity.toolName === "replace_in_file" && path) {
    return fallbackEntry(messageId, activity.id, path, args.old_string, args.new_string);
  }
  if (activity.toolName === "replace_many_in_file" && path && Array.isArray(args.edits)) {
    return args.edits.flatMap((value, index) => {
      const edit = value as Record<string, unknown>;
      return fallbackEntry(
        messageId,
        `${activity.id}:${index}`,
        path,
        edit.old_string,
        edit.new_string,
      );
    });
  }
  if (activity.toolName === "write_file" && path) {
    return fallbackEntry(messageId, activity.id, path, null, args.content);
  }
  return [];
}

function changesFromPatchInput(messageId: string, activityId: string, input: string) {
  const lines = input.replace(/\r\n/g, "\n").split("\n");
  const entries: CodeChangeEntry[] = [];
  let path = "";
  let body: string[] = [];

  const flush = () => {
    if (!path) return;
    const changedLines = body.filter(
      (line) =>
        line.startsWith("@@") ||
        line.startsWith("+") ||
        line.startsWith("-") ||
        line.startsWith(" "),
    );
    const diff = [`--- a/${path}`, `+++ b/${path}`, ...changedLines].join("\n");
    entries.push({
      id: `${messageId}:${activityId}:${entries.length}`,
      path,
      diff,
      ...countChanges(diff),
    });
  };

  for (const line of lines) {
    const header = line.match(/^\*\*\* (?:Add|Update|Delete) File:\s*(.+)$/);
    if (header) {
      flush();
      path = header[1]?.trim() ?? "";
      body = [];
      continue;
    }
    if (line === "*** End Patch") {
      flush();
      path = "";
      body = [];
      continue;
    }
    if (path) body.push(line);
  }
  if (path) flush();
  return entries;
}

function fallbackEntry(
  messageId: string,
  activityId: string,
  path: string,
  oldValue: unknown,
  newValue: unknown,
): CodeChangeEntry[] {
  const oldText = typeof oldValue === "string" ? oldValue : "";
  const newText = typeof newValue === "string" ? newValue : "";
  if (!oldText && !newText) return [];
  const diff = [
    `--- a/${path}`,
    `+++ b/${path}`,
    "@@ -1 +1 @@",
    ...toLines(oldText).map((line) => `-${line}`),
    ...toLines(newText).map((line) => `+${line}`),
  ].join("\n");
  return [
    { id: `${messageId}:${activityId}`, path, diff, oldText, newText, ...countChanges(diff) },
  ];
}

function previewDiff(preview: ToolPreviewPayload) {
  if (preview.unifiedDiff?.trim()) return preview.unifiedDiff;
  if (preview.oldText == null && preview.newText == null) return "";
  const oldLines = toLines(preview.oldText).map((line) => `-${line}`);
  const newLines = toLines(preview.newText).map((line) => `+${line}`);
  return [
    `--- a/${preview.path}`,
    `+++ b/${preview.path}`,
    "@@ -1 +1 @@",
    ...oldLines,
    ...newLines,
  ].join("\n");
}

function splitUnifiedDiff(diff: string, fallbackPath: string) {
  const lines = diff.replace(/\r\n/g, "\n").split("\n");
  const starts: number[] = [];
  for (let index = 0; index < lines.length - 1; index += 1) {
    if (lines[index]?.startsWith("--- ") && lines[index + 1]?.startsWith("+++ ")) {
      starts.push(index);
    }
  }
  if (!starts.length) return [{ path: fallbackPath, diff }];

  return starts.map((start, index) => {
    const end = starts[index + 1] ?? lines.length;
    const sectionLines = lines.slice(start, end);
    const path = diffPath(sectionLines[1] ?? "", fallbackPath);
    return { path, diff: sectionLines.join("\n").trimEnd() };
  });
}

function diffPath(header: string, fallback: string) {
  const raw =
    header
      .replace(/^\+\+\+\s+/, "")
      .split("\t", 1)[0]
      ?.trim() ?? "";
  if (!raw || raw === "/dev/null") return sanitizeDiffPath(fallback);
  return sanitizeDiffPath(raw.replace(/^[ab]\//, ""));
}

export function sanitizeDiffPath(path: string): string {
  return path
    .trim()
    .replace(/^(?:Write|Edit|Read|Delete|Move)\s+/i, "")
    .replace(/^[ab]\//, "")
    .replace(/^\/+/, "");
}

export function findActivityForChange(
  change: CodeChangeEntry,
  messages: ChatMessage[],
): ToolActivity | undefined {
  for (const message of messages) {
    for (const activity of message.toolActivities ?? []) {
      const prefix = `${message.id}:${activity.id}`;
      if (change.id === prefix || change.id.startsWith(`${prefix}:`)) {
        return activity;
      }
    }
  }
  return undefined;
}

export function resolveChangeFilePath(change: CodeChangeEntry, messages: ChatMessage[]): string {
  const activity = findActivityForChange(change, messages);
  const fromPreview = activity?.preview?.path?.trim();
  if (fromPreview) return sanitizeDiffPath(fromPreview);
  const fromArgs = activity?.arguments?.path;
  if (typeof fromArgs === "string" && fromArgs.trim()) {
    return sanitizeDiffPath(fromArgs);
  }
  return sanitizeDiffPath(change.path);
}

function countChanges(diff: string) {
  let added = 0;
  let removed = 0;
  for (const line of diff.replace(/\r\n/g, "\n").split("\n")) {
    if (line.startsWith("+++") || line.startsWith("---")) continue;
    if (line.startsWith("+")) added += 1;
    if (line.startsWith("-")) removed += 1;
  }
  return { added, removed };
}

function mergeChangesByPath(changes: CodeChangeEntry[]) {
  const merged = new Map<string, CodeChangeEntry>();
  for (const change of changes) {
    const key = change.path.replace(/\\/g, "/").toLowerCase();
    const current = merged.get(key);
    if (!current) {
      merged.set(key, change);
      continue;
    }
    current.id = `${current.id}:${change.id}`;
    current.diff = `${current.diff}\n\n${change.diff}`;
    current.oldText = undefined;
    current.newText = undefined;
    current.added += change.added;
    current.removed += change.removed;
  }
  return [...merged.values()];
}

function toLines(value: string | null | undefined) {
  if (!value) return [];
  const lines = value.split(/\r?\n/);
  if (lines[lines.length - 1] === "") lines.pop();
  return lines;
}
