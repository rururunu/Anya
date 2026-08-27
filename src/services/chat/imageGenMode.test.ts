import { describe, expect, it } from "vitest";
import {
  applyImageGenField,
  applyImageGenRatio,
  applyImageGenResolution,
  applyImageGenWidth,
  decodeImageModelSelection,
  defaultImageGenCompose,
  dimensionsFor,
  encodeImageModelSelection,
  imageGenPayload,
  listImageModelChoices,
  normalizeImageGenCompose,
  normalizeImageStyleTemplates,
  selectedImageModelChoiceId,
} from "./imageGenMode";

describe("imageGenMode", () => {
  it("maps ratio and resolution onto pixel size", () => {
    expect(dimensionsFor("1:1", "2k")).toEqual({ width: 2048, height: 2048 });
    expect(dimensionsFor("16:9", "2k")).toEqual({ width: 2048, height: 1152 });
    expect(dimensionsFor("9:16", "2k")).toEqual({ width: 1152, height: 2048 });
    expect(dimensionsFor("auto", "2k")).toEqual({ width: 2048, height: 2048 });
  });

  it("sanitizes unknown compose values", () => {
    expect(
      normalizeImageGenCompose({
        ratio: "21:9",
        quality: "ultra",
        styleId: "unknown",
        count: 99,
      }),
    ).toMatchObject({
      ratio: "21:9",
      resolution: "2k",
      styleId: "unknown",
      count: 1,
    });
  });

  it("falls back to none when styleId is empty", () => {
    expect(normalizeImageGenCompose({ styleId: "  " }).styleId).toBe("none");
  });

  it("migrates the old quality field onto resolution", () => {
    expect(normalizeImageGenCompose({ quality: "low" }).resolution).toBe("1.5k");
    expect(normalizeImageGenCompose({ quality: "high" }).resolution).toBe("4k");
  });

  it("builds a send payload from pixel size and style", () => {
    const payload = imageGenPayload(
      applyImageGenField(
        applyImageGenRatio({ ...defaultImageGenCompose(), count: 2, styleId: "anime" }, "3:4"),
        "count",
        "2",
      ),
    );
    expect(payload.n).toBe(2);
    expect(payload.size).toBe("1536x2048");
    expect(payload.stylePrompt).toContain("anime");
  });

  it("keeps aspect lock when editing width", () => {
    const locked = applyImageGenRatio(defaultImageGenCompose(), "16:9");
    const next = applyImageGenWidth(locked, 1600);
    expect(next.width).toBe(1600);
    expect(next.height).toBe(896);
    expect(next.width % 16).toBe(0);
    expect(next.height % 16).toBe(0);
  });

  it("rescales both edges when resolution changes", () => {
    const next = applyImageGenResolution(applyImageGenRatio(defaultImageGenCompose(), "1:1"), "4k");
    expect(next).toMatchObject({ width: 3840, height: 3840, resolution: "4k" });
  });

  it("includes custom template prompt and example image in the payload", () => {
    const templates = normalizeImageStyleTemplates([
      {
        id: "custom-1",
        name: "Film",
        prompt: "35mm grain",
        exampleImage: "data:image/png;base64,abc",
      },
    ]);
    expect(normalizeImageGenCompose({ styleId: "custom-1" }).styleId).toBe("custom-1");
    const payload = imageGenPayload(
      { ...defaultImageGenCompose(), styleId: "custom-1" },
      templates,
    );
    expect(payload.stylePrompt).toBe("35mm grain");
    expect(payload.exampleImage).toBe("data:image/png;base64,abc");
  });

  it("snaps every preset size to a multiple of 16", () => {
    const ratios = ["auto", "21:9", "16:9", "3:2", "4:3", "1:1", "3:4", "2:3", "9:16"] as const;
    const resolutions = ["1.5k", "2k", "4k"] as const;
    for (const ratio of ratios) {
      for (const resolution of resolutions) {
        const dims = dimensionsFor(ratio, resolution);
        expect(dims.width % 16, `${ratio} ${resolution} width`).toBe(0);
        expect(dims.height % 16, `${ratio} ${resolution} height`).toBe(0);
      }
    }
    expect(dimensionsFor("21:9", "2k")).toEqual({ width: 2048, height: 880 });
    expect(dimensionsFor("3:2", "2k")).toEqual({ width: 2048, height: 1360 });
  });

  it("encodes and decodes image model selection", () => {
    const id = encodeImageModelSelection("openai", "gpt-image-2");
    expect(decodeImageModelSelection(id)).toEqual({
      provider: "openai",
      model: "gpt-image-2",
    });
    expect(decodeImageModelSelection("")).toBeNull();
    expect(decodeImageModelSelection("not-json")).toBeNull();
    expect(decodeImageModelSelection('["openai"]')).toBeNull();
  });

  it("lists enabled image models across providers", () => {
    const choices = listImageModelChoices([
      {
        id: "p1",
        name: "OpenAI",
        models: "gpt-image-2, dall-e-3",
        disabledModels: "dall-e-3",
      },
      { id: "p2", name: "", models: "flux" },
    ]);
    expect(choices.map((item) => item.id)).toEqual([
      encodeImageModelSelection("p1", "gpt-image-2"),
      encodeImageModelSelection("p2", "flux"),
    ]);
    expect(choices[0]?.label).toBe("gpt-image-2 · OpenAI");
    expect(selectedImageModelChoiceId("p1", "gpt-image-2", choices)).toBe(
      encodeImageModelSelection("p1", "gpt-image-2"),
    );
    expect(selectedImageModelChoiceId("p1", "dall-e-3", choices)).toBe("");
  });

  it("does not change compose when applying the model field", () => {
    const current = defaultImageGenCompose();
    expect(applyImageGenField(current, "model", "ignored")).toBe(current);
  });
});
