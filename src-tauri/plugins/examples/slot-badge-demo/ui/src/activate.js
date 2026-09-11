/**
 * Reference plugin proving the generic primitives are enough to build a small,
 * opinionated feature without any Anya source change:
 *  - ctx.slots        -> a badge in the composer accessory row ("cat ears" style spot)
 *  - ctx.assets       -> overrides the mascot idle resource key with an inline SVG
 *  - ctx.bus          -> publishes a heartbeat other plugins can subscribe to
 *  - ctx.i18n         -> localizes the badge label
 *  - ctx.home         -> a settings tab on this plugin's own home page (see ui/about.md for the "About" tab)
 *
 * Copy this folder into the user plugins directory and rename `id` to try it.
 */
export async function activate(ctx) {
  ctx.slots.mount("composer.accessory", {
    id: "badge",
    mount(el) {
      const badge = document.createElement("span");
      badge.textContent = ctx.i18n.t("badge.label", "demo", { "zh-CN": "示例", en: "demo" });
      badge.style.cssText =
        "font-size:11px;opacity:.7;padding:2px 6px;border-radius:8px;background:var(--peek-surface-muted,rgba(255,255,255,.08));";
      el.appendChild(badge);
      const stop = setInterval(() => ctx.bus.publish("slot-badge-demo.heartbeat", { at: Date.now() }), 5000);
      return () => clearInterval(stop);
    },
  });

  ctx.assets.register("mascot.idle", {
    kind: "image",
    source: "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCA0OCA0OCI+PGNpcmNsZSBjeD0iMjQiIGN5PSIyNCIgcj0iMjAiIGZpbGw9IiM1YjdjZmEiLz48L3N2Zz4=",
  });

  const unsubscribe = ctx.bus.subscribe("slot-badge-demo.heartbeat", (payload) => {
    console.debug("slot-badge-demo heartbeat", payload);
  });
  ctx.onDeactivate(unsubscribe);

  // No sidebar.addTab at all -- this plugin has no launcher in Anya's chrome.
  // Its settings live on its own home page instead, via ctx.home.
  ctx.home.setSettingsView((el) => {
    el.textContent = ctx.i18n.t(
      "settings.body",
      "This tab is contributed by a plugin via ctx.home.setSettingsView.",
      { "zh-CN": "这个标签页是插件通过 ctx.home.setSettingsView 加进来的。" },
    );
    return () => ctx.home.clearSettingsView();
  });
}

export function deactivate() {}
