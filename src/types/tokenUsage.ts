export type TokenAccuracy = "exact" | "mixed" | "estimated";

export interface TokenUsage {
  inputTokens: number;
  outputTokens: number;
  systemTokens: number;
  contextTokens: number;
  toolCallTokens: number;
  toolResultTokens: number;
  memoryTokens: number;
  totalTokens: number;
  accuracy: TokenAccuracy;
  source?: string;
  /** Prompt tokens served from cache (DeepSeek reports these inside promptTokens). */
  cacheReadTokens?: number;
  /** Reasoning/thinking tokens (part of outputTokens). */
  reasoningTokens?: number;
}

export interface TokenUsageReport {
  from: number;
  to: number;
  granularity: "day" | "week" | "month";
  total: TokenUsage;
  modelCalls: number;
  byModel: Array<{
    model: string;
    provider?: string;
    usage: TokenUsage;
    calls: number;
    share: number;
  }>;
  timeline: Array<{
    bucket: string;
    label: string;
    totalTokens: number;
    inputTokens: number;
    outputTokens: number;
    models: Record<string, number>;
  }>;
}
