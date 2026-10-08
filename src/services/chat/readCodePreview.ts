import type { ToolActivity } from "@/types/chat";

export interface ReadCodePreview {
  activityId: string;
  path: string;
  startLine: number;
  endLine: number;
  content: string;
}

/** Decode the actual returned window, rather than guessing from requested limits. */
export function readCodePreview(activity: ToolActivity): ReadCodePreview | null {
  if (
    !["read", "read_file"].includes(activity.toolName) ||
    activity.status === "running" ||
    activity.success === false
  )
    return null;
  const result = activity.result ?? "";
  const path =
    result.match(/^<path>([^\n]+)<\/path>/)?.[1] ??
    String(activity.arguments?.file_path ?? activity.arguments?.path ?? "");
  if (!path || /\.(?:docx?|xlsx?|pptx?|pdf|png|jpe?g|webp|gif)$/i.test(path)) return null;
  const body = result.includes("<content>\n")
    ? result.slice(result.indexOf("<content>\n") + 10).split("\n</content>")[0]
    : result;
  const lines = body
    .split(/\r?\n/)
    .map((line) => {
      const match = line.match(/^\s*(\d+)(?:: |\|)(.*)$/);
      return match ? { number: Number(match[1]), text: match[2] } : null;
    })
    .filter((line): line is { number: number; text: string } => Boolean(line));
  if (!lines.length) return null;
  // A numbered code string inside the content remains content, not a second row.
  const continuous = lines.every(
    (line, index) => index === 0 || line.number === lines[index - 1].number + 1,
  );
  if (!continuous) return null;
  return {
    activityId: activity.id,
    path,
    startLine: lines[0].number,
    endLine: lines[lines.length - 1].number,
    content: lines.map((line) => line.text).join("\n"),
  };
}
