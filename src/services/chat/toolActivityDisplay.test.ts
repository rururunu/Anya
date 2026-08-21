import { describe, expect, it } from "vitest";

import {
  hasExpandableActivityContent,
  isProcessSegmentCollapsible,
  summarizeProcessActivities,
} from "@/services/chat/toolActivityDisplay";
import type { ToolActivity } from "@/types/chat";

function activity(partial: Partial<ToolActivity>): ToolActivity {
  return {
    id: partial.id ?? "a1",
    toolName: partial.toolName ?? "read_file",
    title: partial.title ?? "Read file",
    kind: partial.kind ?? "read",
    success: partial.success ?? true,
    status: partial.status ?? "done",
    ...partial,
  };
}

describe("toolActivityDisplay", () => {
  it("treats read/search lines as non-expandable", () => {
    const read = activity({ title: "Read desktop.json L1-14" });
    expect(hasExpandableActivityContent(read)).toBe(false);
    expect(isProcessSegmentCollapsible([read], [read])).toBe(false);
  });

  it("summarizes multi-step process work", () => {
    const items = [
      activity({ id: "1", title: "Read a.rs" }),
      activity({ id: "2", title: "Read b.rs" }),
      activity({ id: "3", toolName: "search_files", title: "Search foo" }),
    ];
    expect(summarizeProcessActivities(items, "en-US")).toBe("Explored 2 files, 1 searches");
    expect(summarizeProcessActivities(items, "zh-CN")).toBe("浏览 2 个文件，1 次搜索");
  });

  it("keeps single-line titles as-is", () => {
    const item = activity({ title: "Read tauri.conf.json L1-73" });
    expect(summarizeProcessActivities([item], "en-US")).toBe("Read tauri.conf.json L1-73");
  });

  it("falls back from empty title to tool name or process summary", () => {
    expect(
      summarizeProcessActivities([activity({ title: "  ", toolName: "browser_read" })], "en-US"),
    ).toBe("browser_read");
    expect(summarizeProcessActivities([activity({ title: "  ", toolName: "  " })], "zh-CN")).toBe(
      "过程详情",
    );
  });
});
