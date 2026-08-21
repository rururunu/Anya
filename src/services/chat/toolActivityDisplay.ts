import type { ToolActivity } from "@/types/chat";
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
