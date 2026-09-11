import { beforeEach, describe, expect, it, vi } from "vitest";

const getCurrentWorkspace = vi.fn();
const openWorkspaceInTerminal = vi.fn();
const activePluginSessionId = vi.fn(() => "");

vi.mock("@/commands/workspace", () => ({
  getCurrentWorkspace: (...args: unknown[]) => getCurrentWorkspace(...args),
  openWorkspaceInTerminal: (...args: unknown[]) => openWorkspaceInTerminal(...args),
}));

vi.mock("@/composables/plugins/conversationSend", () => ({
  activePluginSessionId: () => activePluginSessionId(),
}));

vi.mock("@/stores/chat", () => ({
  useChatStore: () => ({ sessionCompose: {} }),
}));

vi.mock("@/stores/chatSessions", () => ({
  useChatSessionsStore: () => ({ summaries: [] }),
}));

import {
  openPluginWorkspaceInTerminal,
  resolvePluginWorkspaceId,
} from "@/composables/plugins/workspace";

describe("plugin workspace surface", () => {
  beforeEach(() => {
    getCurrentWorkspace.mockReset();
    openWorkspaceInTerminal.mockReset();
    activePluginSessionId.mockReturnValue("");
  });

  it("uses an explicit workspace id", async () => {
    openWorkspaceInTerminal.mockResolvedValue(undefined);
    await openPluginWorkspaceInTerminal("ws-1");
    expect(openWorkspaceInTerminal).toHaveBeenCalledWith("ws-1");
    expect(getCurrentWorkspace).not.toHaveBeenCalled();
  });

  it("falls back to Anya's current workspace", async () => {
    getCurrentWorkspace.mockResolvedValue({ id: "ws-current" });
    await expect(resolvePluginWorkspaceId()).resolves.toBe("ws-current");
  });

  it("throws when no workspace is active", async () => {
    getCurrentWorkspace.mockResolvedValue(null);
    await expect(resolvePluginWorkspaceId()).rejects.toThrow("No active workspace");
  });
});
