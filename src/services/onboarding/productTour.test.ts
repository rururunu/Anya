import { describe, expect, it } from "vitest";

import { hasConfiguredProvider } from "./providerReady";
import { planProductTour, selectProductTourSteps } from "./productTourPlan";

describe("hasConfiguredProvider", () => {
  it("accepts a deepseek key or a custom provider with url and key", () => {
    expect(hasConfiguredProvider({ deepseekApiKey: "  sk-test  " })).toBe(true);
    expect(
      hasConfiguredProvider({
        customProviders: [{ baseUrl: "https://example.com", apiKey: "key" }],
      }),
    ).toBe(true);
    expect(hasConfiguredProvider({ deepseekApiKey: "  " })).toBe(false);
    expect(
      hasConfiguredProvider({
        customProviders: [{ baseUrl: "https://example.com", apiKey: "" }],
      }),
    ).toBe(false);
  });
});

describe("planProductTour", () => {
  it("teaches the key before the composer when no provider is ready", () => {
    expect(planProductTour(false).map((step) => step.id)).toEqual([
      "welcome",
      "provider",
      "composer",
      "hotkey",
      "workspace",
    ]);
    expect(planProductTour(false).find((step) => step.id === "provider")?.allowInteraction).toBe(
      true,
    );
  });

  it("starts from the composer once a provider exists", () => {
    expect(planProductTour(true).map((step) => step.id)).toEqual([
      "welcome",
      "composer",
      "hotkey",
      "workspace",
    ]);
  });

  it("drops anchors that are not on screen", () => {
    const visible = selectProductTourSteps(planProductTour(false), (selector) =>
      selector.includes("composer"),
    );
    expect(visible.map((step) => step.id)).toEqual(["welcome", "composer"]);
  });
});
