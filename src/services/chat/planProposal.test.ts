import { describe, expect, it } from "vitest";
import type { ChatMessage, ToolActivity } from "@/types/chat";
import {
  extractPlanSummary,
  extractPlanTitle,
  isCreatedPlanMessage,
  isPlanExecutePrompt,
  planApprovePrompt,
  planCardCopy,
  planFromHistory,
  planPathFromMessage,
  savePlanFromMessage,
  shouldHidePlanChromeActivity,
  tasksFromHistory,
} from "./planProposal";

function message(activities: ToolActivity[]): ChatMessage {
  return {
    id: "m1",
    sessionId: "s1",
    role: "assistant",
    content: "",
    status: "done",
    timestamp: 1,
    toolActivities: activities,
  };
}

function activity(partial: Partial<ToolActivity> & Pick<ToolActivity, "toolName">): ToolActivity {
  return {
    id: partial.id ?? "a1",
    title: partial.title ?? partial.toolName,
    kind: partial.kind ?? "other",
    status: partial.status ?? "done",
    success: partial.success ?? true,
    arguments: partial.arguments,
    ...partial,
  };
}

const planMarkdown = `# Plugin home page

用固定框架的插件主页取代 settings chrome surface。

## 改动

- 新建 PluginHomeView.vue
`;

describe("extractPlanTitle / extractPlanSummary", () => {
  it("reads the first heading and the first prose paragraph", () => {
    expect(extractPlanTitle(planMarkdown)).toBe("Plugin home page");
    expect(extractPlanSummary(planMarkdown)).toContain("插件主页");
  });
});

describe("savePlanFromMessage / planCardCopy", () => {
  it("prefers save_plan title and summary", () => {
    const msg = message([
      activity({
        toolName: "save_plan",
        arguments: { title: "Plugin home page", content: planMarkdown },
      }),
    ]);
    expect(savePlanFromMessage(msg)?.title).toBe("Plugin home page");
    expect(planCardCopy(msg, "Plan").title).toBe("Plugin home page");
    expect(planCardCopy(msg, "Plan").summary).toContain("插件主页");
  });
});

describe("tasksFromHistory / planFromHistory", () => {
  it("rewinds to the last remaining checklist and save_plan", () => {
    const planned = message([
      activity({
        toolName: "save_plan",
        arguments: { title: "Plugin home page", content: planMarkdown },
      }),
      activity({
        toolName: "update_tasks",
        arguments: { tasks: [{ content: "probe host API", status: "pending" }] },
      }),
    ]);
    const executed = {
      ...message([
        activity({
          id: "later",
          toolName: "update_tasks",
          arguments: { tasks: [{ content: "probe host API", status: "completed" }] },
        }),
      ]),
      id: "m2",
    };
    expect(tasksFromHistory([planned, executed])[0]?.status).toBe("completed");
    expect(tasksFromHistory([planned])[0]?.status).toBe("pending");
    expect(planFromHistory([planned, executed])?.title).toBe("Plugin home page");
    expect(tasksFromHistory([])).toEqual([]);
    expect(planFromHistory([])).toBeNull();
  });
});

describe("isCreatedPlanMessage", () => {
  it("does not treat a plan-gate stop as a created plan", () => {
    const msg = {
      ...message([]),
      content:
        "已停止：计划尚未批准，Shell 和写文件已暂停。请点「批准并执行」，或批准后再发消息继续。",
    };
    expect(isCreatedPlanMessage(msg)).toBe(false);
    expect(msg.content.includes("计划尚未批准")).toBe(true);
  });
});

describe("isPlanExecutePrompt", () => {
  it("matches the canned approve-and-execute prompt", () => {
    expect(
      isPlanExecutePrompt(
        "计划已批准。现在严格按临时方案（.anya/plans/20260921-101500-fix-scroll.md）执行，本回合写操作已解除限制。",
      ),
    ).toBe(true);
    expect(
      isPlanExecutePrompt("Plan approved. Execute strictly according to the plan proposal."),
    ).toBe(true);
    expect(isPlanExecutePrompt("视频要能让用户选择本地文件")).toBe(false);
  });
});

describe("planPathFromMessage / planApprovePrompt", () => {
  it("prefers the explicit save_plan path", () => {
    const msg = message([
      activity({
        toolName: "save_plan",
        arguments: { title: "fix-scroll", path: "plans/fix-scroll.md", content: planMarkdown },
      }),
    ]);
    expect(planPathFromMessage(msg)).toBe("plans/fix-scroll.md");
  });

  it("recovers the host-resolved default path from the tool result", () => {
    const msg = message([
      activity({
        toolName: "save_plan",
        arguments: { title: "fix-scroll", content: planMarkdown },
        result: "Saved plan proposal to `.anya/plans/20260921-101500-fix-scroll.md` (120 bytes).",
      }),
    ]);
    expect(planPathFromMessage(msg)).toBe(".anya/plans/20260921-101500-fix-scroll.md");
  });

  it("carries the host-resolved path into the plan restored from history", () => {
    const msg = message([
      activity({
        toolName: "save_plan",
        arguments: { title: "fix-scroll", content: planMarkdown },
        result: "Saved plan proposal to `.anya/plans/20260921-101500-fix-scroll.md` (120 bytes).",
      }),
    ]);
    expect(savePlanFromMessage(msg)?.path).toBe(".anya/plans/20260921-101500-fix-scroll.md");
    expect(planFromHistory([msg])?.path).toBe(".anya/plans/20260921-101500-fix-scroll.md");
  });

  it("names the actual plan file in the approve prompt", () => {
    const prompt = planApprovePrompt("zh-CN", ".anya/plans/20260921-101500-fix-scroll.md");
    expect(prompt).toContain(".anya/plans/20260921-101500-fix-scroll.md");
    expect(prompt).not.toContain(".anya/plan.md）");
    expect(isPlanExecutePrompt(prompt)).toBe(true);
  });

  it("falls back to the default plan path when none was reported", () => {
    expect(planApprovePrompt("en-US")).toContain("(.anya/plan.md)");
  });
});

describe("shouldHidePlanChromeActivity", () => {
  it("hides save_plan and the matching task list", () => {
    const msg = message([
      activity({
        id: "p",
        toolName: "save_plan",
        arguments: { content: planMarkdown },
      }),
      activity({
        id: "t",
        toolName: "update_tasks",
        arguments: { tasks: [{ content: "新建 PluginHomeView.vue", status: "pending" }] },
      }),
    ]);
    expect(shouldHidePlanChromeActivity(msg.toolActivities![0]!, msg)).toBe(true);
    expect(shouldHidePlanChromeActivity(msg.toolActivities![1]!, msg)).toBe(true);
    expect(isCreatedPlanMessage(msg)).toBe(true);
  });

  it("does not treat an executing turn as a new Created Plan", () => {
    const msg = message([
      activity({
        id: "p",
        toolName: "save_plan",
        arguments: { content: planMarkdown },
      }),
      activity({
        id: "t",
        toolName: "update_tasks",
        arguments: {
          tasks: [
            { content: "probe host API", status: "completed" },
            { content: "write plugin.json", status: "in_progress" },
          ],
        },
      }),
      activity({
        id: "w",
        toolName: "write_file",
        kind: "create",
        arguments: { path: "plugin.json" },
      }),
    ]);
    expect(isCreatedPlanMessage(msg)).toBe(false);
  });
});
