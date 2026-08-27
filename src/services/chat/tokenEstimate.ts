import type { ChatMessage } from "@/types/chat";

const CHARS_PER_TOKEN = 4;
const IMAGE_TOKEN_ESTIMATE = 1000;

export function estimateTextTokens(text: string | undefined): number {
  if (!text) return 0;

  const imageCount = text.match(/data:image\//g)?.length ?? 0;
  const countableText =
    imageCount > 0 ? text.replace(/data:image\/[^)]+/g, "image_placeholder") : text;
  const characters = [...countableText].length;
  const textTokens = characters > 0 ? Math.max(1, Math.floor(characters / CHARS_PER_TOKEN)) : 0;
  return textTokens + imageCount * IMAGE_TOKEN_ESTIMATE;
}

export function estimateMessageTokens(message: ChatMessage): number {
  if (message.estimatedTokens != null && Number.isFinite(message.estimatedTokens)) {
    return message.estimatedTokens;
  }

  let total = estimateTextTokens(message.content) + estimateTextTokens(message.reasoning);

  for (const activity of message.toolActivities ?? []) {
    total += estimateTextTokens(activity.toolName);
    total += estimateTextTokens(activity.title);
    total += estimateTextTokens(activity.detail);
    total += estimateTextTokens(activity.result);
    if (activity.arguments) total += estimateTextTokens(JSON.stringify(activity.arguments));
  }

  return total + 4;
}

/** Trim to at most one decimal, dropping trailing `.0`. */
function compactNumber(value: number): string {
  const rounded = Math.round(value * 10) / 10;
  return Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(1);
}

/**
 * Always use Latin compact units (k / M), never locale 「万」.
 * Under 1,000,000 → k; 1,000,000+ → M.
 */
export function formatTokenCount(tokens: number, _language?: string): string {
  const n = Math.max(0, Math.round(Number.isFinite(tokens) ? tokens : 0));
  if (n < 1_000) return String(n);
  if (n < 1_000_000) {
    // Keep the k band strictly below 1000k (avoid rounding 999,999 → "1000k").
    return `${compactNumber(Math.min(n / 1_000, 999.9))}k`;
  }
  return `${compactNumber(n / 1_000_000)}M`;
}

/** DeepSeek prompt-cache hit rate: cacheRead / (input + cacheRead). */
export function promptCacheHitPercent(inputTokens: number, cacheReadTokens: number): number | null {
  const input = Math.max(0, Number.isFinite(inputTokens) ? inputTokens : 0);
  const cached = Math.max(0, Number.isFinite(cacheReadTokens) ? cacheReadTokens : 0);
  const prompt = input + cached;
  if (prompt <= 0) return null;
  return Math.round((cached / prompt) * 100);
}

export function promptTokenTotal(inputTokens: number, cacheReadTokens: number): number {
  return (
    Math.max(0, Number.isFinite(inputTokens) ? inputTokens : 0) +
    Math.max(0, Number.isFinite(cacheReadTokens) ? cacheReadTokens : 0)
  );
}

export function accumulateCacheUsage<
  T extends { inputTokens: number; cacheReadTokens: number; model?: string },
>(current: T | undefined, delta: T): T {
  return {
    ...current,
    ...delta,
    inputTokens: Math.max(0, current?.inputTokens ?? 0) + Math.max(0, delta.inputTokens),
    cacheReadTokens:
      Math.max(0, current?.cacheReadTokens ?? 0) + Math.max(0, delta.cacheReadTokens),
    model: delta.model ?? current?.model,
  };
}
