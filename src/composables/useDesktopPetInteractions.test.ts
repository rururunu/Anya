/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createApp, defineComponent, ref } from "vue";
import { useDesktopPetInteractions } from "./useDesktopPetInteractions";
import { IPC_EVENTS } from "@/types/ipc";

const listeners = new Map<string, (event: any) => void>();

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({
    label: "desktop-pet",
    setSize: vi.fn().mockResolvedValue(undefined),
    setPosition: vi.fn().mockResolvedValue(undefined),
    outerPosition: vi.fn().mockResolvedValue({ x: 100, y: 100 }),
  }),
}));

vi.mock("@tauri-apps/api/window", () => ({
  currentMonitor: vi.fn().mockResolvedValue({
    size: { width: 1920, height: 1080 },
    scaleFactor: 1,
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((event: string, handler: (event: any) => void) => {
    listeners.set(event, handler);
    return Promise.resolve(() => {
      listeners.delete(event);
    });
  }),
}));

vi.mock("@/services/ipc/commands", () => ({
  listChatSessions: vi.fn().mockResolvedValue({
    sessions: [
      { sessionId: "s1", preview: "会话一" },
      { sessionId: "s2", preview: "会话二" },
    ],
  }),
  respondAskUser: vi.fn().mockResolvedValue(undefined),
  respondPathPermission: vi.fn().mockResolvedValue(undefined),
  respondToolApproval: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@/commands/workspace", () => ({
  listWorkspaces: vi.fn().mockResolvedValue([]),
}));

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

describe("useDesktopPetInteractions queue system", () => {
  beforeEach(() => {
    listeners.clear();
    vi.clearAllMocks();
  });

  it("queues multiple concurrent interactions from different sessions without overwriting", async () => {
    const petSize = ref<"small" | "medium" | "large">("medium");
    const [manager, app] = withSetup(() => useDesktopPetInteractions({ petSize }));

    expect(manager.activeInteraction.value).toBeNull();
    expect(manager.interactionCount.value).toBe(0);

    // 1. Session 1 emits askUser
    const askHandler = listeners.get(IPC_EVENTS.askUser);
    expect(askHandler).toBeDefined();
    await askHandler!({
      payload: {
        requestId: "req-1",
        sessionId: "s1",
        questions: [{ question: "问题 1", options: [] }],
      },
    });

    expect(manager.interactionCount.value).toBe(1);
    expect(manager.activeInteraction.value?.requestId).toBe("req-1");
    expect(manager.activeInteraction.value?.sessionTitle).toBe("会话一");

    // 2. Session 2 emits toolApproval simultaneously
    const toolHandler = listeners.get(IPC_EVENTS.toolApproval);
    expect(toolHandler).toBeDefined();
    await toolHandler!({
      payload: {
        requestId: "req-2",
        sessionId: "s2",
        toolName: "run_terminal",
        title: "执行命令",
      },
    });

    // Both exist in queue! Session 1 was NOT overwritten
    expect(manager.interactionCount.value).toBe(2);
    expect(manager.currentInteractionIndex.value).toBe(0);
    expect(manager.activeInteraction.value?.requestId).toBe("req-1");

    // 3. Navigate queue
    manager.nextInteraction();
    expect(manager.currentInteractionIndex.value).toBe(1);
    expect(manager.activeInteraction.value?.requestId).toBe("req-2");
    expect(manager.activeInteraction.value?.sessionTitle).toBe("会话二");

    manager.prevInteraction();
    expect(manager.currentInteractionIndex.value).toBe(0);
    expect(manager.activeInteraction.value?.requestId).toBe("req-1");

    // 4. Submit answer for session 1 -> automatically advances to session 2
    await manager.submitAskUserAnswer("答案 1");
    expect(manager.interactionCount.value).toBe(1);
    expect(manager.activeInteraction.value?.requestId).toBe("req-2");

    // 5. Submit approval for session 2 -> queue becomes empty
    await manager.submitToolApproval("allow_once");
    expect(manager.interactionCount.value).toBe(0);
    expect(manager.activeInteraction.value).toBeNull();

    app.unmount();
  });

  it("handles external resolution and session cancellation cleanly", async () => {
    const petSize = ref<"small" | "medium" | "large">("medium");
    const [manager, app] = withSetup(() => useDesktopPetInteractions({ petSize }));

    const askHandler = listeners.get(IPC_EVENTS.askUser);
    await askHandler!({
      payload: {
        requestId: "req-A",
        sessionId: "s1",
        questions: [{ question: "QA", options: [] }],
      },
    });
    await askHandler!({
      payload: {
        requestId: "req-B",
        sessionId: "s2",
        questions: [{ question: "QB", options: [] }],
      },
    });

    expect(manager.interactionCount.value).toBe(2);

    // Workbench resolved req-A externally
    const resolvedHandler = listeners.get(IPC_EVENTS.interactionResolved);
    await resolvedHandler!({
      payload: { requestId: "req-A" },
    });

    expect(manager.interactionCount.value).toBe(1);
    expect(manager.activeInteraction.value?.requestId).toBe("req-B");

    // Session s2 cancelled by user -> chatFinished cleans up req-B
    const finishedHandler = listeners.get(IPC_EVENTS.chatFinished);
    await finishedHandler!({
      payload: { sessionId: "s2", finishReason: "cancelled" },
    });

    expect(manager.interactionCount.value).toBe(0);
    expect(manager.activeInteraction.value).toBeNull();

    app.unmount();
  });
});
