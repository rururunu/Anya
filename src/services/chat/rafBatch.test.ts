import { afterEach, describe, expect, it, vi } from "vitest";
import { createRafBatch } from "./rafBatch";

describe("createRafBatch", () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("bounds stream redraws while preserving every delta and drains immediately on finish", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("requestAnimationFrame", undefined);
    const flush = vi.fn();
    const batch = createRafBatch<string>(flush, 50);
    for (let index = 0; index < 100; index++) batch.push(String(index));
    await vi.advanceTimersByTimeAsync(49);
    expect(flush).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(flush).toHaveBeenCalledTimes(1);
    expect(flush.mock.calls[0]?.[0]).toHaveLength(100);
    batch.push("last delta");
    batch.drain();
    expect(flush).toHaveBeenLastCalledWith(["last delta"]);
    await vi.advanceTimersByTimeAsync(100);
    expect(flush).toHaveBeenCalledTimes(2);
    expect(batch.size()).toBe(0);
  });

  it("batches pushes and flushes on drain without rAF", async () => {
    vi.stubGlobal("requestAnimationFrame", undefined);
    vi.stubGlobal("cancelAnimationFrame", undefined);

    const flush = vi.fn();
    const batch = createRafBatch<number>(flush);

    batch.push(1);
    batch.push(2);
    expect(batch.size()).toBe(2);

    await Promise.resolve();
    expect(flush).toHaveBeenCalledWith([1, 2]);
    expect(batch.size()).toBe(0);
  });

  it("schedules requestAnimationFrame when available", () => {
    const callbacks: FrameRequestCallback[] = [];
    vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
      callbacks.push(cb);
      return 42;
    });
    vi.stubGlobal("cancelAnimationFrame", vi.fn());

    const flush = vi.fn();
    const batch = createRafBatch<string>(flush);
    batch.push("a");
    batch.push("b");
    expect(flush).not.toHaveBeenCalled();
    expect(callbacks).toHaveLength(1);

    callbacks[0]!(0);
    expect(flush).toHaveBeenCalledWith(["a", "b"]);
  });

  it("drain cancels pending rAF and flushes immediately", () => {
    const cancel = vi.fn();
    vi.stubGlobal("requestAnimationFrame", () => 7);
    vi.stubGlobal("cancelAnimationFrame", cancel);

    const flush = vi.fn();
    const batch = createRafBatch<number>(flush);
    batch.push(9);
    batch.drain();

    expect(cancel).toHaveBeenCalledWith(7);
    expect(flush).toHaveBeenCalledWith([9]);
  });
});
