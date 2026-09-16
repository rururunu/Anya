import { describe, expect, it } from "vitest";
import type { ToolActivity } from "@/types/chat";
import {
  askUserAnswersForActivity,
  isAskUserTool,
  isPlanSwitchAsk,
  isPlanSwitchQuestion,
  isPlanSwitchTool,
  normalizeAskUserAnswerItems,
  parseAskUserAnswerItems,
  PLAN_SWITCH_KIND,
} from "./askUserAnswer";

const payload = JSON.stringify({
  answers: [
    {
      header: "Entry",
      question: "插件主页的入口放在哪里?",
      selected: ["点击已安装插件列表"],
    },
    {
      question: "是否完全移除 settings 入口?",
      selected: ["是，完全移除"],
    },
  ],
});

function activity(partial: Partial<ToolActivity>): ToolActivity {
  return {
    id: "a1",
    toolName: "ask_user",
    title: "Ask user",
    kind: "other",
    status: "done",
    success: true,
    ...partial,
  };
}

describe("parseAskUserAnswerItems", () => {
  it("reads question and selected rows from the picker JSON", () => {
    const items = parseAskUserAnswerItems(payload);
    expect(items).toHaveLength(2);
    expect(items[0]).toMatchObject({
      header: "Entry",
      question: "插件主页的入口放在哪里?",
      selected: ["点击已安装插件列表"],
    });
  });

  it("returns empty for invalid JSON", () => {
    expect(parseAskUserAnswerItems("not-json")).toEqual([]);
    expect(parseAskUserAnswerItems("")).toEqual([]);
  });
});

describe("normalizeAskUserAnswerItems", () => {
  it("drops empty rows and keeps supplement-only answers", () => {
    expect(
      normalizeAskUserAnswerItems([
        { question: "Q", selected: ["  "] },
        { question: "Skip", selected: [], userSupplement: true },
      ]),
    ).toEqual([
      {
        header: undefined,
        question: "Skip",
        kind: undefined,
        selected: [],
        userSupplement: true,
      },
    ]);
  });
});

describe("askUserAnswersForActivity", () => {
  it("prefers the persisted tool result", () => {
    const items = askUserAnswersForActivity(activity({ result: payload, status: "done" }), [
      { selected: ["stale"] },
    ]);
    expect(items[0]?.question).toBe("插件主页的入口放在哪里?");
  });

  it("falls back to staged answers when the tool result is not Q&A JSON", () => {
    const staged = [{ question: "Q", selected: ["A"] }];
    expect(askUserAnswersForActivity(activity({ status: "running" }), staged)).toEqual(staged);
    expect(askUserAnswersForActivity(activity({ status: "done" }), staged)).toEqual(staged);
  });

  it("treats request_plan_mode as an ask-user tool", () => {
    expect(isAskUserTool("ask_user")).toBe(true);
    expect(isAskUserTool("request_plan_mode")).toBe(true);
    expect(isAskUserTool("run_shell")).toBe(false);
    const items = askUserAnswersForActivity(
      activity({ toolName: "request_plan_mode", result: payload, status: "done" }),
    );
    expect(items).toHaveLength(2);
  });
});

describe("plan switch detection", () => {
  it("recognizes kind, header fallback, and the request_plan_mode tool", () => {
    expect(isPlanSwitchQuestion({ kind: PLAN_SWITCH_KIND, header: "other" })).toBe(true);
    expect(isPlanSwitchQuestion({ header: "切换到计划" })).toBe(true);
    expect(isPlanSwitchQuestion({ header: "Entry" })).toBe(false);
    expect(isPlanSwitchTool("request_plan_mode")).toBe(true);
    expect(isPlanSwitchTool("ask_user")).toBe(false);
    expect(isPlanSwitchAsk([{ header: "切换到计划", selected: ["继续用 Agent"] }])).toBe(true);
    expect(isPlanSwitchAsk([{ question: "普通提问", selected: ["A"] }])).toBe(false);
  });
});
