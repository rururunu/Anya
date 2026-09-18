import { describe, expect, it } from "vitest";
import {
  expressionToSpriteAnimation,
  frameSourceRect,
  normalizeSpritesheetManifest,
  SPRITESHEET_HEIGHT,
  SPRITESHEET_WIDTH,
} from "./spritesheet";

describe("spritesheet contract", () => {
  it("fills ChatGPT defaults from a minimal manifest", () => {
    const m = normalizeSpritesheetManifest({
      id: "socksy",
      spritesheet: "spritesheet.png",
    });
    expect(m?.frameWidth).toBe(192);
    expect(m?.columns).toBe(8);
    expect(m?.animations?.idle?.row).toBe(0);
    expect(m?.animations?.work?.row).toBe(7);
    expect(SPRITESHEET_WIDTH).toBe(1536);
    expect(SPRITESHEET_HEIGHT).toBe(1872);
  });

  it("maps expressions to animation rows", () => {
    expect(expressionToSpriteAnimation("working")).toBe("work");
    expect(expressionToSpriteAnimation("error")).toBe("fail");
    expect(expressionToSpriteAnimation("done")).toBe("jump");
    expect(expressionToSpriteAnimation("idle", "wave")).toBe("wave");
  });

  it("computes frame source rects", () => {
    const m = normalizeSpritesheetManifest({
      id: "x",
      spritesheet: "s.png",
    })!;
    const anim = m.animations!.work!;
    expect(frameSourceRect(m, anim, 0)).toEqual({
      sx: 0,
      sy: 7 * 208,
      sw: 192,
      sh: 208,
    });
    expect(frameSourceRect(m, anim, 3).sx).toBe(3 * 192);
  });
});
