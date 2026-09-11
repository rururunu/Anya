import { beforeEach, describe, expect, it, vi } from "vitest";

const send = vi.fn();
const markSessionStarted = vi.fn();
const setSessionMessages = vi.fn();
const ensureCompose = vi.fn();
const setCompose = vi.fn();
const getCurrentWorkspace = vi.fn();
const listWorkspaces = vi.fn();
const createWorkspace = vi.fn();
const sessions: Record<string, unknown[]> = {};

vi.mock("@/commands/workspace", () => ({
  getCurrentWorkspace: (...args: unknown[]) => getCurrentWorkspace(...args),
  listWorkspaces: (...args: unknown[]) => listWorkspaces(...args),
  createWorkspace: (...args: unknown[]) => createWorkspace(...args),
}));

vi.mock("@/stores/chat", () => ({
  useChatStore: () => ({
    send: (...args: unknown[]) => send(...args),
    markSessionStarted,
    setSessionMessages,
    ensureCompose,
    setCompose,
  }),
}));

vi.mock("@/stores/chatSessions", () => ({
  useChatSessionsStore: () => ({ sessions }),
}));

import { runPluginAgent } from "@/composables/plugins/agentRun";

describe("runPluginAgent", () => {
  beforeEach(() => {
    send.mockReset();
    markSessionStarted.mockReset();
    setSessionMessages.mockReset();
    ensureCompose.mockReset();
    setCompose.mockReset();
    getCurrentWorkspace.mockReset();
    listWorkspaces.mockReset();
    createWorkspace.mockReset();
    for (const key of Object.keys(sessions)) delete sessions[key];
    send.mockResolvedValue(true);
    getCurrentWorkspace.mockResolvedValue({ id: "ws-1" });
    listWorkspaces.mockResolvedValue([]);
    createWorkspace.mockResolvedValue({ id: "C:/repo" });
  });

  it("sends through the chat store on an isolated plugin session", async () => {
    const result = await runPluginAgent("group", "  hello  ", { sessionId: "room" });
    expect(result.sessionId).toBe("plugin:group:room");
    expect(markSessionStarted).toHaveBeenCalledWith("plugin:group:room");
    expect(setCompose).toHaveBeenCalledWith("plugin:group:room", { chatMode: "agent" });
    expect(send).toHaveBeenCalledWith("hello", "plugin:group:room", {
      isolate: true,
      workspaceId: "ws-1",
      quickAsk: false,
    });
  });

  it("does not touch the workbench overlay session", async () => {
    await runPluginAgent("group", "hi");
    const options = send.mock.calls[0]?.[2] as { isolate?: boolean };
    expect(options.isolate).toBe(true);
  });

  it("binds tools to cwd without switching via getCurrentWorkspace", async () => {
    listWorkspaces.mockResolvedValue([{ id: "C:\\repo", root: "C:\\repo" }]);
    await runPluginAgent("demo", "fix tests", {
      sessionId: "cwd-1",
      cwd: "C:/repo",
    });
    expect(createWorkspace).not.toHaveBeenCalled();
    expect(getCurrentWorkspace).not.toHaveBeenCalled();
    expect(send).toHaveBeenCalledWith("fix tests", "plugin:demo:cwd-1", {
      isolate: true,
      workspaceId: "C:\\repo",
      quickAsk: false,
    });
  });

  it("creates a workspace when cwd is a new folder", async () => {
    listWorkspaces.mockResolvedValue([]);
    createWorkspace.mockResolvedValue({ id: "D:/proj" });
    await runPluginAgent("demo", "hello", { cwd: "D:/proj" });
    expect(createWorkspace).toHaveBeenCalledWith("D:/proj");
    expect(send.mock.calls[0]?.[2]).toMatchObject({
      workspaceId: "D:/proj",
      quickAsk: false,
    });
  });
});
