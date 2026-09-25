// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { gsapPickerEnter, gsapPickerLeave } from "./gsapPresets";

const motion = vi.hoisted(() => ({
  killTweensOf: vi.fn(),
  delayedCall: vi.fn(() => ({ kill: vi.fn() })),
  fromTo: vi.fn(),
  to: vi.fn(),
}));
vi.mock("./gsapSafe", () => ({
  gsap: motion,
  clearGsapProps: vi.fn(),
  safeGsap: (_label: string, run: () => void) => run(),
}));

afterEach(() => {
  vi.clearAllMocks();
  vi.unstubAllGlobals();
});

describe("picker motion", () => {
  it("animates a single layer regardless of row count and releases interrupted transitions", () => {
    const target = document.createElement("div");
    for (let i = 0; i < 100; i++) {
      const row = document.createElement("div");
      row.className = "command-item";
      target.append(row);
    }
    const done = vi.fn();
    gsapPickerEnter(target, done);
    expect(motion.fromTo).toHaveBeenCalledTimes(1);
    expect(motion.fromTo.mock.calls[0]?.[0]).toBe(target);
    const options = motion.fromTo.mock.calls[0]?.[2];
    expect(options.duration).toBeLessThanOrEqual(0.15);
    expect(options.stagger).toBeUndefined();
    options.onInterrupt();
    options.onComplete();
    expect(done).toHaveBeenCalledTimes(1);
  });

  it("removes an overlay picker immediately so switching does not wait for leave", () => {
    const panel = document.createElement("div");
    panel.className = "peek-panel";
    const target = document.createElement("div");
    panel.append(target);
    const done = vi.fn();
    gsapPickerLeave(target, done);
    expect(done).toHaveBeenCalledTimes(1);
    expect(motion.to).not.toHaveBeenCalled();
  });

  it("respects reduced motion without delaying Vue's transition", () => {
    vi.spyOn(window, "matchMedia").mockReturnValue({ matches: true } as MediaQueryList);
    const done = vi.fn();
    gsapPickerEnter(document.createElement("div"), done);
    expect(done).toHaveBeenCalledTimes(1);
    expect(motion.fromTo).not.toHaveBeenCalled();
    vi.restoreAllMocks();
  });
});
