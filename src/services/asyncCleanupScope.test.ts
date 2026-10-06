import { describe, expect, it, vi } from "vitest";
import { createAsyncCleanupScope } from "./asyncCleanupScope";

describe("asynchronous subscription ownership", () => {
  it("immediately releases a listener registered after unmount", async () => {
    const scope = createAsyncCleanupScope();
    let complete!: (cleanup: () => void) => void;
    const registered = scope.add(
      new Promise((resolve) => {
        complete = resolve;
      }),
    );
    scope.dispose();
    const cleanup = vi.fn();
    complete(cleanup);
    await registered;
    expect(cleanup).toHaveBeenCalledOnce();
    scope.dispose();
    expect(cleanup).toHaveBeenCalledOnce();
  });
  it("releases every registered subscription once", async () => {
    const scope = createAsyncCleanupScope();
    const first = vi.fn();
    const second = vi.fn();
    await scope.add(Promise.resolve(first));
    await scope.add(Promise.resolve(second));
    scope.dispose();
    scope.dispose();
    expect(first).toHaveBeenCalledOnce();
    expect(second).toHaveBeenCalledOnce();
  });
  it("still cleans other subscriptions if one cleanup throws", async () => {
    const warning = vi.spyOn(console, "warn").mockImplementation(() => {});
    const scope = createAsyncCleanupScope();
    const next = vi.fn();
    await scope.add(
      Promise.resolve(() => {
        throw new Error("already closed");
      }),
    );
    await scope.add(Promise.resolve(next));
    scope.dispose();
    expect(next).toHaveBeenCalledOnce();
    warning.mockRestore();
  });
});
