import { describe, expect, it } from "vitest";

import {
  generatedImagePrompt,
  isLocalImagePath,
  parseGeneratedImageSources,
} from "./localImageSrc";

describe("parseGeneratedImageSources", () => {
  it("reads path: markdown from generate_image results", () => {
    const result = [
      "Generated 1 image with gpt-image-2 (1024x1024, auto).",
      "Saved: C:\\\\Users\\\\a\\\\generated\\\\cat.png",
      "![a cat](path:C:\\\\Users\\\\a\\\\generated\\\\cat.png)",
    ].join("\n");
    expect(parseGeneratedImageSources(result)).toEqual([
      "path:C:\\\\Users\\\\a\\\\generated\\\\cat.png",
    ]);
  });

  it("falls back to Saved: lines", () => {
    expect(parseGeneratedImageSources("Saved: D:/tmp/out.png")).toEqual(["path:D:/tmp/out.png"]);
  });

  it("ignores remote markdown images", () => {
    expect(parseGeneratedImageSources("![x](https://example.com/a.png)")).toEqual([]);
  });

  it("prefers the anya-images JSON fence when present", () => {
    const result = [
      "Generated 1 image with gpt-image-2 (1024x1024, high).",
      "Saved: C:\\\\legacy\\\\ignored.png",
      "![legacy](path:C:\\\\legacy\\\\ignored.png)",
      "```anya-images",
      JSON.stringify({
        version: 1,
        images: [
          { path: "C:\\\\Users\\\\a\\\\generated\\\\cat.png", revised_prompt: "orange cat" },
        ],
      }),
      "```",
    ].join("\n");
    expect(parseGeneratedImageSources(result)).toEqual([
      "path:C:\\\\Users\\\\a\\\\generated\\\\cat.png",
    ]);
  });
});

describe("generatedImagePrompt", () => {
  it("prefers a revised prompt from the tool result", () => {
    const result = [
      "Generated 1 image with gpt-image-2 (1024x1024, auto).",
      "Saved: C:\\\\a\\\\cat.png",
      "![a cat](path:C:\\\\a\\\\cat.png)",
      "Revised prompt: a clearer orange cat in sunlight",
    ].join("\n");
    expect(generatedImagePrompt(result, { prompt: "a cat\nStyle: anime illustration" })).toBe(
      "a clearer orange cat in sunlight",
    );
  });

  it("reads revised prompts from the anya-images fence", () => {
    const path = "path:C:\\\\a\\\\cat.png";
    const result = [
      `![a cat](${path})`,
      "```anya-images",
      JSON.stringify({
        version: 1,
        images: [{ path: "C:\\\\a\\\\cat.png", revised_prompt: "structured revision" }],
      }),
      "```",
    ].join("\n");
    expect(generatedImagePrompt(result, { prompt: "a cat" }, path)).toBe("structured revision");
  });

  it("falls back to the tool argument prompt", () => {
    expect(generatedImagePrompt("Generated 1 image.", { prompt: "  a cat  " })).toBe("a cat");
  });

  it("picks the matching revised prompt when several images were generated", () => {
    const first = "path:C:\\\\a\\\\one.png";
    const second = "path:C:\\\\a\\\\two.png";
    const result = [
      `![one](${first})`,
      "Revised prompt: first take",
      `![two](${second})`,
      "Revised prompt: second take",
    ].join("\n");
    expect(generatedImagePrompt(result, { prompt: "two cats" }, second)).toBe("second take");
  });
});

describe("isLocalImagePath", () => {
  it("accepts windows, unc, unix, and path: wrappers", () => {
    expect(isLocalImagePath("path:C:\\\\a.png")).toBe(true);
    expect(isLocalImagePath("C:\\\\a.png")).toBe(true);
    expect(isLocalImagePath("/tmp/a.png")).toBe(true);
    expect(isLocalImagePath("https://example.com/a.png")).toBe(false);
    expect(isLocalImagePath("data:image/png;base64,abc")).toBe(false);
  });
});
