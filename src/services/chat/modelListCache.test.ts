/** @vitest-environment jsdom */
import { beforeEach, describe, expect, it } from "vitest";
import { readCachedModelList, writeCachedModelList } from "./modelListCache";
import type { ChatModelInfo } from "@/types/chat";

const CACHE_KEY = "aaa.chatModels.v1";

const models: ChatModelInfo[] = [
  {
    id: "deepseek/deepseek-v4.1-flash",
    ownedBy: "command-code",
    provider: "command-code",
    displayName: "v4-flash",
  },
  { id: "moonshotai/Kimi-K3", ownedBy: "command-code", provider: "command-code" },
];

describe("model list boot cache", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("round-trips a host model list", () => {
    writeCachedModelList(models);

    expect(readCachedModelList()).toEqual(models);
  });

  it("starts empty when nothing was cached", () => {
    expect(readCachedModelList()).toEqual([]);
  });

  it("ignores a corrupted payload", () => {
    localStorage.setItem(CACHE_KEY, "{not json");

    expect(readCachedModelList()).toEqual([]);
  });

  it("ignores an unknown cache version", () => {
    localStorage.setItem(CACHE_KEY, JSON.stringify({ version: 99, models }));

    expect(readCachedModelList()).toEqual([]);
  });

  it("drops entries the picker cannot render", () => {
    localStorage.setItem(
      CACHE_KEY,
      JSON.stringify({
        version: 1,
        savedAt: Date.now(),
        models: [
          ...models,
          { id: "", provider: "command-code" },
          { id: "no-provider", ownedBy: "x" },
          "not-an-object",
        ],
      }),
    );

    expect(readCachedModelList()).toEqual(models);
  });
});
