import type { ReasoningEffort } from "@/types/setting";

/**
 * Official thinking ladders differ by family:
 * DeepSeek: disabled / low / high / max
 * OpenAI GPT-5: none / minimal / low / medium / high / xhigh / max
 * OpenAI o-series: low / medium / high
 * Grok 4.6: low / medium / high / xhigh (cannot disable)
 * Grok 4.5: low / medium / high
 * Claude: low / medium / high / xhigh / max
 * Kimi K3: low / high / max (always on)
 * Kimi K2: thinking on/off
 * Qwen 3.8 Max: disabled / low / medium / xhigh
 * other Qwen: thinking on/off
 * GLM-5.2+: none / minimal / low / medium / high / xhigh / max
 * MiniMax: on/off
 */
export type ReasoningFamily =
  | "deepseek"
  | "openai"
  | "openai-o"
  | "grok"
  | "grok45"
  | "claude"
  | "kimi-k3"
  | "kimi-k2"
  | "qwen38"
  | "qwen"
  | "glm52"
  | "glm51"
  | "glm"
  | "minimax"
  | "generic";

export type ReasoningProfile = {
  family: ReasoningFamily;
  levels: ReasoningEffort[];
  defaultLevel: ReasoningEffort;
};

const RANK: ReasoningEffort[] = [
  "disabled",
  "none",
  "minimal",
  "low",
  "medium",
  "high",
  "xhigh",
  "max",
];

const PROFILES: Record<ReasoningFamily, ReasoningProfile> = {
  deepseek: {
    family: "deepseek",
    levels: ["disabled", "low", "high", "max"],
    defaultLevel: "high",
  },
  openai: {
    family: "openai",
    levels: ["none", "minimal", "low", "medium", "high", "xhigh", "max"],
    defaultLevel: "medium",
  },
  "openai-o": { family: "openai-o", levels: ["low", "medium", "high"], defaultLevel: "medium" },
  grok: { family: "grok", levels: ["low", "medium", "high", "xhigh"], defaultLevel: "high" },
  grok45: { family: "grok45", levels: ["low", "medium", "high"], defaultLevel: "high" },
  claude: {
    family: "claude",
    levels: ["low", "medium", "high", "xhigh", "max"],
    defaultLevel: "high",
  },
  "kimi-k3": { family: "kimi-k3", levels: ["low", "high", "max"], defaultLevel: "max" },
  "kimi-k2": { family: "kimi-k2", levels: ["disabled", "high"], defaultLevel: "high" },
  qwen38: {
    family: "qwen38",
    levels: ["disabled", "low", "medium", "xhigh"],
    defaultLevel: "xhigh",
  },
  qwen: { family: "qwen", levels: ["disabled", "high"], defaultLevel: "high" },
  glm52: {
    family: "glm52",
    levels: ["none", "minimal", "low", "medium", "high", "xhigh", "max"],
    defaultLevel: "max",
  },
  glm51: {
    family: "glm51",
    levels: ["none", "minimal", "low", "medium", "high", "xhigh"],
    defaultLevel: "xhigh",
  },
  glm: { family: "glm", levels: ["disabled", "high"], defaultLevel: "high" },
  minimax: { family: "minimax", levels: ["disabled", "high"], defaultLevel: "high" },
  generic: {
    family: "generic",
    levels: ["disabled", "low", "medium", "high", "max"],
    defaultLevel: "high",
  },
};

export function profileAllowsOff(profile: ReasoningProfile): boolean {
  return profile.levels.some((level) => level === "disabled" || level === "none");
}

const OFF_LEVELS: ReasoningEffort[] = ["disabled", "none"];

export function clampReasoningEffort(
  requested: ReasoningEffort,
  profile: ReasoningProfile,
): ReasoningEffort {
  if (profile.levels.includes(requested)) {
    return requested;
  }
  if (requested === "disabled" && profile.levels.includes("none")) {
    return "none";
  }
  if (requested === "none" && profile.levels.includes("disabled")) {
    return "disabled";
  }
  if (
    OFF_LEVELS.includes(requested) &&
    !profile.levels.some((level) => OFF_LEVELS.includes(level))
  ) {
    return profile.defaultLevel;
  }
  const target = RANK.indexOf(requested);
  let best = profile.defaultLevel;
  let bestDist = Number.POSITIVE_INFINITY;
  for (const level of profile.levels) {
    const dist = Math.abs(RANK.indexOf(level) - target);
    if (dist < bestDist || (dist === bestDist && RANK.indexOf(level) > RANK.indexOf(best))) {
      bestDist = dist;
      best = level;
    }
  }
  return best;
}

export function resolveReasoningFamily(modelId: string, providerId = ""): ReasoningFamily | null {
  const model = modelId.trim().toLowerCase();
  const provider = providerId.trim().toLowerCase();
  if (!model && !provider) {
    return null;
  }

  if (model.startsWith("deepseek") || model.includes("deepseek") || provider === "deepseek") {
    return "deepseek";
  }

  if (isGrokModel(model, provider)) {
    return isGrok45(model) ? "grok45" : "grok";
  }

  if (/(^|[-_./])kimi[-_.]?k3\b/.test(model) || model.includes("kimi-k3")) {
    return "kimi-k3";
  }
  if (/(^|[-_./])(kimi|moonshot)\b/.test(model) || provider === "kimi" || provider === "moonshot") {
    return "kimi-k2";
  }

  if (/qwen3[.-]?8/.test(model)) {
    return "qwen38";
  }
  if (/(^|[-_./])(qwen|qwq|qvq)\b/.test(model) || provider === "qwen") {
    return "qwen";
  }

  if (/glm[-_.]?5[.-]?[23]/.test(model) || /glm[-_.]?5[.-]?[4-9]/.test(model)) {
    return "glm52";
  }
  if (/glm[-_.]?5[.-]?1/.test(model)) {
    return "glm51";
  }
  if (/(^|[-_./])(glm|chatglm)\b/.test(model) || provider === "zhipu" || provider === "glm") {
    return "glm";
  }

  if (/(^|[-_./])(claude|anthropic)\b/.test(model) || provider === "anthropic") {
    return "claude";
  }

  if (/(^|[-_./])(minimax|mimo)\b/.test(model) || provider === "minimax") {
    return "minimax";
  }

  if (/\bo[1-4](?:-mini|-pro)?\b/.test(model)) {
    return "openai-o";
  }
  if (/(^|[-_./])gpt-5\b/.test(model)) {
    return "openai";
  }

  return null;
}

export function profileForModel(modelId: string, providerId = ""): ReasoningProfile | null {
  const family = resolveReasoningFamily(modelId, providerId);
  return family ? { ...PROFILES[family], levels: [...PROFILES[family].levels] } : null;
}

export function genericReasoningProfile(): ReasoningProfile {
  return { ...PROFILES.generic, levels: [...PROFILES.generic.levels] };
}

/** Custom Responses endpoints in this app use xAI-shaped `reasoning.effort`. */
export function responsesReasoningProfile(): ReasoningProfile {
  return { ...PROFILES.grok, levels: [...PROFILES.grok.levels] };
}

function isGrokModel(model: string, provider: string): boolean {
  return (
    /(^|[-_./])grok\b/.test(model) ||
    provider === "xai" ||
    provider === "grok" ||
    provider.includes("x.ai")
  );
}

function isGrok45(model: string): boolean {
  return /grok[-_.]?4[.-]?5/.test(model);
}
