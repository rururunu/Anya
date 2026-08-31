import { describe, expect, it } from "vitest";
import { normalizeModelProtocol, normalizeProviderApiProtocol } from "@/types/setting";

describe("provider protocol config", () => {
  it("normalizes the provider default including Anthropic", () => {
    expect(normalizeProviderApiProtocol(undefined)).toBe("chatCompletions");
    expect(normalizeProviderApiProtocol("responses")).toBe("responses");
    expect(normalizeProviderApiProtocol("anthropicMessages")).toBe("anthropicMessages");
  });

  it("treats unknown per-model values as unset", () => {
    expect(normalizeModelProtocol("inherit")).toBeUndefined();
    expect(normalizeModelProtocol("chatCompletions")).toBe("chatCompletions");
    expect(normalizeModelProtocol("anthropicMessages")).toBe("anthropicMessages");
  });
});
