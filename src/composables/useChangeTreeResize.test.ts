import { beforeEach, describe, expect, it } from "vitest";

import {
  CHANGE_TREE_DEFAULT_WIDTH,
  CHANGE_TREE_MAX_WIDTH,
  CHANGE_TREE_MIN_WIDTH,
  clampChangeTreeWidth,
  readStoredChangeTreeWidth,
} from "./useChangeTreeResize";

const STORAGE_KEY = "anya.diffChangeTreeWidth.v1";
const memory = new Map<string, string>();

beforeEach(() => {
  memory.clear();
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: {
      getItem: (key: string) => memory.get(key) ?? null,
      setItem: (key: string, value: string) => memory.set(key, value),
    },
  });
});

describe("changed-files tree width", () => {
  it("keeps the previous fixed width as the default", () => {
    expect(CHANGE_TREE_DEFAULT_WIDTH).toBe(196);
    expect(readStoredChangeTreeWidth()).toBe(CHANGE_TREE_DEFAULT_WIDTH);
  });

  it("clamps to the minimum and the absolute maximum", () => {
    expect(clampChangeTreeWidth(20, 1200)).toBe(CHANGE_TREE_MIN_WIDTH);
    expect(clampChangeTreeWidth(4000, 4000)).toBe(CHANGE_TREE_MAX_WIDTH);
  });

  it("never eats the room the diff pane needs", () => {
    // 700 - 240 leaves 460 for the tree.
    expect(clampChangeTreeWidth(600, 700)).toBe(460);
    // A narrow pane keeps the tree usable instead of collapsing it.
    expect(clampChangeTreeWidth(400, 360)).toBe(CHANGE_TREE_MIN_WIDTH);
  });

  it("rounds the requested width", () => {
    expect(clampChangeTreeWidth(233.6, 1200)).toBe(234);
  });

  it("restores a stored width but never below the minimum", () => {
    memory.set(STORAGE_KEY, "310");
    expect(readStoredChangeTreeWidth()).toBe(310);

    memory.set(STORAGE_KEY, "12");
    expect(readStoredChangeTreeWidth()).toBe(CHANGE_TREE_MIN_WIDTH);

    memory.set(STORAGE_KEY, "not a number");
    expect(readStoredChangeTreeWidth()).toBe(CHANGE_TREE_DEFAULT_WIDTH);
  });
});
