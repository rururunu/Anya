import { describe, expect, it, vi } from "vitest";

import { safeGsap } from "./gsapSafe";

describe("safeGsap", () => {
  it("runs the primary path when no error", () => {
    let ran = false;
    safeGsap("ok", () => {
      ran = true;
    });
    expect(ran).toBe(true);
  });

  it("invokes fallback and does not throw when primary fails", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    let fallback = false;
    expect(() => {
      safeGsap(
        "boom",
        () => {
          throw new Error("webview split");
        },
        () => {
          fallback = true;
        },
      );
    }).not.toThrow();
    expect(fallback).toBe(true);
    expect(warn).toHaveBeenCalled();
    warn.mockRestore();
  });

  it("swallows fallback errors too", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    expect(() => {
      safeGsap(
        "double",
        () => {
          throw new Error("primary");
        },
        () => {
          throw new Error("fallback");
        },
      );
    }).not.toThrow();
    expect(warn).toHaveBeenCalledTimes(2);
    warn.mockRestore();
  });
});
