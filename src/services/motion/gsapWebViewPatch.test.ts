import { afterEach, describe, expect, it, vi } from "vitest";

describe("gsapWebViewPatch", () => {
  afterEach(() => {
    vi.resetModules();
    vi.unstubAllGlobals();
  });

  it("coerces undefined individual transform props to none", async () => {
    const base = {
      scale: undefined,
      rotate: undefined,
      translate: "none",
      getPropertyValue: () => "",
    } as unknown as CSSStyleDeclaration;

    const getComputedStyle = vi.fn(() => base);
    vi.stubGlobal("window", {
      getComputedStyle,
    });
    vi.stubGlobal("getComputedStyle", getComputedStyle);

    await import("./gsapWebViewPatch");

    const el = {} as Element;
    const cs = window.getComputedStyle(el);
    expect(cs.scale).toBe("none");
    expect(cs.rotate).toBe("none");
    expect(cs.translate).toBe("none");
  });
});
