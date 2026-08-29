import type { TaskItem, ToolActivity } from "@/types/chat";
import { hunkFromPlainEdit, parseUnifiedDiffHunks, type DiffHunk } from "@/services/chat/toolDiff";

const FILE_OPERATION_KINDS = new Set(["create", "edit", "delete", "move"]);
const TASK_LIST_TOOLS = new Set(["update_tasks", "todo_write"]);

export type EnrichedToolActivity = {
  activity: ToolActivity;
  hunks: DiffHunk[];
  tasks: TaskItem[];
  filePath: string;
};

/** Map raw tool activities to display rows with diff hunks and task lists. */
export function enrichToolActivities(
  activities: ToolActivity[],
  options: { nested?: boolean; flat?: boolean },
): EnrichedToolActivity[] {
  return activities
    .filter((activity) => {
      if (activity.kind === "read" && !options.nested && !options.flat) return false;
      return !(activity.toolName === "ask_user" && activity.status !== "running");
    })
    .map((activity) => {
      const hunks = collectActivityHunks(activity);
      const filePath = String(activity.preview?.path ?? activity.arguments?.path ?? "");
      return {
        activity,
        hunks,
        tasks: tasksFromActivity(activity),
        filePath: filePath || "file",
      };
    });
}

/** Extract task list items from update_tasks / todo_write tool arguments. */
export function tasksFromActivity(activity: ToolActivity): TaskItem[] {
  if (!TASK_LIST_TOOLS.has(activity.toolName)) return [];
  const raw = activity.arguments?.tasks;
  if (!Array.isArray(raw)) return [];
  return raw.flatMap((value) => {
    if (!value || typeof value !== "object") return [];
    const item = value as Record<string, unknown>;
    const content = String(item.content ?? "").trim();
    if (!content) return [];
    const activeForm =
      typeof item.activeForm === "string"
        ? item.activeForm
        : typeof item.active_form === "string"
          ? item.active_form
          : undefined;
    const level = typeof item.level === "number" ? item.level : undefined;
    return [
      {
        content,
        status: String(item.status ?? "pending"),
        activeForm,
        level,
      },
    ];
  });
}

/** Collect diff hunks for a file-operation tool activity. */
export function collectActivityHunks(activity: ToolActivity): DiffHunk[] {
  if (activity.kind === "shell") return [];
  const args = activity.arguments ?? {};
  const preview = activity.preview;
  if (preview?.unifiedDiff?.trim()) {
    return parseUnifiedDiffHunks(preview.unifiedDiff);
  }
  if (activity.toolName === "replace_many_in_file" && Array.isArray(args.edits)) {
    return args.edits.map((value) => {
      const edit = value as Record<string, unknown>;
      return hunkFromPlainEdit(String(edit.old_string ?? ""), String(edit.new_string ?? ""));
    });
  }
  if (preview && (preview.oldText != null || preview.newText != null)) {
    return [hunkFromPlainEdit(preview.oldText ?? "", preview.newText ?? "")];
  }
  if (activity.toolName === "replace_in_file") {
    return [hunkFromPlainEdit(String(args.old_string ?? ""), String(args.new_string ?? ""))];
  }
  if (activity.toolName === "write_file") {
    return [hunkFromPlainEdit("", String(args.content ?? ""))];
  }
  if (activity.kind === "delete") {
    const deleted = args.old_string ?? args.symbol ?? args.start_anchor ?? activity.detail ?? "";
    return [hunkFromPlainEdit(String(deleted), "")];
  }
  if (!FILE_OPERATION_KINDS.has(activity.kind) && !preview) return [];
  return [];
}

export function isImageGenActivity(activity: ToolActivity) {
  return activity.kind === "image" || activity.toolName === "generate_image";
}
