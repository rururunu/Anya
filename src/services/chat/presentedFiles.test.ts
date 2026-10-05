import { describe, expect, it } from "vitest";
import { presentedFiles } from "./presentedFiles";
import type { ChatMessage, ToolActivity } from "@/types/chat";

const file = {
  path: "report.pptx",
  absolutePath: "C:\\work\\report.pptx",
  name: "report.pptx",
  size: 120,
  description: "Slides",
};
function delivery(partial: Partial<ToolActivity> = {}): ToolActivity {
  return {
    id: "p1",
    toolName: "present",
    kind: "tool",
    title: "Deliver",
    status: "done",
    success: true,
    result: JSON.stringify({ version: 1, files: [file] }),
    ...partial,
  };
}
function message(activities: ToolActivity[]): ChatMessage {
  return {
    id: "a",
    sessionId: "s",
    role: "assistant",
    status: "done",
    timestamp: 1,
    content: "",
    toolActivities: activities,
  };
}
describe("presentedFiles", () => {
  it("reconstructs cards from restored history without inspecting final prose", () => {
    const restored = JSON.parse(JSON.stringify(message([delivery()])));
    expect(presentedFiles(restored)).toEqual([file]);
  });
  it("rejects failed, unfinished, malformed and child declarations", () => {
    expect(
      presentedFiles(
        message([
          delivery({ success: false }),
          delivery({ status: "running" }),
          delivery({ result: "broken" }),
          delivery({ subagentId: "child" }),
          delivery({ result: undefined, arguments: { files: [file] } }),
          delivery({
            result: JSON.stringify({
              version: 1,
              files: [{ ...file, absolutePath: "https://example.com" }],
            }),
          }),
        ]),
      ),
    ).toEqual([]);
  });
  it("deduplicates Windows paths, retaining the latest description", () => {
    const latest = { ...file, absolutePath: "c:/work/report.pptx", description: "Final slides" };
    expect(
      presentedFiles(
        message([
          delivery(),
          delivery({ result: JSON.stringify({ version: 1, files: [latest] }) }),
        ]),
      ),
    ).toEqual([latest]);
  });
});
