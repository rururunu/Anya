import { expect, it } from "vitest";
import { readCodePreview } from "./readCodePreview";
import type { ToolActivity } from "@/types/chat";
const activity = (toolName: string, result: string): ToolActivity => ({
  id: "read-1",
  toolName,
  title: "read",
  kind: "tool",
  status: "done",
  success: true,
  arguments: { path: "src/sample.yml" },
  result,
});
it("retains actual DeepSeek read line numbers and code indentation", () => {
  const preview = readCodePreview(
    activity(
      "read",
      "<path>C:/code/sample.yml</path>\n<type>file</type>\n<content>\n368: chatZkConfig:\n369:   serverSecret: secret://value\n370: \n\n(End of file - total 370 lines)\n</content>",
    ),
  );
  expect(preview?.path).toBe("C:/code/sample.yml");
  expect(preview?.startLine).toBe(368);
  expect(preview?.endLine).toBe(370);
  expect(preview?.content).toBe("chatZkConfig:\n  serverSecret: secret://value\n");
});
it("supports native reads and ignores metadata/footer rows", () => {
  const preview = readCodePreview(
    activity(
      "read_file",
      "[file] src/sample.yml size=42B\n    12|const text = '<script>';\n    13|  return text;\n… more lines follow; pass offset=14 to continue",
    ),
  );
  expect(preview?.content).toBe("const text = '<script>';\n  return text;");
  expect(preview?.startLine).toBe(12);
  expect(preview?.endLine).toBe(13);
});
it("does not turn failed reads, searches or image reads into code previews", () => {
  const source = activity("read", "1: code");
  expect(readCodePreview({ ...source, success: false })).toBeNull();
  expect(readCodePreview({ ...source, toolName: "grep" })).toBeNull();
  expect(readCodePreview({ ...source, arguments: { file_path: "image.png" } })).toBeNull();
});
