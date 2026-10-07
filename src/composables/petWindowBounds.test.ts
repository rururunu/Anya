import { describe, expect, it } from "vitest";
import { clampPetPosition, getPetEdgePressure } from "./petWindowBounds";

const monitors = [
  { position: { x: 0, y: 0 }, size: { width: 1920, height: 1080 } },
  { position: { x: -1280, y: 100 }, size: { width: 1280, height: 1024 } },
];

describe("desktop pet window bounds", () => {
  it("keeps the entire pet window visible at every edge", () => {
    const size = { width: 215, height: 215 };
    expect(clampPetPosition({ x: 1900, y: 1050 }, size, monitors)).toEqual({ x: 1705, y: 865 });
    expect(clampPetPosition({ x: -1400, y: 0 }, size, monitors)).toEqual({ x: -1280, y: 100 });
  });

  it("retains valid positions on a secondary monitor", () => {
    expect(clampPetPosition({ x: -800, y: 300 }, { width: 215, height: 215 }, monitors)).toEqual({
      x: -800,
      y: 300,
    });
  });

  it("reports pressure only when requested movement reaches an edge", () => {
    expect(getPetEdgePressure({ x: 190, y: -65 }, { x: 125, y: 0 })).toEqual({ x: 1, y: -1 });
    expect(getPetEdgePressure({ x: 50, y: 50 }, { x: 50, y: 50 })).toEqual({ x: 0, y: 0 });
  });
});
