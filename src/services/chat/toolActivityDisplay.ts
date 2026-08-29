import type { ToolActivity, WorkTimelineItem } from "@/types/chat";
import type { AppLanguage } from "@/types/setting";
import { tr } from "@/services/i18n";
import { SUBAGENT_TOOLS } from "@/services/chat/subagentTools";

const HIDE_RESULT_TOOLS = new Set([
  "read_file",
  "list_folder",
  "find_files",
  "search_files",
  "list_symbols",
  "fetch_url",
]);

const FILE_TOOLS = new Set([
  "read_file",
  "list_folder",
  "find_files",
  "read_chat",
  "read_shell_output",
  "get_context",
  "get_workspace",
]);

const SEARCH_TOOLS = new Set([
  "search_files",
  "grep",
  "list_symbols",
  "fetch_url",
  "search_memory",
  "search_chats",
]);

export function shouldShowActivityResult(activity: ToolActivity): boolean {
  return Boolean(
    activity.result && activity.status !== "running" && !HIDE_RESULT_TOOLS.has(activity.toolName),
  );
}

export function hasExpandableActivityContent(
  activity: ToolActivity,
  options: {
    childCount?: number;
    showSubagentDetails?: boolean;
  } = {},
): boolean {
  const childCount = options.childCount ?? 0;
  const showSubagentDetails = options.showSubagentDetails ?? false;

  if (SUBAGENT_TOOLS.has(activity.toolName)) {
    if (childCount > 0) return true;
    if (showSubagentDetails) {
      return Boolean(activity.detail?.trim() || shouldShowActivityResult(activity));
    }
    return false;
  }

  if (activity.detail?.trim()) return true;
  if (shouldShowActivityResult(activity)) return true;
  if (childCount > 0) return true;
  return false;
}

function categorizeActivity(activity: ToolActivity): "file" | "search" | "other" {
  if (SEARCH_TOOLS.has(activity.toolName)) return "search";
  if (FILE_TOOLS.has(activity.toolName)) return "file";
  if (/^(search|grep|find)\b/i.test(activity.title)) return "search";
  if (/^read\b/i.test(activity.title)) return "file";
  if (activity.kind === "read") return "file";
  return "other";
}

export function summarizeProcessActivities(
  activities: ToolActivity[],
  language: AppLanguage,
): string {
  if (activities.length === 0) return tr(language, "processSummary");
  if (activities.length === 1) {
    const activity = activities[0];
    return activity?.title?.trim() || activity?.toolName?.trim() || tr(language, "processSummary");
  }

  let files = 0;
  let searches = 0;
  let others = 0;

  for (const activity of activities) {
    const category = categorizeActivity(activity);
    if (category === "file") files += 1;
    else if (category === "search") searches += 1;
    else others += 1;
  }

  const parts: string[] = [];
  if (files > 0) parts.push(tr(language, "processExploredFiles", { count: files }));
  if (searches > 0) parts.push(tr(language, "processExploredSearches", { count: searches }));
  if (others > 0) parts.push(tr(language, "processExploredSteps", { count: others }));

  if (parts.length === 1) return parts[0]!;
  if (parts.length === 2) {
    return tr(language, "processExploredPair", { first: parts[0]!, second: parts[1]! });
  }
  return parts.join(language.startsWith("zh") ? "，" : ", ");
}

/** How many distinct reasoning chunks occurred in this assistant turn. */
export function countReasoningSegments(
  workTimeline: WorkTimelineItem[] | undefined,
  reasoning: string | undefined,
): number {
  const fromTimeline =
    workTimeline?.filter((item) => item.type === "reasoning" && item.content.trim()).length ?? 0;
  if (fromTimeline > 0) return fromTimeline;
  return reasoning?.trim() ? 1 : 0;
}

/** Compact stats for the completed work fold header (tools / searches / thinking). */
export function summarizeWorkFoldMeta(
  activities: ToolActivity[],
  reasoningSegmentCount: number,
  language: AppLanguage,
): string {
  const tools = activities.filter((activity) => activity.toolName !== "ask_user");
  let searchCount = 0;
  for (const activity of tools) {
    if (categorizeActivity(activity) === "search") searchCount += 1;
  }

  const parts: string[] = [];
  if (tools.length > 0) {
    parts.push(tr(language, "turnStatsTools", { count: tools.length }));
  }
  if (searchCount > 0) {
    parts.push(tr(language, "processExploredSearches", { count: searchCount }));
  }
  if (reasoningSegmentCount > 0) {
    parts.push(tr(language, "thinkingCount", { count: reasoningSegmentCount }));
  }

  if (parts.length === 0) return "";
  return parts.join(" · ");
}

function basename(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

/** Short variant label for disclosure rows (Think / Tool row title slot). */
export function toolVariantLabel(activity: ToolActivity, language: AppLanguage): string {
  if (activity.kind === "shell" || activity.toolName === "run_command") {
    return tr(language, "toolVariantShell");
  }
  if (activity.kind === "image" || activity.toolName === "generate_image") {
    return tr(language, "toolVariantImage");
  }
  if (SEARCH_TOOLS.has(activity.toolName) || activity.kind === "search") {
    return tr(language, "toolVariantSearch");
  }
  if (FILE_TOOLS.has(activity.toolName) || activity.kind === "read") {
    return tr(language, "toolVariantRead");
  }
  if (activity.kind === "create" || activity.toolName === "write_file") {
    return tr(language, "toolVariantWrite");
  }
  if (activity.kind === "edit" || activity.kind === "delete" || activity.kind === "move") {
    return tr(language, "toolVariantEdit");
  }
  if (SUBAGENT_TOOLS.has(activity.toolName)) {
    return tr(language, "toolVariantAgent");
  }
  return tr(language, "toolVariantTool");
}

/** Ellipsized summary for collapsed tool rows. */
export function activitySummaryLine(activity: ToolActivity): string {
  const args = activity.arguments ?? {};
  const path = String(
    activity.preview?.path ??
      args.path ??
      args.AbsolutePath ??
      args.TargetFile ??
      args.file_path ??
      "",
  ).trim();
  if (path) return basename(path);

  const query = String(args.query ?? args.Query ?? args.pattern ?? "").trim();
  if (query) return query;

  const command = String(args.CommandLine ?? args.command ?? args.commandLine ?? "").trim();
  if (command) return command;

  const title = activity.title?.trim();
  if (title) {
    const sep = title.indexOf(" ");
    return sep === -1 ? title : title.slice(sep + 1).trim() || title;
  }

  const detail = activity.detail?.trim();
  if (detail) {
    const line = detail.split("\n")[0]?.trim();
    if (line) return line.length > 120 ? `${line.slice(0, 117)}...` : line;
  }

  return activity.toolName?.trim() || "";
}

export function isProcessSegmentCollapsible(
  activities: ToolActivity[],
  activityPool: ToolActivity[],
  options: { showSubagentDetails?: boolean } = {},
): boolean {
  if (activities.length > 1) return true;
  const activity = activities[0];
  if (!activity) return false;
  const childCount = activityPool.filter((item) => item.parentActivityId === activity.id).length;
  return hasExpandableActivityContent(activity, {
    childCount,
    showSubagentDetails: options.showSubagentDetails,
  });
}
