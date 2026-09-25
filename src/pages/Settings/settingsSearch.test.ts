import { describe, expect, it } from "vitest";
import { matchesStandaloneSettingsCategory } from "./settingsSearch";

describe("standalone settings search", () => {
  it("finds localized provider fields", () => {
    expect(matchesStandaloneSettingsCategory("provider", "zh-CN", "官网 URL")).toBe(true);
    expect(matchesStandaloneSettingsCategory("provider", "en-US", "Official website")).toBe(true);
    expect(matchesStandaloneSettingsCategory("image", "zh-CN", "官网 URL")).toBe(false);
  });

  it("indexes independent settings pages", () => {
    expect(matchesStandaloneSettingsCategory("workspace", "zh-CN", "文件夹")).toBe(true);
    expect(matchesStandaloneSettingsCategory("usage", "en-US", "Token usage")).toBe(true);
    expect(matchesStandaloneSettingsCategory("archive", "zh-CN", "恢复选中")).toBe(true);
  });
});
