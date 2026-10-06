// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { revealOverlay } from "./reveal";

afterEach(() => vi.unstubAllGlobals());
describe("overlay reveal", () => {
  it("animates only opacity and transform and supports cancellation on hide", () => {
    const element = document.createElement("div");
    const cancel = vi.fn();
    element.animate = vi.fn(() => ({ cancel }) as unknown as Animation);
    const stop = revealOverlay(element);
    expect(element.animate).toHaveBeenCalledWith(
      [
        { opacity: 0, transform: "scale(0.97)" },
        { opacity: 1, transform: "scale(1)" },
      ],
      expect.objectContaining({ duration: 240, easing: "cubic-bezier(0.22, 1, 0.36, 1)" }),
    );
    stop();
    expect(cancel).toHaveBeenCalledOnce();
  });
  it("uses an immediate fallback for reduced motion or unsupported WebViews", () => {
    vi.stubGlobal("matchMedia", () => ({ matches: true }));
    const element = document.createElement("div");
    element.animate = vi.fn();
    revealOverlay(element)();
    expect(element.animate).not.toHaveBeenCalled();
    expect(() => revealOverlay(null)()).not.toThrow();
  });
});
