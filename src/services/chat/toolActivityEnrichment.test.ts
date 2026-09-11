import { describe, expect, it } from "vitest";

import { isImageGenActivity } from "@/services/chat/toolActivityEnrichment";

describe("isImageGenActivity", () => {
  it("keeps generate_image as an image card", () => {
    expect(isImageGenActivity({ kind: "image", toolName: "generate_image" })).toBe(true);
  });

  it("does not treat computer-use screenshots as generated images", () => {
    expect(isImageGenActivity({ kind: "image", toolName: "plugin_computer-use__screenshot" })).toBe(
      false,
    );
    expect(isImageGenActivity({ kind: "other", toolName: "plugin_computer-use__screenshot" })).toBe(
      false,
    );
  });
});
