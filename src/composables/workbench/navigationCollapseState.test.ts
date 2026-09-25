import { beforeEach, describe, expect, it } from "vitest";

import {
  persistExpandedNavigationState,
  readExpandedNavigationState,
} from "./navigationCollapseState";

const STORAGE_KEY = "anya.workbenchNavigationExpanded.v1";

const memory = new Map<string, string>();
let failWrites = false;

beforeEach(() => {
  memory.clear();
  failWrites = false;
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: {
      getItem: (key: string) => memory.get(key) ?? null,
      setItem: (key: string, value: string) => {
        if (failWrites) throw new Error("storage is full");
        memory.set(key, value);
      },
      removeItem: (key: string) => {
        memory.delete(key);
      },
      clear: () => memory.clear(),
    },
  });
});

describe("navigation collapse persistence", () => {
  it("treats a fresh install as fully collapsed", () => {
    const state = readExpandedNavigationState();
    expect([...state.sections]).toEqual([]);
    expect([...state.workspaces]).toEqual([]);
  });

  it("round-trips the expanded ids", () => {
    persistExpandedNavigationState({
      sections: new Set(["quick", "pinned"]),
      workspaces: new Set(["w1", "w2"]),
    });

    const state = readExpandedNavigationState();
    expect([...state.sections].sort()).toEqual(["pinned", "quick"]);
    expect([...state.workspaces].sort()).toEqual(["w1", "w2"]);
  });

  it("collapses everything again once the stored entry is dropped", () => {
    persistExpandedNavigationState({ sections: new Set(["quick"]), workspaces: new Set(["w1"]) });
    memory.delete(STORAGE_KEY);

    const state = readExpandedNavigationState();
    expect([...state.sections]).toEqual([]);
    expect([...state.workspaces]).toEqual([]);
  });

  it("falls back to fully collapsed on corrupted or partial storage", () => {
    memory.set(STORAGE_KEY, "{not json");
    expect([...readExpandedNavigationState().workspaces]).toEqual([]);

    memory.set(STORAGE_KEY, JSON.stringify({ sections: "nope", workspaces: [7, null, "ok"] }));
    const state = readExpandedNavigationState();
    expect([...state.sections]).toEqual([]);
    expect([...state.workspaces]).toEqual(["ok"]);
  });

  it("survives a storage that refuses writes", () => {
    failWrites = true;
    expect(() =>
      persistExpandedNavigationState({ sections: new Set(["quick"]), workspaces: new Set() }),
    ).not.toThrow();
    expect(memory.has(STORAGE_KEY)).toBe(false);
  });
});
