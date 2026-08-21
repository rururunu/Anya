import { describe, expect, it, beforeEach } from "vitest";
import {
  compareByResourceUsage,
  loadResourceUsage,
  recordResourceUsage,
  recordResourceUsages,
  resourceFromToolActivity,
  resourceUsageScore,
  saveResourceUsage,
  sortByResourceUsage,
  type ResourceUsageStore,
} from "./resourceUsage";

const STORAGE_KEY = "anya.resourceUsage.v1";

const memory = new Map<string, string>();

beforeEach(() => {
  memory.clear();
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: {
      getItem: (key: string) => memory.get(key) ?? null,
      setItem: (key: string, value: string) => {
        memory.set(key, value);
      },
      removeItem: (key: string) => {
        memory.delete(key);
      },
    },
  });
});

describe("resourceUsage", () => {
  it("records and scores frequency with recency", () => {
    const now = 1_700_000_000_000;
    recordResourceUsage("skill", "docx", now - 86_400_000);
    recordResourceUsage("skill", "docx", now);
    recordResourceUsage("skill", "pandoc", now);

    const store = loadResourceUsage();
    expect(store.skill.docx.count).toBe(2);
    expect(store.skill.pandoc.count).toBe(1);
    expect(resourceUsageScore("skill", "docx", store, now)).toBeGreaterThan(
      resourceUsageScore("skill", "pandoc", store, now),
    );
  });

  it("sorts items by usage then id", () => {
    const store: ResourceUsageStore = {
      skill: {},
      mcp: {
        gmail: { count: 5, lastUsedAt: 100 },
        github: { count: 1, lastUsedAt: 200 },
        filesystem: { count: 5, lastUsedAt: 50 },
      },
    };
    saveResourceUsage(store);
    const sorted = sortByResourceUsage(
      [{ id: "github" }, { id: "filesystem" }, { id: "gmail" }, { id: "unused" }],
      "mcp",
      (item) => item.id,
      store,
      300,
    ).map((item) => item.id);
    expect(sorted[0]).toBe("gmail");
    expect(sorted[1]).toBe("filesystem");
    expect(sorted[2]).toBe("github");
    expect(sorted[3]).toBe("unused");
    expect(compareByResourceUsage("mcp", "gmail", "github", store, 300)).toBeLessThan(0);
  });

  it("parses skill and mcp tool activities", () => {
    expect(resourceFromToolActivity("mcp__gmail__send_email")).toEqual({
      kind: "mcp",
      id: "gmail",
    });
    expect(resourceFromToolActivity("run_skill", { name: "docx" })).toEqual({
      kind: "skill",
      id: "docx",
    });
    expect(resourceFromToolActivity("generate_word")).toEqual({
      kind: "skill",
      id: "generate_word",
    });
    expect(resourceFromToolActivity("review_security")).toEqual({
      kind: "skill",
      id: "security_review",
    });
    expect(resourceFromToolActivity("read_file")).toBeNull();
  });

  it("records mention batches once", () => {
    recordResourceUsages([
      { kind: "skill", id: "docx" },
      { kind: "mcp", id: "gmail" },
      { kind: "skill", id: "docx" },
    ]);
    const store = loadResourceUsage();
    expect(store.skill.docx.count).toBe(2);
    expect(store.mcp.gmail.count).toBe(1);
    expect(memory.has(STORAGE_KEY)).toBe(true);
  });
});
