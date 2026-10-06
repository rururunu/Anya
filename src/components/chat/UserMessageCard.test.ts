// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import UserMessageCard from "./UserMessageCard.vue";
import type { ChatMessage } from "@/types/chat";

vi.mock("@/stores/setting", () => ({
  useSettingStore: () => ({ language: "en", mcpServers: [] }),
}));
vi.mock("@/stores/plugins", () => ({ usePluginsStore: () => ({ plugins: [] }) }));
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
function card(inlineEdit = true) {
  const wrapper = mount(UserMessageCard, {
    props: { message, sessionId: message.sessionId, canResend: true, busy: false, inlineEdit },
  });
  wrappers.push(wrapper);
  return wrapper;
}
afterEach(() => {
  for (const wrapper of wrappers.splice(0)) wrapper.unmount();
});

describe("inline user message editing", () => {
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
});
