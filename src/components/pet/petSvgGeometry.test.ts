import { describe, expect, it } from "vitest";
import {
  interpolatePetOutline,
  petOutline,
  petOutlinePath,
  petPressureEase,
  squeezePetOutline,
} from "./petSvgGeometry";

describe("pet SVG outline", () => {
  it("keeps every reaction on the same cubic path topology", () => {
    for (const shape of [
      "rest",
      "left",
      "right",
      "squash",
      "hop",
      "doubleHop",
      "stretch",
      "curious",
      "morph",
      "twirl",
      "dizzy",
    ] as const) {
      const outline = petOutline(shape);
      expect(outline).toHaveLength(26);
      expect(petOutlinePath(outline).match(/ C/g)).toHaveLength(4);
      expect(petOutlinePath(outline).endsWith(" Z")).toBe(true);
    }
  });

  it("interpolates actual SVG control points without changing either source", () => {
    const rest = petOutline("rest");
    const hop = petOutline("hop");
    const halfway = interpolatePetOutline(rest, hop, 0.5);
    expect(halfway[1]).toBe(99);
    expect(halfway[13]).toBe(410);
    expect(rest[1]).toBe(108);
    expect(hop[1]).toBe(90);
  });

  it("deforms only the contacted side while leaving the opposite side intact", () => {
    const rest = petOutline("rest");
    const right = squeezePetOutline(rest, { x: 1, y: 0 });
    expect(right[6]).toBe(rest[6] - 20);
    expect(right[18]).toBe(rest[18]);
    expect(right[1]).toBe(rest[1]);
    expect(right[13]).toBe(rest[13]);

    const left = squeezePetOutline(rest, { x: -1, y: 0 });
    expect(left[18]).toBe(rest[18] + 20);
    expect(left[6]).toBe(rest[6]);

    const top = squeezePetOutline(rest, { x: 0, y: -1 });
    expect(top[1]).toBe(rest[1] + 20);
    expect(top[13]).toBe(rest[13]);

    const bottom = squeezePetOutline(rest, { x: 0, y: 1 });
    expect(bottom[13]).toBe(rest[13] - 20);
    expect(bottom[1]).toBe(rest[1]);
  });

  it("adds a small release overshoot and settles exactly at rest", () => {
    expect(petPressureEase(0.3, true)).toBeGreaterThan(1);
    expect(petPressureEase(1, true)).toBe(1);
    expect(petPressureEase(1, false)).toBe(1);
  });
});
