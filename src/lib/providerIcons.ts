import type { Component } from "vue";
import DeepSeekIcon from "@/components/icons/DeepSeekIcon.vue";
import GeminiIcon from "@/components/icons/GeminiIcon.vue";
import KimiIcon from "@/components/icons/KimiIcon.vue";
import MiMoIcon from "@/components/icons/MiMoIcon.vue";
import MiniMaxIcon from "@/components/icons/MiniMaxIcon.vue";
import VolcengineIcon from "@/components/icons/VolcengineIcon.vue";
import ZhipuIcon from "@/components/icons/ZhipuIcon.vue";
import type { ChatModelInfo } from "@/types/chat";
import type { CustomProviderConfig } from "@/types/setting";

export const DEEPSEEK_PROVIDER = "deepseek";
export const GEMINI_PROVIDER = "gemini";

const providerIcons: Record<string, Component> = {
  [DEEPSEEK_PROVIDER]: DeepSeekIcon,
  [GEMINI_PROVIDER]: GeminiIcon,
  mimo: MiMoIcon,
  zhipu: ZhipuIcon,
  volcengine: VolcengineIcon,
  minimax: MiniMaxIcon,
  kimi: KimiIcon,
};

export function getProviderIcon(
  provider?: string | null,
  presetId?: string | null,
): Component | null {
  const preset = presetId?.trim();
  if (preset && providerIcons[preset]) {
    return providerIcons[preset];
  }
  const key = provider?.trim();
  if (!key) return null;
  return providerIcons[key] ?? null;
}

export function resolveCustomPresetId(
  providerId: string | null | undefined,
  customProviders: CustomProviderConfig[],
): string | undefined {
  if (!providerId) return undefined;
  return customProviders.find((item) => item.id === providerId)?.presetId;
}

export function isDeepSeekProvider(provider?: string | null): boolean {
  return provider === DEEPSEEK_PROVIDER;
}

export function isGeminiProvider(provider?: string | null): boolean {
  return provider === GEMINI_PROVIDER;
}

/** Human-readable vendor label for model picker section headers. */
export function getProviderDisplayName(
  provider?: string | null,
  customProviders: CustomProviderConfig[] = [],
): string {
  const key = provider?.trim() ?? "";
  if (!key) return "Other";
  if (key === DEEPSEEK_PROVIDER) return "DeepSeek";
  if (key === GEMINI_PROVIDER) return "Gemini";
  const custom = customProviders.find((item) => item.id === key);
  if (custom?.name?.trim()) return custom.name.trim();
  const preset = custom?.presetId?.trim();
  if (preset === "mimo") return "小米 MiMo";
  if (preset === "zhipu") return "智谱 GLM";
  if (preset === "volcengine") return "火山方舟";
  if (preset === "minimax") return "MiniMax";
  if (preset === "kimi") return "Kimi";
  return key
    .split(/[-_\s]+/)
    .filter(Boolean)
    .map(capitalizeWord)
    .join(" ");
}

export type ModelProviderGroup = {
  provider: string;
  label: string;
  models: ChatModelInfo[];
};

const PROVIDER_SORT_ORDER = [DEEPSEEK_PROVIDER, GEMINI_PROVIDER];

/** Group models by provider; known vendors first, then A–Z. */
export function groupModelsByProvider(
  models: ChatModelInfo[],
  customProviders: CustomProviderConfig[] = [],
): ModelProviderGroup[] {
  const map = new Map<string, ChatModelInfo[]>();
  for (const model of models) {
    const key = model.provider?.trim() || "other";
    const list = map.get(key);
    if (list) {
      list.push(model);
    } else {
      map.set(key, [model]);
    }
  }

  return Array.from(map.entries())
    .sort(([a], [b]) => {
      const ai = PROVIDER_SORT_ORDER.indexOf(a);
      const bi = PROVIDER_SORT_ORDER.indexOf(b);
      if (ai !== -1 || bi !== -1) {
        if (ai === -1) return 1;
        if (bi === -1) return -1;
        return ai - bi;
      }
      return getProviderDisplayName(a, customProviders).localeCompare(
        getProviderDisplayName(b, customProviders),
        undefined,
        { sensitivity: "base" },
      );
    })
    .map(([provider, grouped]) => ({
      provider,
      label: getProviderDisplayName(provider === "other" ? "" : provider, customProviders),
      models: grouped,
    }));
}

function capitalizeWord(value: string): string {
  if (!value) return value;
  return value.charAt(0).toUpperCase() + value.slice(1);
}

/** Turn Antigravity / Gemini internal ids into readable labels. */
export function formatGeminiDisplayName(modelId: string | undefined | null): string {
  const raw = (modelId ?? "").trim();
  if (!raw) return "";
  const rest = raw.replace(/^gemini[-_]/i, "");
  const lower = rest.toLowerCase();

  let tier = "";
  let body = rest;
  if (lower.endsWith("-agent")) {
    tier = "Agent";
    body = rest.slice(0, -"-agent".length);
  } else if (lower.endsWith("-high")) {
    tier = "High";
    body = rest.slice(0, -"-high".length);
  } else if (lower.endsWith("-low")) {
    tier = "Low";
    body = rest.slice(0, -"-low".length);
  }
  body = body.replace(/-+$/, "");

  const parts = body.split("-").filter(Boolean);
  if (parts.length >= 2) {
    const version = parts[0];
    const family = parts.slice(1).map(capitalizeWord).join(" ");
    const base = `Gemini ${version} ${family}`;
    return tier ? `${base} (${tier})` : base;
  }
  if (parts.length === 1) {
    return `Gemini ${capitalizeWord(parts[0])}`;
  }
  return `Gemini ${body}`;
}

/**
 * Short display label for a model. DeepSeek models drop the `deepseek-` prefix
 * (e.g. `deepseek-v4-pro` → `v4-pro`) since the brand icon already conveys the vendor.
 */
export function formatModelDisplayName(
  modelId: string | undefined | null,
  provider?: string | null,
): string {
  const id = (modelId ?? "").trim();
  if (!id) return id;
  if (isDeepSeekProvider(provider) && /^deepseek[-_]/i.test(id)) {
    return id.replace(/^deepseek[-_]/i, "");
  }
  if (isGeminiProvider(provider)) {
    return formatGeminiDisplayName(id);
  }
  return id;
}

/** Primary label for model pickers and the chat input badge. */
export function getModelDisplayLabel(
  model: Pick<ChatModelInfo, "id" | "provider" | "displayName">,
): string {
  const fromApi = model.displayName?.trim();
  if (fromApi) return fromApi;
  return formatModelDisplayName(model.id, model.provider);
}

/** Optional muted subtitle under the primary model label. */
export function getModelDisplaySubtitle(
  model: Pick<ChatModelInfo, "id" | "provider" | "ownedBy" | "displayName">,
): string | null {
  if (isDeepSeekProvider(model.provider)) {
    return null;
  }
  if (isGeminiProvider(model.provider)) {
    const label = getModelDisplayLabel(model);
    const shortId = formatModelDisplayName(model.id, model.provider);
    if (shortId !== label && shortId !== model.id) {
      return shortId;
    }
    return null;
  }
  const owner = model.ownedBy?.trim();
  return owner || null;
}
