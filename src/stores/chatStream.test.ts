import { describe, expect, it } from "vitest";
import type { ChatMessage } from "@/types/chat";
import { appendTimelineInject, appendTimelineText, withTimelineInject } from "./chatStream";

function assistant(timeline?: ChatMessage["workTimeline"]): ChatMessage {
  return {
    id: "a1",
    sessionId: "s1",
    role: "assistant",
    content: "draft",
    status: "streaming",
    timestamp: 1,
    workTimeline: timeline,
  };
}

describe("appendTimelineInject", () => {
  it("pins the follow-up so later content starts a new segment after it", () => {
    let timeline = appendTimelineText(undefined, "before ", "content");
    timeline = appendTimelineInject(timeline, "inj-1", "use a flowchart");
    timeline = appendTimelineText(timeline, "after", "content");
    expect(timeline.map((item) => item.type)).toEqual(["content", "inject", "content"]);
    expect(timeline[1]).toMatchObject({ type: "inject", content: "use a flowchart" });
    expect(timeline[2]).toMatchObject({ type: "content", content: "after" });
  });

  it("does not duplicate the same inject", () => {
    const once = appendTimelineInject(undefined, "inj-1", "hello");
    expect(appendTimelineInject(once, "inj-1", "hello")).toEqual(once);
  });
});

describe("withTimelineInject", () => {
  it("is a no-op when the assistant timeline already has the inject", () => {
    const message = assistant([{ type: "inject", id: "u2", content: "use a flowchart" }]);
    const inject: ChatMessage = {
      id: "u2",
      sessionId: "s1",
      role: "user",
      content: "<!--peek:soft-inject-->\nuse a flowchart",
      injected: true,
      status: "done",
      timestamp: 2,
    };
    expect(withTimelineInject(message, inject)).toBe(message);
  });
});
