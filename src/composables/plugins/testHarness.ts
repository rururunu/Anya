import type { PluginActivateContext, PluginMount } from "@/composables/plugins/sdk";

/**
 * Builds a minimal fake `ctx` for unit-testing `activate`/`deactivate` without
 * booting Vue/Pinia or the real plugin runtime. Records every call so a test
 * can assert which contract methods a plugin used.
 */
export function createFakePluginContext(pluginId = "test-plugin") {
  const calls: string[] = [];
  const deactivateFns: Array<() => void> = [];

  const ctx = {
    pluginId,
    app: undefined,
    pinia: undefined,
    sidebar: {
      addTab: (tab: { id: string; title: string; icon?: string; mount: PluginMount }) => {
        calls.push(`sidebar.addTab:${tab.id}`);
      },
      removeTab: (id: string) => calls.push(`sidebar.removeTab:${id}`),
    },
    workbench: {
      setView: (view: { id: string; title: string; mount: PluginMount } | null) =>
        calls.push(`workbench.setView:${view?.id ?? "null"}`),
    },
    composer: {
      addAccessory: (item: { id: string; mount: PluginMount }) =>
        calls.push(`composer.addAccessory:${item.id}`),
      removeAccessory: (id: string) => calls.push(`composer.removeAccessory:${id}`),
    },
    conversation: {
      addMaterials: (item: { id: string; mount: PluginMount }) =>
        calls.push(`conversation.addMaterials:${item.id}`),
      removeMaterials: (id: string) => calls.push(`conversation.removeMaterials:${id}`),
      mount: (el: HTMLElement) => {
        calls.push("conversation.mount");
        void el;
        return () => calls.push("conversation.unmount");
      },
      unmount: () => calls.push("conversation.unmount"),
      send: async (text: string) => {
        calls.push(`conversation.send:${text.slice(0, 40)}`);
      },
      run: async (prompt: string) => {
        calls.push(`conversation.run:${prompt.slice(0, 40)}`);
        return { sessionId: "plugin:test-plugin:test" };
      },
      sessionId: () => {
        calls.push("conversation.sessionId");
        return "test-session";
      },
    },
    workspace: {
      openInTerminal: async (id?: string) => {
        calls.push(`workspace.openInTerminal:${id ?? ""}`);
      },
    },
    agent: {
      mount: (el: HTMLElement) => {
        calls.push("agent.mount");
        void el;
        return () => calls.push("agent.unmount");
      },
      unmount: () => calls.push("agent.unmount"),
      send: async (text: string) => {
        calls.push(`agent.send:${text.slice(0, 40)}`);
      },
      run: async (prompt: string) => {
        calls.push(`agent.run:${prompt.slice(0, 40)}`);
        return { sessionId: "plugin:test-plugin:test" };
      },
      sessionId: () => {
        calls.push("agent.sessionId");
        return "test-session";
      },
    },
    slots: {
      list: () => [],
      mount: (anchorId: string, view: { id: string }) => {
        calls.push(`slots.mount:${anchorId}:${view.id}`);
        return true;
      },
      unmount: (anchorId: string, id: string) => calls.push(`slots.unmount:${anchorId}:${id}`),
    },
    assets: {
      list: () => [],
      register: (key: string) => calls.push(`assets.register:${key}`),
      unregister: (key: string) => calls.push(`assets.unregister:${key}`),
    },
    bus: {
      publish: (topic: string) => calls.push(`bus.publish:${topic}`),
      subscribe: (topic: string, fn: (payload: unknown) => void) => {
        calls.push(`bus.subscribe:${topic}`);
        void fn;
        return () => calls.push(`bus.unsubscribe:${topic}`);
      },
    },
    i18n: { t: (_key: string, fallback: string) => fallback },
    stores: { chat: undefined, setting: undefined },
    host: {
      rpc: async () => undefined,
      on: () => () => {},
    },
    fs: {
      pick: async () => {
        calls.push("fs.pick");
        return null;
      },
    },
    onDeactivate: (fn: () => void) => deactivateFns.push(fn),
  } as unknown as PluginActivateContext;

  return {
    ctx,
    calls,
    runDeactivateHooks: () => deactivateFns.splice(0).forEach((fn) => fn()),
  };
}
