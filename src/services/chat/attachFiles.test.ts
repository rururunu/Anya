import { describe, expect, it } from "vitest";
import { extractAttachedFileChips } from "@/services/chat/attachFiles";

describe("extractAttachedFileChips", () => {
  it("reconstructs content chips from inlined attachment tags", () => {
    const content = [
      "please review this",
      "",
      '<peek-attached-file name="notes.txt" path="C:\\proj\\notes.txt">',
      "hello world",
      "</peek-attached-file>",
    ].join("\n");

    const chips = extractAttachedFileChips(content);
    expect(chips).toHaveLength(1);
    expect(chips[0]?.name).toBe("notes.txt");
    expect(chips[0]?.path).toBe("C:\\proj\\notes.txt");
    expect(chips[0]?.content).toContain("hello world");
  });

  it("keeps skipped attachments content-less", () => {
    const content =
      '<peek-attached-file name="photo.png" path="photo.png" skipped="binary skipped" />';
    const chips = extractAttachedFileChips(content);
    expect(chips).toHaveLength(1);
    expect(chips[0]).toMatchObject({
      name: "photo.png",
      content: null,
      skippedReason: "binary skipped",
    });
  });

  it("returns an empty list when there are no attachments", () => {
    expect(extractAttachedFileChips("just a plain message")).toEqual([]);
  });
});
