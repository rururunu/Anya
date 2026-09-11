import { describe, expect, it } from "vitest";
import { pluginIconUrl } from "@/services/plugins/ipc";
import {
  displayPluginIcon,
  isPluginIconUrl,
  resolveSlotIcon,
} from "@/composables/plugins/pluginIcon";

describe("resolveSlotIcon", () => {
  it("keeps an explicit image URL", () => {
    const url = "anya-plugin://localhost/bid-writer/ui/mark.svg";
    expect(resolveSlotIcon("bid-writer", url, "ui/icon.svg")).toBe(url);
    expect(isPluginIconUrl(url)).toBe(true);
  });

  it("uses the manifest file instead of a Lucide glyph name", () => {
    expect(resolveSlotIcon("cat-ears", "sparkles", "ui/icon.svg")).toBe(
      pluginIconUrl("cat-ears", "ui/icon.svg"),
    );
  });

  it("keeps a Lucide glyph when the plugin has no icon file", () => {
    expect(resolveSlotIcon("terminal", "terminal", undefined)).toBe("terminal");
  });

  it("falls back to ui/icon.svg when nothing is passed", () => {
    expect(resolveSlotIcon("demo", undefined, undefined)).toBe(
      pluginIconUrl("demo", "ui/icon.svg"),
    );
  });
});

describe("displayPluginIcon", () => {
  it("uses the manifest file for the plugins list", () => {
    expect(displayPluginIcon({ id: "terminal", icon: "ui/icon.svg" })).toBe(
      pluginIconUrl("terminal", "ui/icon.svg"),
    );
  });

  it("falls back to a live tab glyph when the manifest has no icon", () => {
    expect(displayPluginIcon({ id: "notes" }, "sticky-note")).toBe("sticky-note");
  });

  it("maps the bundled terminal to the Terminal glyph without a file", () => {
    expect(displayPluginIcon({ id: "terminal" })).toBe("terminal");
  });
});
