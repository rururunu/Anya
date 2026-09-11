/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createApp, defineComponent } from "vue";
import { useDesktopPetMotion } from "./useDesktopPetMotion";

function withSetup<T>(composable: () => T): [T, ReturnType<typeof createApp>] {
  let result: T;
  const app = createApp(
    defineComponent({
      setup() {
        result = composable();
        return () => {};
      },
    }),
  );
  app.mount(document.createElement("div"));
  return [result!, app];
}

describe("useDesktopPetMotion", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("triggers a valid action on click and auto-resets to idle after duration", async () => {
    const [motion, app] = withSetup(() => useDesktopPetMotion());
    expect(motion.currentAction.value).toBe("idle");
    expect(motion.comboCount.value).toBe(0);

    motion.triggerClickReaction();

    // Advance to rAF
    await vi.advanceTimersByTimeAsync(20);
    expect(motion.comboCount.value).toBe(1);
    expect(["twirl", "bounce", "curious", "jiggle", "double-hop"]).toContain(
      motion.currentAction.value,
    );

    // Advance past max action duration (750ms)
    await vi.advanceTimersByTimeAsync(800);
    expect(motion.currentAction.value).toBe("idle");
    app.unmount();
  });

  it("accumulates comboCount, enables isCombo, and activates combo decor for rapid clicks", async () => {
    const [motion, app] = withSetup(() => useDesktopPetMotion());

    // Click 1
    motion.triggerClickReaction();
    await vi.advanceTimersByTimeAsync(20);
    expect(motion.comboCount.value).toBe(1);
    expect(motion.isCombo.value).toBe(false);
    expect(motion.showComboDecor.value).toBe(false);

    // Click 2 (within 900ms)
    await vi.advanceTimersByTimeAsync(150);
    motion.triggerClickReaction();
    await vi.advanceTimersByTimeAsync(20);
    expect(motion.comboCount.value).toBe(2);
    expect(motion.isCombo.value).toBe(true);
    expect(["twirl", "spin", "double-hop", "bounce"]).toContain(motion.currentAction.value);

    // Click 3 (combo >= 3 triggers sparkles/decor)
    await vi.advanceTimersByTimeAsync(150);
    motion.triggerClickReaction();
    await vi.advanceTimersByTimeAsync(20);
    expect(motion.comboCount.value).toBe(3);
    expect(motion.showComboDecor.value).toBe(true);

    // Click 4
    await vi.advanceTimersByTimeAsync(150);
    motion.triggerClickReaction();
    await vi.advanceTimersByTimeAsync(20);
    expect(motion.comboCount.value).toBe(4);

    // Click 5 (combo >= 5 triggers dizzy!)
    await vi.advanceTimersByTimeAsync(150);
    motion.triggerClickReaction();
    await vi.advanceTimersByTimeAsync(20);
    expect(motion.currentAction.value).toBe("dizzy");

    // After dizzy duration finishes, resets
    await vi.advanceTimersByTimeAsync(800);
    expect(motion.currentAction.value).toBe("idle");
    expect(motion.showComboDecor.value).toBe(false);
    app.unmount();
  });

  it("calculates tilt direction based on click coordinates", async () => {
    const [motion, app] = withSetup(() => useDesktopPetMotion());

    const fakeElement = document.createElement("div");
    vi.spyOn(fakeElement, "getBoundingClientRect").mockReturnValue({
      left: 100,
      top: 100,
      width: 200,
      height: 200,
      right: 300,
      bottom: 300,
      x: 100,
      y: 100,
      toJSON: () => {},
    });

    // Left click (clientX 120 vs center 200 => diff -80 < -36)
    const leftEvent = {
      currentTarget: fakeElement,
      clientX: 120,
    } as unknown as MouseEvent;

    motion.triggerClickReaction(leftEvent);
    expect(motion.tiltDirection.value).toBe("left");

    // Right click (clientX 280 vs center 200 => diff 80 > 36)
    const rightEvent = {
      currentTarget: fakeElement,
      clientX: 280,
    } as unknown as MouseEvent;

    motion.triggerClickReaction(rightEvent);
    expect(motion.tiltDirection.value).toBe("right");

    // Center click (clientX 200)
    const centerEvent = {
      currentTarget: fakeElement,
      clientX: 200,
    } as unknown as MouseEvent;

    motion.triggerClickReaction(centerEvent);
    expect(motion.tiltDirection.value).toBe("center");
    app.unmount();
  });
});
