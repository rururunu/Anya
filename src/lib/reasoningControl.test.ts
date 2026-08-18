import { describe, expect, it } from "vitest";
import type { ChatModelInfo } from "@/types/chat";
import type { CustomProviderConfig } from "@/types/setting";
import {
  effectiveReasoningEffort,
  effortOptionsForControl,
  resolveReasoningControl,
} from "./reasoningControl";

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

  it("exposes DeepSeek effort including disabled", () => {
    expect(
      resolveReasoningControl({
        modelId: "deepseek-chat",
        providerId: "deepseek",
      }),
    ).toEqual({ kind: "effort", allowsDisabled: true });
  });

  it("detects DeepSeek from the model id even on a custom provider", () => {
    expect(
      resolveReasoningControl({
        modelId: "deepseek-reasoner",
        providerId: "my-relay",
      }),
    ).toEqual({ kind: "effort", allowsDisabled: true });
  });

  it("exposes Grok effort without disabled", () => {
    expect(
      resolveReasoningControl({
        modelId: "grok-4.6",
        providerId: "custom-xai",
      }),
    ).toEqual({ kind: "effort", allowsDisabled: false });
  });

  it("treats Responses custom providers as effort controls", () => {
    expect(
      resolveReasoningControl({
        modelId: "my-reasoner",
        providerId: "prov-1",
        customProviders: [
          custom({ id: "prov-1", models: "my-reasoner", apiProtocol: "responses" }),
        ],
      }),
    ).toEqual({ kind: "effort", allowsDisabled: false });
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
});

describe("effectiveReasoningEffort", () => {
  it("maps disabled to high when the provider cannot turn thinking off", () => {
    expect(effectiveReasoningEffort("disabled", { kind: "effort", allowsDisabled: false })).toBe(
      "high",
    );
    expect(effectiveReasoningEffort("max", { kind: "effort", allowsDisabled: false })).toBe("max");
    expect(effectiveReasoningEffort("disabled", { kind: "effort", allowsDisabled: true })).toBe(
      "disabled",
    );
  });
});

describe("effortOptionsForControl", () => {
  it("omits disabled for Grok-style controls", () => {
    expect(
      effortOptionsForControl({ kind: "effort", allowsDisabled: false }).map(
        (option) => option.value,
      ),
    ).toEqual(["high", "max"]);
    expect(
      effortOptionsForControl({ kind: "effort", allowsDisabled: true }).map(
        (option) => option.value,
      ),
    ).toEqual(["disabled", "high", "max"]);
  });
});
