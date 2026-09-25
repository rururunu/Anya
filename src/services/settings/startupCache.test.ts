/** @vitest-environment jsdom */
import { beforeEach, describe, expect, it } from "vitest";
import { readCachedComposerSettings, writeCachedComposerSettings } from "./startupCache";
import type { ReasoningEffort } from "@/types/setting";

const CACHE_KEY = "aaa.composerSettings.v1";

describe("composer settings boot cache", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("round-trips the composer-relevant settings", () => {
    writeCachedComposerSettings({
      reasoningEffort: "high",
      chatModel: "deepseek/deepseek-v4.1-flash",
      chatModelProvider: "command-code",
    });

    expect(readCachedComposerSettings()).toEqual({
      reasoningEffort: "high",
      chatModel: "deepseek/deepseek-v4.1-flash",
      chatModelProvider: "command-code",
    });
  });

  it("returns null when nothing was cached", () => {
    expect(readCachedComposerSettings()).toBeNull();
  });

  it("returns null for a corrupted payload", () => {
    localStorage.setItem(CACHE_KEY, "not json at all");

    expect(readCachedComposerSettings()).toBeNull();
  });

  it("returns null when no model was stored", () => {
    localStorage.setItem(
      CACHE_KEY,
      JSON.stringify({ version: 1, reasoningEffort: "high", chatModel: "", chatModelProvider: "" }),
    );

    expect(readCachedComposerSettings()).toBeNull();
  });

  it("falls back to 关闭 for an unknown thinking strength", () => {
    writeCachedComposerSettings({
      reasoningEffort: "ultra" as ReasoningEffort,
      chatModel: "gpt-5",
      chatModelProvider: "openai",
    });

    expect(readCachedComposerSettings()?.reasoningEffort).toBe("disabled");
  });
});
