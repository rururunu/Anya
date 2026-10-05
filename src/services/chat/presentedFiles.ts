import type { ChatMessage } from "@/types/chat";

export interface PresentedFile {
  path: string;
  absolutePath: string;
  name: string;
  description?: string;
  size: number;
}

/** Only successful durable declarations produce cards; prose and arguments do not. */
export function presentedFiles(message: ChatMessage): PresentedFile[] {
  const files = new Map<string, PresentedFile>();
  for (const activity of message.toolActivities ?? []) {
    if (
      activity.toolName !== "present" ||
      activity.status !== "done" ||
      !activity.success ||
      activity.subagentId ||
      !activity.result
    )
      continue;
    let data: unknown;
    try {
      data = JSON.parse(activity.result);
    } catch {
      continue;
    }
    if (
      !data ||
      typeof data !== "object" ||
      !("version" in data) ||
      data.version !== 1 ||
      !("files" in data) ||
      !Array.isArray(data.files)
    )
      continue;
    for (const value of data.files) {
      if (!value || typeof value !== "object") continue;
      const file = value as Partial<PresentedFile>;
      if (
        typeof file.path !== "string" ||
        typeof file.absolutePath !== "string" ||
        !/^(?:[A-Za-z]:[\\/]|\\\\|\/)/.test(file.absolutePath) ||
        file.absolutePath.includes("\0") ||
        typeof file.name !== "string" ||
        !file.name ||
        typeof file.size !== "number" ||
        !Number.isFinite(file.size) ||
        file.size < 0 ||
        (file.description != null && typeof file.description !== "string")
      )
        continue;
      const key = /^[A-Za-z]:|^\\\\/.test(file.absolutePath)
        ? file.absolutePath.replace(/\\/g, "/").toLowerCase()
        : file.absolutePath;
      files.set(key, {
        path: file.path,
        absolutePath: file.absolutePath,
        name: file.name,
        size: file.size,
        description: file.description ?? undefined,
      });
    }
  }
  return [...files.values()];
}
