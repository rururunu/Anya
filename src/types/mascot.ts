/** Items the mascot can hold; the caller maps app state onto these. */
export type MascotTool = "phone" | "wrench" | "clipboard" | "brush";

/**
 * Facial state, roughly mirroring an agent turn:
 * idle (blinks, follows) / thinking (eyes drift up) / working (eyes scan) /
 * talking (eyes pulse) / waiting (wide round eyes) / done (happy squint) / error (> < eyes).
 */
export type MascotState =
  "idle" | "thinking" | "working" | "talking" | "waiting" | "done" | "error";
