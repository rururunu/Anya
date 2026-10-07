export type PetShape =
  | "rest"
  | "left"
  | "right"
  | "squash"
  | "hop"
  | "doubleHop"
  | "stretch"
  | "curious"
  | "morph"
  | "twirl"
  | "dizzy";

// All outlines use the same four cubic segments, so every point can move smoothly.
const outlines: Record<PetShape, number[]> = {
  rest: [
    256, 108, 356, 108, 428, 164, 428, 272, 428, 388, 381, 428, 256, 428, 131, 428, 84, 388, 84,
    272, 84, 164, 156, 108, 256, 108,
  ],
  left: [
    245, 112, 348, 100, 425, 155, 420, 267, 416, 388, 374, 428, 252, 428, 113, 428, 76, 391, 76,
    274, 76, 171, 130, 128, 245, 112,
  ],
  right: [
    267, 112, 382, 128, 436, 171, 436, 274, 436, 391, 399, 428, 260, 428, 138, 428, 96, 388, 92,
    267, 87, 155, 164, 100, 267, 112,
  ],
  squash: [
    256, 161, 364, 151, 445, 191, 445, 289, 445, 390, 387, 430, 256, 430, 125, 430, 67, 390, 67,
    289, 67, 191, 148, 151, 256, 161,
  ],
  hop: [
    256, 90, 345, 90, 408, 142, 408, 244, 414, 345, 382, 392, 256, 392, 130, 392, 98, 345, 104, 244,
    104, 142, 167, 90, 256, 90,
  ],
  doubleHop: [
    256, 69, 356, 72, 420, 130, 420, 242, 420, 352, 383, 395, 256, 395, 129, 395, 92, 352, 92, 242,
    92, 130, 156, 72, 256, 69,
  ],
  stretch: [
    256, 56, 350, 56, 402, 148, 402, 267, 408, 383, 377, 428, 256, 428, 135, 428, 104, 383, 110,
    267, 110, 148, 162, 56, 256, 56,
  ],
  curious: [
    256, 99, 355, 82, 435, 146, 427, 265, 427, 386, 382, 428, 256, 428, 130, 428, 85, 386, 85, 265,
    77, 146, 157, 82, 256, 99,
  ],
  twirl: [
    275, 115, 375, 106, 432, 177, 413, 276, 393, 378, 356, 429, 242, 427, 112, 426, 75, 386, 91,
    270, 102, 157, 170, 100, 275, 115,
  ],
  dizzy: [
    256, 141, 369, 133, 446, 174, 440, 278, 434, 381, 372, 426, 256, 426, 140, 426, 78, 381, 72,
    278, 66, 174, 143, 133, 256, 141,
  ],
  morph: [
    256, 117, 335, 68, 439, 139, 427, 258, 415, 374, 411, 429, 256, 428, 101, 427, 97, 374, 85, 258,
    73, 139, 177, 68, 256, 117,
  ],
};

export function petOutline(shape: PetShape): number[] {
  return outlines[shape];
}

export function interpolatePetOutline(
  from: readonly number[],
  to: readonly number[],
  progress: number,
): number[] {
  const t = Math.max(0, Math.min(1, progress));
  return from.map((value, index) => value + (to[index] - value) * t);
}

export function squeezePetOutline(
  points: readonly number[],
  pressure: { x: number; y: number },
): number[] {
  const result = [...points];
  const horizontal = Math.max(-1, Math.min(1, pressure.x));
  const vertical = Math.max(-1, Math.min(1, pressure.y));

  // Touch only the three control points on the contacted side. The opposite
  // silhouette and the face's space stay where they are.
  if (horizontal > 0) {
    result[4] -= 10 * horizontal;
    result[6] -= 20 * horizontal;
    result[8] -= 10 * horizontal;
  } else if (horizontal < 0) {
    result[16] -= 10 * horizontal;
    result[18] -= 20 * horizontal;
    result[20] -= 10 * horizontal;
  }

  if (vertical > 0) {
    result[11] -= 10 * vertical;
    result[13] -= 20 * vertical;
    result[15] -= 10 * vertical;
  } else if (vertical < 0) {
    result[1] -= 20 * vertical;
    result[3] -= 10 * vertical;
    result[23] -= 10 * vertical;
    result[25] -= 20 * vertical;
  }

  return result;
}

export function petPressureEase(progress: number, releasing: boolean): number {
  const t = Math.max(0, Math.min(1, progress));
  if (t === 1) return 1;
  if (!releasing) return 1 - (1 - t) ** 3;
  return 1 - Math.exp(-8 * t) * Math.cos(10 * t);
}

export function petOutlinePath(points: readonly number[]): string {
  const p = points.map((value) => Number(value.toFixed(2)));
  return `M${p[0]} ${p[1]} C${p[2]} ${p[3]} ${p[4]} ${p[5]} ${p[6]} ${p[7]} C${p[8]} ${p[9]} ${p[10]} ${p[11]} ${p[12]} ${p[13]} C${p[14]} ${p[15]} ${p[16]} ${p[17]} ${p[18]} ${p[19]} C${p[20]} ${p[21]} ${p[22]} ${p[23]} ${p[24]} ${p[25]} Z`;
}
