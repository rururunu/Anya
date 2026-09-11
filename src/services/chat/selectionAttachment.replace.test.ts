import { describe, expect, it } from "vitest";
import { replaceUserVisibleText } from "@/services/chat/selectionAttachment";

describe("replaceUserVisibleText", () => {
  it("replaces the visible prompt and keeps image markdown", () => {
    const original = "draw a cat\n\n![image](path:C:/shot.png)";
    expect(replaceUserVisibleText(original, "draw a dog")).toBe(
      "draw a dog\n\n![image](path:C:/shot.png)",
    );
  });

  it("prepends text onto an image-only message", () => {
    const original = "![image](path:C:/shot.png)";
    expect(replaceUserVisibleText(original, "what is this")).toBe(
      "what is this\n\n![image](path:C:/shot.png)",
    );
  });
});
