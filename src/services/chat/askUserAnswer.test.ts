import { describe, expect, it } from "vitest";
import type { ToolActivity } from "@/types/chat";
import {
  askUserAnswersForActivity,
  normalizeAskUserAnswerItems,
  parseAskUserAnswerItems,
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
});
