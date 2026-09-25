import { describe, expect, it } from "vitest";
import {
  faviconUrlsForWebsite,
  parseProviderModels,
  serializeProviderModels,
  syncRemoteModels,
} from "./providerPresets";

describe("providerPresets", () => {
  it("resolves favicon candidates from the official website, not an API path", () => {
    expect(faviconUrlsForWebsite("https://commandcode.ai/")).toEqual([
      "https://www.google.com/s2/favicons?sz=64&domain=commandcode.ai",
      "https://commandcode.ai/favicon.ico",
    ]);
    expect(faviconUrlsForWebsite("https://api.commandcode.ai/provider/v1/")[0]).toContain(
      "domain=api.commandcode.ai",
    );
    expect(faviconUrlsForWebsite("invalid-url")).toEqual([]);
  });
  it("parses comma and newline separated models and skips duplicates and empty parts", () => {
    const parsed = parseProviderModels(
      "deepseek-chat, deepseek-reasoner\n\ncustom-1，custom-2,deepseek-chat",
    );
    expect(parsed).toEqual(["deepseek-chat", "deepseek-reasoner", "custom-1", "custom-2"]);
  });

  it("serializes model array with newlines", () => {
    expect(serializeProviderModels(["a", "b"])).toBe("a\nb");
  });

  describe("syncRemoteModels", () => {
    it("preserves user-added models even if they do not exist on remote", () => {
      const current = [
        { id: "deepseek-chat", disabled: false, custom: false },
        { id: "my-finetune-model", disabled: false, custom: true },
      ];
      const remote = ["deepseek-chat", "deepseek-reasoner"];

      const result = syncRemoteModels(current, remote);
      expect(result.map((entry) => entry.id)).toEqual([
        "deepseek-chat",
        "my-finetune-model",
        "deepseek-reasoner",
      ]);
    });

    it("removes non-custom models that disappeared from remote", () => {
      const current = [
        { id: "deprecated-model", disabled: true, custom: false },
        { id: "deepseek-chat", disabled: false, custom: false },
        { id: "user-custom", disabled: false, custom: true },
      ];
      const remote = ["deepseek-chat", "deepseek-reasoner"];

      const result = syncRemoteModels(current, remote);
      expect(result.map((entry) => entry.id)).toEqual([
        "deepseek-chat",
        "user-custom",
        "deepseek-reasoner",
      ]);
    });

    it("keeps existing disabled states for retained models and enables newly fetched remote models", () => {
      const current = [
        { id: "deepseek-chat", disabled: false, custom: false },
        { id: "deepseek-reasoner", disabled: true, custom: false },
      ];
      const remote = ["deepseek-chat", "deepseek-reasoner", "deepseek-v3"];

      const result = syncRemoteModels(current, remote);
      expect(result).toEqual([
        { id: "deepseek-chat", disabled: false, custom: false },
        { id: "deepseek-reasoner", disabled: true, custom: false },
        { id: "deepseek-v3", disabled: false, custom: false },
      ]);
    });
  });
});
