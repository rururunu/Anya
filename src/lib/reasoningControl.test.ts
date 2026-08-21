import { describe, expect, it } from "vitest";
import type { ChatModelInfo } from "@/types/chat";
import type { CustomProviderConfig } from "@/types/setting";
import {
  effectiveReasoningEffort,
  effortOptionsForControl,
  resolveReasoningControl,
} from "./reasoningControl";
import { clampReasoningEffort, profileForModel } from "./reasoningProfiles";

const gemini: ChatModelInfo = {
  id: "gemini-3.1-pro",
  ownedBy: "google",
  provider: "gemini",
  thinkingVariants: [
    { id: "gemini-3.1-pro", label: "High" },
    { id: "gemini-3.1-pro-low", label: "Low" },
  ],
};

function custom(partial: Partial<CustomProviderConfig> & Pick<CustomProviderConfig, "id">) {
  return {
    name: "Custom",
    baseUrl: "https://api.example.com/v1",
    apiKey: "sk-test",
    models: "gpt-4o",
    ...partial,
  } satisfies CustomProviderConfig;
}

function effortLevels(
  modelId: string,
  providerId: string,
  extra?: Partial<Parameters<typeof resolveReasoningControl>[0]>,
) {
  const control = resolveReasoningControl({ modelId, providerId, ...extra });
  expect(control.kind).toBe("effort");
  if (control.kind !== "effort") {
    throw new Error("expected effort control");
  }
  return control.profile.levels;
}

describe("resolveReasoningControl", () => {
  it("prefers Gemini-style model variants over effort", () => {
    expect(
      resolveReasoningControl({
        modelId: "gemini-3.1-pro",
        providerId: "gemini",
        entry: gemini,
      }),
    ).toEqual({ kind: "variants" });
  });

  it("uses DeepSeek official levels including off and low", () => {
    expect(effortLevels("deepseek-chat", "deepseek")).toEqual(["disabled", "low", "high", "max"]);
  });

  it("detects DeepSeek from the model id even on a custom provider", () => {
    expect(effortLevels("deepseek-reasoner", "my-relay")).toEqual([
      "disabled",
      "low",
      "high",
      "max",
    ]);
  });

  it("uses Grok 4.6 official levels without off", () => {
    expect(effortLevels("grok-4.6", "custom-xai")).toEqual(["low", "medium", "high", "xhigh"]);
  });

  it("omits xhigh on Grok 4.5", () => {
    expect(effortLevels("grok-4.5", "xai")).toEqual(["low", "medium", "high"]);
  });

  it("uses GPT-5 official names including none and minimal", () => {
    expect(effortLevels("gpt-5.1", "openai")).toEqual([
      "none",
      "minimal",
      "low",
      "medium",
      "high",
      "xhigh",
      "max",
    ]);
  });

  it("uses o-series low/medium/high", () => {
    expect(effortLevels("o3-mini", "openai")).toEqual(["low", "medium", "high"]);
  });

  it("treats unknown Responses custom providers as Grok-shaped effort", () => {
    expect(
      effortLevels("my-reasoner", "prov-1", {
        customProviders: [
          custom({ id: "prov-1", models: "my-reasoner", apiProtocol: "responses" }),
        ],
      }),
    ).toEqual(["low", "medium", "high", "xhigh"]);
  });

  it("hides the chip for Chat Completions models without variants", () => {
    expect(
      resolveReasoningControl({
        modelId: "gpt-4o",
        providerId: "prov-1",
        customProviders: [custom({ id: "prov-1", apiProtocol: "chatCompletions" })],
      }),
    ).toEqual({ kind: "none" });
  });

  it("exposes family-specific effort for common thinking models", () => {
    expect(effortLevels("kimi-k3", "aggregator")).toEqual(["low", "high", "max"]);
    expect(effortLevels("glm-5.2", "aggregator")).toEqual([
      "none",
      "minimal",
      "low",
      "medium",
      "high",
      "xhigh",
      "max",
    ]);
    expect(effortLevels("qwen3.8-max", "aggregator")).toEqual([
      "disabled",
      "low",
      "medium",
      "xhigh",
    ]);
    expect(effortLevels("minimax-m3", "aggregator")).toEqual(["disabled", "high"]);
    expect(effortLevels("claude-sonnet-4", "aggregator")).toEqual([
      "low",
      "medium",
      "high",
      "xhigh",
      "max",
    ]);
  });

  it("prefers advertised /models reasoning over name heuristics for hide/show", () => {
    expect(
      effortLevels("gpt-4o", "openrouter", {
        entry: {
          id: "gpt-4o",
          ownedBy: "openai",
          provider: "openrouter",
          reasoning: { supported: true, canDisable: true },
        },
      }),
    ).toEqual(["disabled", "low", "medium", "high", "max"]);

    expect(
      resolveReasoningControl({
        modelId: "claude-sonnet-4",
        providerId: "openrouter",
        entry: {
          id: "claude-sonnet-4",
          ownedBy: "anthropic",
          provider: "openrouter",
          reasoning: { supported: false },
        },
      }),
    ).toEqual({ kind: "none" });
  });
});

describe("effectiveReasoningEffort", () => {
  it("clamps disabled onto families that cannot turn thinking off", () => {
    const grok = resolveReasoningControl({ modelId: "grok-4.6", providerId: "xai" });
    expect(effectiveReasoningEffort("disabled", grok)).toBe("high");
    expect(effectiveReasoningEffort("max", grok)).toBe("xhigh");
    expect(effectiveReasoningEffort("low", grok)).toBe("low");
  });

  it("maps disabled to none on GPT-5", () => {
    const gpt = resolveReasoningControl({ modelId: "gpt-5.1", providerId: "openai" });
    expect(effectiveReasoningEffort("disabled", gpt)).toBe("none");
  });

  it("keeps DeepSeek disabled", () => {
    const deepseek = resolveReasoningControl({ modelId: "deepseek-chat", providerId: "deepseek" });
    expect(effectiveReasoningEffort("disabled", deepseek)).toBe("disabled");
  });
});

describe("effortOptionsForControl", () => {
  it("lists official Grok and DeepSeek names", () => {
    const grok = resolveReasoningControl({ modelId: "grok-4.6", providerId: "xai" });
    expect(effortOptionsForControl(grok).map((option) => option.value)).toEqual([
      "low",
      "medium",
      "high",
      "xhigh",
    ]);
    const deepseek = resolveReasoningControl({ modelId: "deepseek-chat", providerId: "deepseek" });
    expect(effortOptionsForControl(deepseek).map((option) => option.value)).toEqual([
      "disabled",
      "low",
      "high",
      "max",
    ]);
  });
});

describe("clampReasoningEffort", () => {
  it("maps neighboring official names", () => {
    const kimi = profileForModel("kimi-k3");
    expect(kimi).not.toBeNull();
    if (!kimi) {
      return;
    }
    expect(clampReasoningEffort("medium", kimi)).toBe("high");
    expect(clampReasoningEffort("xhigh", kimi)).toBe("max");
    expect(clampReasoningEffort("disabled", kimi)).toBe("max");
  });
});
