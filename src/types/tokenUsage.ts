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

export interface DeepSeekBalanceInfo {
  currency: string;
  totalBalance: string;
  grantedBalance: string;
  toppedUpBalance: string;
}

export interface DeepSeekBalanceReport {
  configured: boolean;
  isAvailable: boolean | null;
  balances: DeepSeekBalanceInfo[];
}

export interface DeepSeekFileObject {
  id: string;
  object: string;
  bytes: number;
  createdAt: number;
  filename: string;
  purpose: string;
  expiresAt?: number | null;
}

export interface DeepSeekFileList {
  object: string;
  data: DeepSeekFileObject[];
  firstId?: string | null;
  lastId?: string | null;
  hasMore: boolean;
}

export interface DeepSeekFileDeleteResult {
  id: string;
  deleted: boolean;
}
