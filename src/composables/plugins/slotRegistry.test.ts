import { beforeEach, describe, expect, it } from "vitest";
import {
  contentForSurface,
  getSlotConflicts,
  getSlotEntries,
  mountSlot,
  tabIsChromeAction,
  tabOpensMainPane,
  unmountPluginFromAllAnchors,
} from "@/composables/plugins/slotRegistry";

describe("slotRegistry governance", () => {
  beforeEach(() => {
    unmountPluginFromAllAnchors("plugin-a");
    unmountPluginFromAllAnchors("plugin-b");
  });

  it("stacks conversation materials above the live chat", () => {
    mountSlot("conversation.materials", { id: "a:mat", pluginId: "plugin-a", mount: () => {} });
    mountSlot("conversation.materials", { id: "b:mat", pluginId: "plugin-b", mount: () => {} });
    expect(getSlotEntries("conversation.materials").map((e) => e.pluginId)).toEqual([
      "plugin-a",
      "plugin-b",
    ]);
  });

  it("stacks multiple plugins on a stack anchor", () => {
    mountSlot("sidebar.tabs", { id: "a:1", pluginId: "plugin-a", mount: () => {} });
    mountSlot("sidebar.tabs", { id: "b:1", pluginId: "plugin-b", mount: () => {} });
    expect(getSlotEntries("sidebar.tabs").map((e) => e.pluginId)).toEqual(["plugin-a", "plugin-b"]);
  });

  it("rejects a second plugin on an exclusive anchor and records a conflict", () => {
    const first = mountSlot("workbench.main", {
      id: "a:main",
      pluginId: "plugin-a",
      mount: () => {},
    });
    const second = mountSlot("workbench.main", {
      id: "b:main",
      pluginId: "plugin-b",
      mount: () => {},
    });
    expect(first).toBe(true);
    expect(second).toBe(false);
    expect(getSlotEntries("workbench.main").map((e) => e.pluginId)).toEqual(["plugin-a"]);
    expect(getSlotConflicts().at(-1)).toMatchObject({
      anchorId: "workbench.main",
      ownerPluginId: "plugin-a",
      rejectedPluginId: "plugin-b",
    });
  });

  it("defaults omitted chrome to views and keeps an explicit empty list", () => {
    mountSlot("sidebar.tabs", { id: "a:1", pluginId: "plugin-a", mount: () => {} });
    expect(getSlotEntries("sidebar.tabs")[0]?.chrome).toEqual(["views"]);
    mountSlot("sidebar.tabs", {
      id: "a:2",
      pluginId: "plugin-a",
      mount: () => {},
      chrome: [],
    });
    expect(getSlotEntries("sidebar.tabs").find((e) => e.id === "a:2")?.chrome).toEqual([]);
    mountSlot("sidebar.tabs", {
      id: "a:3",
      pluginId: "plugin-a",
      mount: () => {},
      chrome: ["nav", "header", "bogus"] as never,
    });
    expect(getSlotEntries("sidebar.tabs").find((e) => e.id === "a:3")?.chrome).toEqual([
      "nav",
      "header",
    ]);
  });

  it("throws for an unknown anchor id", () => {
    expect(() =>
      mountSlot("nonexistent.anchor", { id: "x", pluginId: "plugin-a", mount: () => {} }),
    ).toThrow();
  });

  it("keeps `header` chrome in the normalized surfaces list", () => {
    mountSlot("sidebar.tabs", {
      id: "a:1",
      pluginId: "plugin-a",
      mount: () => {},
      chrome: ["header"],
    });
    expect(getSlotEntries("sidebar.tabs").find((e) => e.id === "a:1")?.chrome).toEqual(["header"]);
  });

  it("resolves per-surface content, falling back to mount", () => {
    const viewsMount = () => {};
    const headerMount = () => {};
    mountSlot("sidebar.tabs", {
      id: "a:1",
      pluginId: "plugin-a",
      mount: viewsMount,
      chrome: ["views", "header"],
      content: { header: headerMount },
    });
    const entry = getSlotEntries("sidebar.tabs").find((e) => e.id === "a:1")!;
    expect(contentForSurface(entry, "header")).toBe(headerMount);
    expect(contentForSurface(entry, "views")).toBe(viewsMount);
  });

  it("treats nav-without-views as the center main pane, not the review strip", () => {
    expect(tabOpensMainPane(["nav"])).toBe(true);
    expect(tabOpensMainPane(["nav", "header"])).toBe(true);
    expect(tabOpensMainPane(["nav", "views"])).toBe(false);
    expect(tabOpensMainPane(["views"])).toBe(false);
    expect(tabOpensMainPane(["header"])).toBe(false);
    expect(tabOpensMainPane(undefined)).toBe(false);
  });

  it("treats header-without-nav-or-views as a chrome action, not a pane", () => {
    expect(tabIsChromeAction(["header"])).toBe(true);
    expect(tabIsChromeAction(["header", "views"])).toBe(false);
    expect(tabIsChromeAction(["header", "nav"])).toBe(false);
    expect(tabIsChromeAction(["nav"])).toBe(false);
    expect(tabIsChromeAction(["views"])).toBe(false);
    expect(tabIsChromeAction(undefined)).toBe(false);
  });

  it("keeps onClick on a header action tab", () => {
    const onClick = () => {};
    mountSlot("sidebar.tabs", {
      id: "a:open",
      pluginId: "plugin-a",
      mount: () => {},
      chrome: ["header"],
      onClick,
    });
    expect(getSlotEntries("sidebar.tabs").find((e) => e.id === "a:open")?.onClick).toBe(onClick);
  });
});
