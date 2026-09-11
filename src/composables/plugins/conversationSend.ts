import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { shallowRef } from "vue";

import {
  runPluginAgent,
  type PluginAgentRunOptions,
  type PluginAgentRunResult,
} from "@/composables/plugins/agentRun";

export type { PluginAgentRunOptions, PluginAgentRunResult };

type ConversationSend = (text: string) => Promise<void>;

let sendImpl: ConversationSend | null = null;
let askUnlisten: UnlistenFn | null = null;
let askListenStarted = false;

/** Plugin-owned node that should host Anya's live agent conversation UI. */
export const pluginChatMountEl = shallowRef<HTMLElement | null>(null);

/** Embed (or clear) the workbench agent conversation into a plugin page. */
export function setPluginChatMount(el: HTMLElement | null) {
  pluginChatMountEl.value = el;
}

/** Teleport the live conversation into `el`; returned fn clears only this mount. */
export function embedAgentConversation(el: HTMLElement): () => void {
  setPluginChatMount(el);
  return () => {
    if (pluginChatMountEl.value === el) setPluginChatMount(null);
  };
}

export type PluginAgentSurface = {
  mount: (el: HTMLElement) => () => void;
  unmount: () => void;
  send: (text: string) => Promise<void>;
  run: (prompt: string, options?: PluginAgentRunOptions) => Promise<PluginAgentRunResult>;
  sessionId: () => string;
};

/** Shared `ctx.agent` / `ctx.conversation` surface (conversation keeps extra aliases). */
export function createPluginAgentSurface(
  pluginId: string,
  onCleanup: (fn: () => void) => void,
): PluginAgentSurface {
  return {
    mount(el) {
      const off = embedAgentConversation(el);
      onCleanup(off);
      return off;
    },
    unmount() {
      setPluginChatMount(null);
    },
    send: (text) => sendToActiveConversation(text),
    run: (prompt, options) => runPluginAgent(pluginId, prompt, options),
    sessionId: () => activePluginSessionId(),
  };
}

/** Workbench registers `submitMessage` so plugins can inject into the live chat. */
export function bindPluginConversationSend(fn: ConversationSend | null) {
  sendImpl = fn;
}

/** Send text into the current workbench conversation (same path as the composer). */
export async function sendToActiveConversation(text: string): Promise<void> {
  const trimmed = text.trim();
  if (!trimmed) throw new Error("prompt is empty");
  if (!sendImpl) throw new Error("conversation send is not bound");
  await sendImpl(trimmed);
}

/** Isolated-window `AnyaPlugin.askAnya` lands here; forward it into the chat. */
export async function listenPluginAskAnya(): Promise<void> {
  if (askListenStarted) return;
  askListenStarted = true;
  askUnlisten = await listen<{ pluginId?: string; prompt?: string }>("plugin-ask-anya", (event) => {
    const prompt = event.payload?.prompt;
    if (typeof prompt === "string" && prompt.trim()) {
      void sendToActiveConversation(prompt).catch(() => {
        /* composer not ready */
      });
    }
  });
}

/** Drop the isolated-window ask-anya listener (tests / workbench teardown). */
export function stopListeningPluginAskAnya() {
  askUnlisten?.();
  askUnlisten = null;
  askListenStarted = false;
}

let sessionIdImpl: (() => string) | null = null;

/** Workbench registers the active session so plugins can read `ctx.agent.sessionId()`. */
export function bindPluginAgentSession(fn: (() => string) | null) {
  sessionIdImpl = fn;
}

/** Current workbench session id, or empty if none. */
export function activePluginSessionId(): string {
  return sessionIdImpl?.() ?? "";
}
