// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import UserMessageCard from "./UserMessageCard.vue";
import type { ChatMessage } from "@/types/chat";

const mocked = vi.hoisted(() => ({
  settings: {
    language: "en",
    mcpServers: [{ id: "example", title: "Example MCP", enabled: true }],
    enabledBuiltinSkills: [] as string[],
  },
  plugins: {
    plugins: [{ id: "computer-use", name: "Computer Use", enabled: true, role: "agent" }],
    pluginsLoaded: true,
    refresh: vi.fn().mockResolvedValue(undefined),
  },
}));
vi.mock("@/stores/setting", () => ({
  useSettingStore: () => mocked.settings,
}));
vi.mock("@/stores/plugins", () => ({
  usePluginsStore: () => mocked.plugins,
}));
vi.mock("@/commands/skills", () => ({ listSkills: vi.fn().mockResolvedValue([]) }));
vi.mock("@/commands/workspace", () => ({
  listWorkspaceFiles: vi.fn().mockResolvedValue(["src/main.ts"]),
}));
vi.mock("@/components/chat/UserMessageFooter.vue", () => ({
  default: { template: '<div class="full-editor-footer" />' },
}));

const message: ChatMessage = {
  id: "user-1",
  sessionId: "session-1",
  role: "user",
  content: "Original message",
  status: "done",
  timestamp: 1,
};
const wrappers: ReturnType<typeof mount>[] = [];
function card(inlineEdit = true, content = message.content) {
  const wrapper = mount(UserMessageCard, {
    props: {
      message: { ...message, content },
      sessionId: message.sessionId,
      canResend: true,
      busy: false,
      inlineEdit,
    },
  });
  wrappers.push(wrapper);
  return wrapper;
}
async function typeInEdit(wrapper: ReturnType<typeof mount>, text: string) {
  const field = wrapper.get("[contenteditable='true']");
  field.element.textContent = text;
  await field.trigger("input");
  await flushPromises();
}
afterEach(() => {
  for (const wrapper of wrappers.splice(0)) wrapper.unmount();
});

describe("inline user message editing", () => {
  it("shows the full user message in the overlay while keeping workbench folding", () => {
    const longText = Array.from({ length: 14 }, (_, index) => `Line ${index + 1}`).join("\n");
    const overlay = card(true, longText);
    expect(overlay.get(".user-composer").classes()).not.toContain("is-collapsed");
    expect(overlay.find(".user-bubble-toggle").exists()).toBe(false);
    expect(overlay.get(".user-message-text").text()).toBe(longText);

    const workbench = card(false, longText);
    expect(workbench.get(".user-composer").classes()).toContain("is-collapsed");
    expect(workbench.find(".user-bubble-toggle").exists()).toBe(true);
  });

  it("does not expose a sent-time tooltip on workbench messages", () => {
    const wrapper = card(false);
    expect(wrapper.get(".user-composer").attributes("title")).not.toBe(
      new Date(message.timestamp).toLocaleString("en"),
    );
  });
  it("allows queued messages to be edited, removed or used as guidance", async () => {
    const wrapper = card();
    await wrapper.setProps({ queued: true });
    const actions = wrapper.findAll(".queued-message-actions button");
    expect(wrapper.find(".user-rewind-btn").exists()).toBe(false);
    expect(wrapper.get(".queued-message-actions").text()).toBe("");
    expect(actions.map((action) => action.attributes("aria-label"))).toEqual(["Guide", "Delete"]);
    expect(actions[0]!.find(".lucide-corner-down-left").exists()).toBe(true);
    expect(wrapper.find(".queued-state .lucide-clock-3").exists()).toBe(true);
    await actions[0]!.trigger("click");
    await actions[1]!.trigger("click");
    expect(wrapper.emitted("remove")).toEqual([[]]);
    expect(wrapper.emitted("guide")).toEqual([[]]);
    await wrapper.get(".user-message-text").trigger("click");
    await flushPromises();
    const field = wrapper.get("[contenteditable='true']");
    field.element.textContent = "Updated queued message";
    await field.trigger("input");
    await field.trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("resend")).toEqual([["Updated queued message"]]);
    expect(wrapper.find("[contenteditable='true']").exists()).toBe(false);
  });

  it("edits the existing text and resends it with Enter without opening the full composer footer", async () => {
    const wrapper = card();
    await wrapper.get(".user-message-text").trigger("click");
    await flushPromises();
    expect(wrapper.find(".full-editor-footer").exists()).toBe(false);
    expect(wrapper.find(".user-message-text").exists()).toBe(false);
    expect(wrapper.get(".user-composer-body").text()).toBe("Original message");
    const field = wrapper.get("[contenteditable='true']");
    field.element.textContent = "Edited message";
    await field.trigger("input");
    await field.trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("resend")).toEqual([["Edited message"]]);
  });

  it("cancels edits with Escape and keeps the original message", async () => {
    const wrapper = card();
    await wrapper.get(".user-message-text").trigger("click");
    await flushPromises();
    const field = wrapper.get("[contenteditable='true']");
    field.element.textContent = "Unsent edit";
    await field.trigger("input");
    await field.trigger("keydown", { key: "Escape" });
    expect(wrapper.get(".user-message-text").text()).toBe(message.content);
    expect(wrapper.emitted("resend")).toBeUndefined();
  });

  it("preserves the workbench editor and rejects an empty resend", async () => {
    const wrapper = card(false);
    await wrapper.get(".user-message-text").trigger("click");
    await flushPromises();
    expect(wrapper.find(".full-editor-footer").exists()).toBe(true);
    const field = wrapper.get("[contenteditable='true']");
    field.element.textContent = "   ";
    await field.trigger("input");
    await field.trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("resend")).toBeUndefined();
  });

  it.each([true, false])(
    "shows # resources and / commands while editing (inline=%s)",
    async (inline) => {
      const wrapper = card(inline);
      await wrapper.get(".user-message-text").trigger("click");
      await flushPromises();
      await typeInEdit(wrapper, "Try #");
      expect(wrapper.get(".hash-suggestion-list").text()).toContain("Computer Use");
      expect(wrapper.get(".hash-suggestion-list").text()).toContain("Example MCP");
      await typeInEdit(wrapper, "Try /");
      expect(wrapper.get(".user-edit-suggestions").text()).toContain("/new");
    },
  );

  it("shows workspace files after @ while editing", async () => {
    const wrapper = card(false);
    await wrapper.setProps({ workspaceRoot: "workspace-1" });
    await wrapper.get(".user-message-text").trigger("click");
    await flushPromises();
    await typeInEdit(wrapper, "Use @");
    expect(wrapper.get(".file-suggestion-list").text()).toContain("src/main.ts");
  });

  it("scrolls the edit suggestions with keyboard selection", async () => {
    const wrapper = card();
    await wrapper.get(".user-message-text").trigger("click");
    await flushPromises();
    await typeInEdit(wrapper, "Try #");
    const list = wrapper.get(".hash-suggestion-list").element as HTMLElement;
    Object.defineProperty(list, "clientHeight", { configurable: true, value: 20 });
    const rows = wrapper.findAll(".hash-suggestion-item");
    Object.defineProperty(rows[1]!.element, "offsetTop", { configurable: true, value: 35 });
    Object.defineProperty(rows[1]!.element, "offsetHeight", { configurable: true, value: 30 });
    await wrapper.get("[contenteditable='true']").trigger("keydown", { key: "ArrowDown" });
    await flushPromises();
    expect(rows[1]!.classes()).toContain("active");
    expect(list.scrollTop).toBe(45);
  });
});
