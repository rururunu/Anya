import { describe, expect, it } from "vitest";
import { hasUsableStrokes, normalizeRect } from "./imageEditReference";

describe("imageEditReference", () => {
  it("rejects tiny regions", () => {
    expect(normalizeRect({ x: 0.1, y: 0.1, w: 0.01, h: 0.5 })).toBeNull();
    expect(normalizeRect({ x: 0.1, y: 0.1, w: 0.5, h: 0.01 })).toBeNull();
  });

  it("clamps and normalizes inverted-ish bounds", () => {
    expect(normalizeRect({ x: 0.2, y: 0.3, w: 0.4, h: 0.5 })).toEqual({
      x: 0.2,
      y: 0.3,
      w: 0.4,
      h: 0.5,
    });
    expect(normalizeRect({ x: -0.1, y: 0.9, w: 1.5, h: 0.5 })).toEqual({
      x: 0,
      y: 0.9,
      w: 1,
      h: 0.1,
    });
  });

  it("detects usable freehand strokes", () => {
    expect(hasUsableStrokes([])).toBe(false);
    expect(hasUsableStrokes([{ radius: 0.04, points: [] }])).toBe(false);
    expect(
      hasUsableStrokes([
        {
          radius: 0.04,
          points: [
            { x: 0.2, y: 0.3 },
            { x: 0.25, y: 0.35 },
          ],
        },
      ]),
    ).toBe(true);
  });
});
