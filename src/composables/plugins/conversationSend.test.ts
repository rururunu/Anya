import { afterEach, describe, expect, it } from "vitest";
import {
  activePluginSessionId,
  bindPluginAgentSession,
  bindPluginConversationSend,
  createPluginAgentSurface,
  embedAgentConversation,
  pluginChatMountEl,
  sendToActiveConversation,
  setPluginChatMount,
} from "@/composables/plugins/conversationSend";

describe("conversationSend", () => {
  afterEach(() => {
    bindPluginConversationSend(null);
    bindPluginAgentSession(null);
    setPluginChatMount(null);
  });

  it("forwards trimmed text to the bound composer submit", async () => {
    const seen: string[] = [];
    bindPluginConversationSend(async (text) => {
      seen.push(text);
    });
    await sendToActiveConversation("  hello  ");
    expect(seen).toEqual(["hello"]);
  });

  it("rejects empty text and an unbound composer", async () => {
    await expect(sendToActiveConversation("   ")).rejects.toThrow("empty");
    await expect(sendToActiveConversation("hi")).rejects.toThrow("not bound");
  });

  it("records a plugin page node as the live chat mount", () => {
    const el = {} as HTMLElement;
    setPluginChatMount(el);
    expect(pluginChatMountEl.value).toBe(el);
    setPluginChatMount(null);
    expect(pluginChatMountEl.value).toBeNull();
  });

  it("binds sessionId and shares send/mount on ctx.agent", async () => {
    const seen: string[] = [];
    bindPluginConversationSend(async (text) => {
      seen.push(text);
    });
    bindPluginAgentSession(() => "sess-1");
    const cleanups: Array<() => void> = [];
    const agent = createPluginAgentSurface("demo", (fn) => cleanups.push(fn));
    const el = {} as HTMLElement;
    const off = agent.mount(el);
    expect(pluginChatMountEl.value).toBe(el);
    expect(agent.sessionId()).toBe("sess-1");
    await agent.send("  ping  ");
    expect(seen).toEqual(["ping"]);
    off();
    expect(pluginChatMountEl.value).toBeNull();
    expect(activePluginSessionId()).toBe("sess-1");
  });

  it("embedAgentConversation only clears its own node", () => {
    const a = {} as HTMLElement;
    const b = {} as HTMLElement;
    const offA = embedAgentConversation(a);
    embedAgentConversation(b);
    offA();
    expect(pluginChatMountEl.value).toBe(b);
  });
});
