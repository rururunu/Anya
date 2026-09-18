import { describe, expect, it } from "vitest";
import {
  codeChangesFromUnifiedDiff,
  extractCodeChanges,
  lastRoundMessages,
} from "@/services/chat/codeChanges";
import type { ChatMessage, ToolActivity } from "@/types/chat";

function message(partial: Partial<ChatMessage> & Pick<ChatMessage, "id" | "role">): ChatMessage {
  return {
    sessionId: "s",
    content: "",
    status: "done",
    timestamp: 0,
    ...partial,
  };
}

function edit(id: string, path: string): ToolActivity {
  return {
    id,
    toolName: "write_file",
    title: path,
    kind: "edit",
    status: "done",
    success: true,
    arguments: { path, content: "next" },
    preview: {
      path,
      kind: "edit",
      oldText: "prev",
      newText: "next",
      unifiedDiff: `--- a/${path}\n+++ b/${path}\n@@ -1 +1 @@\n-prev\n+next`,
    },
  };
}

describe("lastRoundMessages", () => {
  it("returns messages from the latest user turn", () => {
    const messages = [
      message({ id: "u1", role: "user" }),
      message({ id: "a1", role: "assistant" }),
      message({ id: "u2", role: "user" }),
      message({ id: "a2", role: "assistant" }),
    ];
    expect(lastRoundMessages(messages).map((item) => item.id)).toEqual(["u2", "a2"]);
  });

  it("keeps the whole transcript when there is no user turn", () => {
    const messages = [message({ id: "a1", role: "assistant" })];
    expect(lastRoundMessages(messages)).toEqual(messages);
  });
});

describe("extractCodeChanges last round", () => {
  it("ignores edits from earlier turns", () => {
    const messages = [
      message({ id: "u1", role: "user" }),
      message({ id: "a1", role: "assistant", toolActivities: [edit("t1", "old.ts")] }),
      message({ id: "u2", role: "user" }),
      message({ id: "a2", role: "assistant", toolActivities: [edit("t2", "new.ts")] }),
    ];
    expect(extractCodeChanges(lastRoundMessages(messages)).map((item) => item.path)).toEqual([
      "new.ts",
    ]);
  });
});

describe("codeChangesFromUnifiedDiff", () => {
  it("splits git hunks and keeps deleted-file paths", () => {
    const diff = [
      "--- a/keep.ts",
      "+++ b/keep.ts",
      "@@ -1 +1 @@",
      "-old",
      "+new",
      "--- a/gone.ts",
      "+++ /dev/null",
      "@@ -1 +0,0 @@",
      "-bye",
    ].join("\n");
    expect(codeChangesFromUnifiedDiff(diff).map((item) => item.path)).toEqual([
      "keep.ts",
      "gone.ts",
    ]);
  });
});
