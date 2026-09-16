import { describe, expect, it } from "vitest";
import type { ChatMessage } from "@/types/chat";
import { lastTurnUserContent } from "./chatHistory";

function msg(
  partial: Partial<ChatMessage> & Pick<ChatMessage, "id" | "role" | "content">,
): ChatMessage {
  return {
    sessionId: "s1",
    status: "done",
    timestamp: 1,
    ...partial,
  };
}

describe("lastTurnUserContent", () => {
  it("returns the latest non-injected user text", () => {
    expect(
      lastTurnUserContent([
        msg({ id: "u1", role: "user", content: "first" }),
        msg({ id: "a1", role: "assistant", content: "ok" }),
        msg({ id: "u2", role: "user", content: "second" }),
      ]),
    ).toBe("second");
  });

  it("skips soft-injected follow-ups so a duplicate queue item can be detected", () => {
    expect(
      lastTurnUserContent([
        msg({ id: "u1", role: "user", content: "hello" }),
        msg({ id: "a1", role: "assistant", content: "", status: "streaming" }),
        msg({
          id: "u2",
          role: "user",
          content: "<!--peek:soft-inject-->\nhello",
          injected: true,
        }),
      ]),
    ).toBe("hello");
  });

  it("returns empty when there is no user turn", () => {
    expect(lastTurnUserContent([msg({ id: "a1", role: "assistant", content: "hi" })])).toBe("");
  });
});
