/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createApp, defineComponent } from "vue";
import {
  resolveAgentExpression,
  useDesktopPetAgentSessions,
  type SessionState,
} from "./useDesktopPetAgentSessions";

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

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

describe("resolveAgentExpression", () => {
  it("resolves idle when no sessions and no transient state", () => {
    expect(resolveAgentExpression([], null)).toBe("idle");
  });

  it("resolves transient state when no active sessions", () => {
    expect(resolveAgentExpression([], "done")).toBe("done");
    expect(resolveAgentExpression([], "error")).toBe("error");
  });

  it("prioritizes working over talking and thinking", () => {
    const s1: SessionState = {
      sessionId: "s1",
      status: "working",
      activeTools: new Set(["tool-1"]),
      lastActiveAt: Date.now(),
    };
    const s2: SessionState = {
      sessionId: "s2",
      status: "talking",
      activeTools: new Set(),
      lastActiveAt: Date.now(),
    };
    const s3: SessionState = {
      sessionId: "s3",
      status: "thinking",
      activeTools: new Set(),
      lastActiveAt: Date.now(),
    };

    expect(resolveAgentExpression([s1, s2, s3], null)).toBe("working");
    expect(resolveAgentExpression([s2, s3], null)).toBe("talking");
    expect(resolveAgentExpression([s3], null)).toBe("thinking");
  });

  it("active sessions override transient done/error", () => {
    const s1: SessionState = {
      sessionId: "s1",
      status: "working",
      activeTools: new Set(["tool-1"]),
      lastActiveAt: Date.now(),
    };
    expect(resolveAgentExpression([s1], "done")).toBe("working");
  });
});

describe("useDesktopPetAgentSessions", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("handles complete single-session lifecycle with tool execution", async () => {
    const [manager, app] = withSetup(() => useDesktopPetAgentSessions());

    expect(manager.agentExpression.value).toBe("idle");

    // Chat started
    manager.handleChatStarted({ sessionId: "sess-1" });
    expect(manager.agentExpression.value).toBe("thinking");

    // Tool started
    manager.handleToolStarted({ sessionId: "sess-1", activityId: "act-1", toolName: "read_file" });
    expect(manager.agentExpression.value).toBe("working");

    // Tool finished -> transitions back to thinking
    manager.handleToolFinished({ sessionId: "sess-1", activityId: "act-1" });
    expect(manager.agentExpression.value).toBe("thinking");

    // Delta stream -> talking
    manager.handleChatDelta({ sessionId: "sess-1", delta: "Hello!" });
    expect(manager.agentExpression.value).toBe("talking");

    // Chat finished normally -> done -> idle after timeout
    manager.handleChatFinished({ sessionId: "sess-1", finishReason: "stop" });
    expect(manager.agentExpression.value).toBe("done");

    await vi.advanceTimersByTimeAsync(3600);
    expect(manager.agentExpression.value).toBe("idle");

    app.unmount();
  });

  it("immediately returns to idle when session is cancelled/stopped", async () => {
    const [manager, app] = withSetup(() => useDesktopPetAgentSessions());

    manager.handleChatStarted({ sessionId: "sess-1" });
    manager.handleToolStarted({
      sessionId: "sess-1",
      activityId: "act-1",
      toolName: "run_command",
    });
    expect(manager.agentExpression.value).toBe("working");

    // User clicked Stop -> cancelled
    manager.handleChatFinished({ sessionId: "sess-1", finishReason: "cancelled" });
    expect(manager.agentExpression.value).toBe("idle");

    app.unmount();
  });

  it("supports concurrent multi-session priority without trampling states", async () => {
    const [manager, app] = withSetup(() => useDesktopPetAgentSessions());

    // Session A starts working
    manager.handleChatStarted({ sessionId: "sess-A" });
    manager.handleToolStarted({ sessionId: "sess-A", activityId: "act-A", toolName: "build" });
    expect(manager.agentExpression.value).toBe("working");

    // Session B starts and streams
    manager.handleChatStarted({ sessionId: "sess-B" });
    manager.handleChatDelta({ sessionId: "sess-B", delta: "Processing" });
    // Still working because working > talking
    expect(manager.agentExpression.value).toBe("working");

    // Session A tool finishes and session A finishes
    manager.handleToolFinished({ sessionId: "sess-A", activityId: "act-A" });
    manager.handleChatFinished({ sessionId: "sess-A", finishReason: "stop" });

    // Session B is still running! Pet must display Session B (talking), NOT done!
    expect(manager.agentExpression.value).toBe("talking");

    // Now Session B finishes
    manager.handleChatFinished({ sessionId: "sess-B", finishReason: "stop" });
    // All sessions finished -> show done
    expect(manager.agentExpression.value).toBe("done");

    await vi.advanceTimersByTimeAsync(3600);
    expect(manager.agentExpression.value).toBe("idle");

    app.unmount();
  });

  it("handles multi-tool parallel execution within a single session", async () => {
    const [manager, app] = withSetup(() => useDesktopPetAgentSessions());

    manager.handleChatStarted({ sessionId: "sess-1" });
    manager.handleToolStarted({ sessionId: "sess-1", activityId: "t1" });
    manager.handleToolStarted({ sessionId: "sess-1", activityId: "t2" });
    expect(manager.agentExpression.value).toBe("working");

    // First tool finishes, second still active
    manager.handleToolFinished({ sessionId: "sess-1", activityId: "t1" });
    expect(manager.agentExpression.value).toBe("working");

    // Second tool finishes -> back to thinking
    manager.handleToolFinished({ sessionId: "sess-1", activityId: "t2" });
    expect(manager.agentExpression.value).toBe("thinking");

    app.unmount();
  });

  it("watchdog cleans up zombie session exceeding timeout", async () => {
    const [manager, app] = withSetup(() => useDesktopPetAgentSessions({ sessionTimeoutMs: 1000 }));

    manager.handleChatStarted({ sessionId: "zombie" });
    manager.handleToolStarted({ sessionId: "zombie", activityId: "t1" });
    expect(manager.agentExpression.value).toBe("working");

    // Advance time past 1000ms and run watchdog
    await vi.advanceTimersByTimeAsync(1100);
    manager.runWatchdog();

    expect(manager.agentExpression.value).toBe("idle");

    app.unmount();
  });
});
