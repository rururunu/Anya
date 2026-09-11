import { describe, expect, it } from "vitest";
import {
  getAssetOverride,
  getAssetConflicts,
  registerAsset,
  unregisterPluginAssets,
} from "@/composables/plugins/assetRegistry";

describe("assetRegistry", () => {
  it("registers an allowed kind for a known key", () => {
    expect(registerAsset("mascot.idle", "plugin-a", { kind: "video", source: "x.webm" })).toBe(
      true,
    );
    expect(getAssetOverride("mascot.idle")).toMatchObject({ pluginId: "plugin-a", kind: "video" });
    unregisterPluginAssets("plugin-a");
  });

  it("rejects a second plugin on the same key and records a conflict", () => {
    registerAsset("workbench.backdrop", "plugin-a", { kind: "image", source: "a.png" });
    expect(
      registerAsset("workbench.backdrop", "plugin-b", { kind: "image", source: "b.png" }),
    ).toBe(false);
    expect(getAssetOverride("workbench.backdrop")?.pluginId).toBe("plugin-a");
    expect(getAssetConflicts().at(-1)).toMatchObject({
      key: "workbench.backdrop",
      ownerPluginId: "plugin-a",
      rejectedPluginId: "plugin-b",
    });
    unregisterPluginAssets("plugin-a");
    expect(getAssetConflicts().some((item) => item.ownerPluginId === "plugin-a")).toBe(false);
    unregisterPluginAssets("plugin-b");
  });

  it("rejects a disallowed kind for a constrained key", () => {
    expect(() =>
      registerAsset("tray.icon", "plugin-a", { kind: "video", source: "x.webm" }),
    ).toThrow();
  });

  it("rejects an unknown asset key", () => {
    expect(() =>
      registerAsset("no.such.key", "plugin-a", { kind: "image", source: "x.png" }),
    ).toThrow();
  });
});
