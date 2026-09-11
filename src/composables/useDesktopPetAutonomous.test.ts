/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createApp, defineComponent } from "vue";
import { useDesktopPetAutonomous } from "./useDesktopPetAutonomous";

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

describe("useDesktopPetAutonomous", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("triggers autonomous action after idle interval", async () => {
    const [auto, app] = withSetup(() => useDesktopPetAutonomous());
    auto.start();

    expect(auto.currentAutonomousAction.value).toBe("none");

    // Advance timers until an action triggers (up to 16s)
    while (auto.currentAutonomousAction.value === "none") {
      await vi.advanceTimersByTimeAsync(1000);
    }

    expect(["wrench", "morph", "look-around", "stretch"]).toContain(
      auto.currentAutonomousAction.value,
    );

    // After action finishes (e.g. 2600ms), returns to none
    await vi.advanceTimersByTimeAsync(3000);
    expect(auto.currentAutonomousAction.value).toBe("none");

    app.unmount();
  });

  it("handles look-around gaze sequence accurately", async () => {
    const [auto, app] = withSetup(() => useDesktopPetAutonomous());
    auto.executeAutonomousAction("look-around");

    expect(auto.currentAutonomousAction.value).toBe("look-around");

    // Phase 1: look left
    await vi.advanceTimersByTimeAsync(150);
    expect(auto.autonomousGaze.value).toEqual({ x: -26, y: -5 });

    // Phase 2: look right
    await vi.advanceTimersByTimeAsync(900);
    expect(auto.autonomousGaze.value).toEqual({ x: 28, y: -4 });

    // Phase 3: look center
    await vi.advanceTimersByTimeAsync(850);
    expect(auto.autonomousGaze.value).toEqual({ x: 0, y: 0 });

    // Action ends
    await vi.advanceTimersByTimeAsync(600);
    expect(auto.currentAutonomousAction.value).toBe("none");
    expect(auto.autonomousGaze.value).toBeNull();

    app.unmount();
  });

  it("interrupts immediately when user interacts", async () => {
    const [auto, app] = withSetup(() => useDesktopPetAutonomous());
    auto.executeAutonomousAction("wrench");
    expect(auto.currentAutonomousAction.value).toBe("wrench");

    auto.interruptAutonomous();
    expect(auto.currentAutonomousAction.value).toBe("none");
    expect(auto.autonomousGaze.value).toBeNull();

    app.unmount();
  });

  it("triggers sleep after deep idle", async () => {
    const onEnterSleep = vi.fn();
    const [auto, app] = withSetup(() =>
      useDesktopPetAutonomous({
        onEnterSleep,
      }),
    );
    auto.start();

    // Advance by 50s deep sleep timer
    await vi.advanceTimersByTimeAsync(50100);
    expect(onEnterSleep).toHaveBeenCalledTimes(1);

    app.unmount();
  });
});
