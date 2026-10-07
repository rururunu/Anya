/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createApp, defineComponent, ref } from "vue";
import { useDesktopPet } from "./useDesktopPet";

const mockSetPosition = vi.hoisted(() => vi.fn().mockResolvedValue(undefined));
const mockStartDragging = vi.hoisted(() => vi.fn().mockResolvedValue(undefined));

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

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({
    label: "desktop-pet",
    setSize: vi.fn().mockResolvedValue(undefined),
    setPosition: mockSetPosition,
    outerPosition: vi.fn().mockResolvedValue({ x: 100, y: 100 }),
    outerSize: vi.fn().mockResolvedValue({ width: 215, height: 215 }),
    startDragging: mockStartDragging,
  }),
}));

vi.mock("@tauri-apps/api/window", () => ({
  availableMonitors: vi
    .fn()
    .mockResolvedValue([{ position: { x: 0, y: 0 }, size: { width: 1920, height: 1080 } }]),
  currentMonitor: vi.fn().mockResolvedValue({
    position: { x: 0, y: 0 },
    size: { width: 1920, height: 1080 },
    scaleFactor: 1,
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("@/services/ipc/commands", () => ({
  showWorkbench: vi.fn().mockResolvedValue(undefined),
  toggleDesktopPet: vi.fn().mockResolvedValue(undefined),
  toggleOverlayFromPet: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("./useDesktopPetInteractions", () => ({
  useDesktopPetInteractions: () => ({
    activeInteraction: ref(null),
    submitAskUserAnswer: vi.fn(),
    submitPathPermission: vi.fn(),
    submitToolApproval: vi.fn(),
    dismissInteraction: vi.fn(),
  }),
}));

describe("useDesktopPet", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockSetPosition.mockClear();
    mockStartDragging.mockClear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("plays spritesheet wave gesture on click without CSS bounce action", async () => {
    const [pet, app] = withSetup(() => useDesktopPet());
    pet.appearance.value = { mode: "spritesheet", kind: "spritesheet", source: "test-pet.png" };
    expect(pet.appearance.value.mode).toBe("spritesheet");
    expect(pet.expression.value).toBe("idle");
    expect(pet.spriteGesture.value).toBeNull();

    pet.onPetClick();
    expect(pet.spriteGesture.value).toBe("wave");
    expect(pet.expression.value).toBe("idle");

    await vi.advanceTimersByTimeAsync(700);
    expect(pet.spriteGesture.value).toBeNull();
    app.unmount();
  });

  it("wakes up sleeping pet and plays wave on click", async () => {
    const [pet, app] = withSetup(() => useDesktopPet());
    pet.appearance.value = { mode: "spritesheet", kind: "spritesheet", source: "test-pet.png" };
    pet.expression.value = "sleeping";
    expect(pet.expression.value).toBe("sleeping");

    pet.onPetClick();
    expect(pet.expression.value).toBe("idle");
    expect(pet.spriteGesture.value).toBe("wave");

    await vi.advanceTimersByTimeAsync(700);
    expect(pet.spriteGesture.value).toBeNull();
    app.unmount();
  });

  it("prioritizes active interaction as waiting", async () => {
    const [pet, app] = withSetup(() => useDesktopPet());
    expect(pet.expression.value).toBe("idle");

    pet.activeInteraction.value = {
      kind: "ask_user",
      requestId: "req-1",
      sessionId: "s1",
      sessionTitle: "Title",
      question: "Which option?",
      options: [],
    } as any;

    expect(pet.expression.value).toBe("waiting");

    pet.activeInteraction.value = null;
    expect(pet.expression.value).toBe("idle");
    app.unmount();
  });

  it("clamps each drag move and shows edge pressure before pointer release", async () => {
    const [pet, app] = withSetup(() => useDesktopPet());
    const target = document.createElement("div");
    target.setPointerCapture = vi.fn();
    const pointer = (screenX: number, screenY: number) =>
      ({
        button: 0,
        pointerId: 1,
        screenX,
        screenY,
        currentTarget: target,
      }) as unknown as PointerEvent;
    await pet.onPetPointerDown(pointer(100, 100));
    pet.onPetPointerMove(pointer(-100, 100));
    expect(pet.edgePressure.value.x).toBe(-1);
    await Promise.resolve();
    expect(mockSetPosition).toHaveBeenCalledWith(expect.objectContaining({ x: 0, y: 100 }));
    expect(mockStartDragging).not.toHaveBeenCalled();
    pet.onPetPointerUp(pointer(-100, 100));
    await Promise.resolve();
    app.unmount();
  });
});
