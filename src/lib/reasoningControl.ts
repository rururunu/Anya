import { modelHasThinkingVariants } from "@/lib/modelThinking";
import {
  clampReasoningEffort,
  genericReasoningProfile,
  profileForModel,
  responsesReasoningProfile,
  type ReasoningProfile,
} from "@/lib/reasoningProfiles";
import type { ChatModelInfo } from "@/types/chat";
import {
  isReasoningEffort as isKnownReasoningEffort,
  normalizeProviderApiProtocol,
  reasoningEffortOptions,
  type CustomProviderConfig,
  type ReasoningEffort,
} from "@/types/setting";

/**
 * Input-bar thinking control.
 *
 * - `variants`: swap model IDs in the same family (Gemini High / Low / Agent).
 * - `effort`: keep the model, write `settings.reasoningEffort` using that
 *   family's official levels (DeepSeek low/high/max, GPT none/minimal/…, Grok
 *   low/medium/high/xhigh, …).
 */
export type ReasoningControl =
  { kind: "none" } | { kind: "variants" } | { kind: "effort"; profile: ReasoningProfile };

export type ResolveReasoningControlInput = {
  modelId: string;
  providerId: string;
  entry?: ChatModelInfo | null;
  customProviders?: CustomProviderConfig[];
};

export function isReasoningEffort(value: string): value is ReasoningEffort {
  return isKnownReasoningEffort(value);
}

export function resolveReasoningControl(input: ResolveReasoningControlInput): ReasoningControl {
  const entry = input.entry ?? null;
  if (entry && modelHasThinkingVariants(entry)) {
    return { kind: "variants" };
  }

  const model = (entry?.id ?? input.modelId).trim().toLowerCase();
  const provider = (entry?.provider ?? input.providerId).trim().toLowerCase();

  const advertised = entry?.reasoning;
  if (advertised && !advertised.supported) {
    return { kind: "none" };
  }

  const profile = profileForModel(model, provider);
  if (profile) {
    return { kind: "effort", profile };
  }

  if (advertised?.supported) {
    return { kind: "effort", profile: genericReasoningProfile() };
  }

  const custom = findCustomProvider(
    input.customProviders,
    entry?.provider ?? input.providerId,
    model,
  );
  if (custom && normalizeProviderApiProtocol(custom.apiProtocol) === "responses") {
    return { kind: "effort", profile: responsesReasoningProfile() };
  }

  return { kind: "none" };
}

export function effectiveReasoningEffort(
  requested: ReasoningEffort,
  control: ReasoningControl,
): ReasoningEffort {
  if (control.kind !== "effort") {
    return requested;
  }
  return clampReasoningEffort(requested, control.profile);
}

export function effortOptionsForControl(control: ReasoningControl) {
  if (control.kind !== "effort") {
    return [];
  }
  return control.profile.levels
    .map((value) => reasoningEffortOptions.find((option) => option.value === value))
    .filter((option): option is (typeof reasoningEffortOptions)[number] => option != null);
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
