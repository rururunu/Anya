import type { ChatMessage } from "@/types/chat";

export function isCompactionSummary(
  message: Pick<ChatMessage, "id" | "content"> | { id?: string; content?: string },
): boolean {
  return (
    Boolean(message.id?.startsWith("compact-")) ||
    Boolean(message.content?.includes("<compaction-summary>"))
  );
}

export function parseContextCompactedStatus(kind: string): {
  folded: number;
  usageRatio?: number;
  estimatedTokens?: number;
  contextWindowTokens?: number;
} | null {
  if (!kind.startsWith("context_compacted")) {
    return null;
  }
  const parts = kind.split(":");
  const folded = Number.parseInt(parts[1] ?? "0", 10);
  const usageRatio = parts[2] ? Number.parseFloat(parts[2]) : undefined;
  const estimatedTokens = parts[3] ? Number.parseInt(parts[3], 10) : undefined;
  const contextWindowTokens = parts[4] ? Number.parseInt(parts[4], 10) : undefined;
  return {
    folded: Number.isFinite(folded) ? folded : 0,
    usageRatio: Number.isFinite(usageRatio) ? usageRatio : undefined,
    estimatedTokens: Number.isFinite(estimatedTokens) ? estimatedTokens : undefined,
    contextWindowTokens: Number.isFinite(contextWindowTokens) ? contextWindowTokens : undefined,
  };
}
