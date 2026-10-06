import { describe, expect, it, vi } from "vitest";
import { executeSlashCommand, slashCommands } from "./slash";

vi.mock("@/services/ipc", () => ({ listChatSessions: vi.fn(), openSettings: vi.fn() }));

describe("slash commands", () => {
  it("offers a new conversation action on both composer surfaces", async () => {
    expect(slashCommands[0].command).toBe("/new");
    expect(await executeSlashCommand("/new")).toBe("newConversation");
  });
  it("opens mode and tool permission settings without a conversation", async () => {
    expect(await executeSlashCommand("/mode")).toBe("openMode");
    expect(await executeSlashCommand("/security")).toBe("openSecurity");
  });

  it("removes the environment context command", async () => {
    expect(slashCommands.some((item) => item.command === "/context")).toBe(false);
    expect(await executeSlashCommand("/context")).toBeNull();
  });
  it("routes the two conversation views to distinct open actions", async () => {
    expect(await executeSlashCommand("/popup")).toBe("openPopup");
    expect(await executeSlashCommand("/workbench")).toBe("openWorkbench");
  });
});
