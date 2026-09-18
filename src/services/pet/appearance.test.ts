import { describe, expect, it } from "vitest";
import { normalizePetAppearance } from "./appearance";

describe("normalizePetAppearance", () => {
  it("defaults legacy skin payloads to media mode", () => {
    expect(normalizePetAppearance({ kind: "svg", source: "orb.svg" })).toEqual({
      mode: "media",
      kind: "svg",
      source: "orb.svg",
    });
  });

  it("accepts companion mode with config", () => {
    expect(
      normalizePetAppearance({
        mode: "companion",
        config: { accent: "#3366ff", variant: "pill" },
      }),
    ).toEqual({
      mode: "companion",
      config: { accent: "#3366ff", variant: "pill" },
    });
  });

  it("accepts html media", () => {
    expect(
      normalizePetAppearance({
        mode: "media",
        kind: "html",
        source: "anya-plugin://localhost/demo/stage.html",
      }),
    ).toEqual({
      mode: "media",
      kind: "html",
      source: "anya-plugin://localhost/demo/stage.html",
    });
  });
});
