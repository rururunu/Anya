import { modelHasThinkingVariants } from "@/lib/modelThinking";
import type { ChatModelInfo } from "@/types/chat";
import {
  normalizeProviderApiProtocol,
  reasoningEffortOptions,
  type CustomProviderConfig,
  type ReasoningEffort,
} from "@/types/setting";

/**
 * Input-bar thinking control.
 *
 * - `variants`: swap model IDs in the same family (Gemini High / Low / Agent).
 * - `effort`: keep the model, write `settings.reasoningEffort` (DeepSeek thinking
 *   and Responses `reasoning.effort`, e.g. Grok).
 */
export type ReasoningControl =
  { kind: "none" } | { kind: "variants" } | { kind: "effort"; allowsDisabled: boolean };

export type ResolveReasoningControlInput = {
  modelId: string;
  providerId: string;
  entry?: ChatModelInfo | null;
  customProviders?: CustomProviderConfig[];
};

export function isReasoningEffort(value: string): value is ReasoningEffort {
  return value === "disabled" || value === "high" || value === "max";
}

export function resolveReasoningControl(input: ResolveReasoningControlInput): ReasoningControl {
  const entry = input.entry ?? null;
  if (entry && modelHasThinkingVariants(entry)) {
    return { kind: "variants" };
  }

  const model = (entry?.id ?? input.modelId).trim().toLowerCase();
  const provider = (entry?.provider ?? input.providerId).trim().toLowerCase();

  if (isDeepSeekModel(model, provider)) {
    return { kind: "effort", allowsDisabled: true };
  }

  if (isGrokModel(model, provider)) {
    return { kind: "effort", allowsDisabled: false };
  }

  const custom = findCustomProvider(
    input.customProviders,
    entry?.provider ?? input.providerId,
    model,
  );
  if (custom && normalizeProviderApiProtocol(custom.apiProtocol) === "responses") {
    return { kind: "effort", allowsDisabled: false };
  }

  return { kind: "none" };
}

/** Grok cannot turn reasoning off; the request layer maps `disabled` → `high`. */
export function effectiveReasoningEffort(
  requested: ReasoningEffort,
  control: ReasoningControl,
): ReasoningEffort {
  if (control.kind === "effort" && !control.allowsDisabled && requested === "disabled") {
    return "high";
  }
  return requested;
}

export function effortOptionsForControl(control: ReasoningControl) {
  if (control.kind !== "effort") {
    return [];
  }
  return control.allowsDisabled
    ? reasoningEffortOptions
    : reasoningEffortOptions.filter((option) => option.value !== "disabled");
}

function isDeepSeekModel(model: string, provider: string): boolean {
  return model.startsWith("deepseek") || provider === "deepseek";
}

function isGrokModel(model: string, provider: string): boolean {
  return model.startsWith("grok") || provider === "xai" || provider.includes("x.ai");
}

function findCustomProvider(
  providers: CustomProviderConfig[] | undefined,
  providerId: string,
  modelId: string,
): CustomProviderConfig | undefined {
  if (!providers?.length) {
    return undefined;
  }
  const pid = providerId.trim();
  if (pid) {
    const byId = providers.find((provider) => provider.id === pid);
    if (byId) {
      return byId;
    }
  }
  const mid = modelId.trim().toLowerCase();
  if (!mid) {
    return undefined;
  }
  return providers.find((provider) =>
    provider.models
      .split(/[\n,，]/)
      .map((part) => part.trim().toLowerCase())
      .includes(mid),
  );
}
