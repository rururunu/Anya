// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { shallowMount } from "@vue/test-utils";
import MessageList from "./MessageList.vue";
import UserMessageCard from "./UserMessageCard.vue";
import { useChatStore } from "@/stores/chat";
import type { ChatMessage } from "@/types/chat";

vi.mock("@/services/ipc", () => ({
  chatCancel: vi.fn(),
  openSettings: vi.fn(),
  rewindSession: vi.fn(),
  setPlanMode: vi.fn(),
}));
const wrappers: ReturnType<typeof shallowMount>[] = [];
const user: ChatMessage = {
  id: "request",
  sessionId: "conversation",
  role: "user",
  content: "Original request",
  status: "done",
  timestamp: 1,
  fromQueue: true,
};
const assistant: ChatMessage = {
  id: "reply",
  sessionId: "conversation",
  role: "assistant",
  content: "Working",
  status: "streaming",
  timestamp: 2,
};

beforeEach(() => {
  setActivePinia(createPinia());
});
afterEach(() => {
  for (const wrapper of wrappers.splice(0)) wrapper.unmount();
});

describe("overlay queued conversation", () => {
  it("does not expose a sent-time tooltip on workbench assistant replies", () => {
    const wrapper = shallowMount(MessageList, {
      props: { messages: [user, assistant], sessionId: user.sessionId },
    });
    wrappers.push(wrapper);
    expect(wrapper.get(".message-item.assistant").attributes("title")).toBeUndefined();
  });
  it("does not show reply references or queue highlighting for ordinary messages", () => {
    const wrapper = shallowMount(MessageList, {
      props: {
        messages: [{ ...user, fromQueue: false }, assistant],
        sessionId: user.sessionId,
        inlineConversation: true,
      },
    });
    wrappers.push(wrapper);
    expect(wrapper.find(".reply-message-reference").exists()).toBe(false);
    expect(wrapper.find(".reply-target").exists()).toBe(false);
  });
  it("renders pending messages as user rows and routes their edit, delete and guide actions", async () => {
    const store = useChatStore();
    store.setStagedLocal(user.sessionId, ["Next request"]);
    const edit = vi.spyOn(store, "editStagedMessage").mockResolvedValue();
    const remove = vi.spyOn(store, "removeStagedMessage").mockResolvedValue();
    const guide = vi.spyOn(store, "guideStagedMessage").mockResolvedValue();
    const wrapper = shallowMount(MessageList, {
      props: { messages: [user, assistant], sessionId: user.sessionId, inlineConversation: true },
    });
    wrappers.push(wrapper);
    expect(wrapper.find(".reply-target[data-message-id='request']").exists()).toBe(true);
    expect(wrapper.get(".reply-message-reference").text()).toContain(user.content);
    const queued = wrapper.get(".queued-user-message").getComponent(UserMessageCard);
    expect(queued.props("queued")).toBe(true);
    queued.vm.$emit("resend", "Edited request");
    queued.vm.$emit("remove");
    queued.vm.$emit("guide");
    expect(edit).toHaveBeenCalledWith(user.sessionId, 0, "Next request", "Edited request");
    expect(remove).toHaveBeenCalledWith(user.sessionId, 0);
    expect(guide).toHaveBeenCalledWith(user.sessionId, 0);
  });

  it("places guidance below the original request and keeps the reply linked to that request", async () => {
    const guidance: ChatMessage = {
      ...user,
      id: "guidance",
      content: "Please focus on tests",
      injected: true,
      timestamp: 3,
    };
    const wrapper = shallowMount(MessageList, {
      props: {
        messages: [user, assistant],
        sessionId: user.sessionId,
        inlineConversation: true,
      },
    });
    wrappers.push(wrapper);
    await wrapper.setProps({ messages: [user, assistant, guidance] });
    const rows = wrapper.findAll("article[data-message-id]");
    expect(rows.map((row) => row.attributes("data-message-id"))).toEqual([
      "request",
      "guidance",
      "reply",
    ]);
    expect(wrapper.get(".reply-message-reference").text()).toContain(user.content);
    expect(wrapper.find(".message-item.inject").exists()).toBe(false);
  });
});
