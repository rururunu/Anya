import { describe, expect, it } from "vitest";
import { createComposerUndoStack, type ComposerSnapshot } from "@/services/chat/composerUndo";
import type { ComposerSegment } from "@/services/chat/composerSegments";

describe("createComposerUndoStack", () => {
  it("pops snapshots in LIFO order", () => {
    const stack = createComposerUndoStack();
    stack.push({ message: "a", segments: [] });
    stack.push({ message: "b", segments: [] });
    expect(stack.depth).toBe(2);
    expect(stack.pop()).toEqual({ message: "b", segments: [] });
    expect(stack.pop()).toEqual({ message: "a", segments: [] });
    expect(stack.pop()).toBeUndefined();
  });

  it("evicts the oldest snapshot beyond the cap", () => {
    const stack = createComposerUndoStack(2);
    stack.push({ message: "1", segments: [] });
    stack.push({ message: "2", segments: [] });
    stack.push({ message: "3", segments: [] });
    expect(stack.depth).toBe(2);
    expect(stack.pop()?.message).toBe("3");
    expect(stack.pop()?.message).toBe("2");
    expect(stack.pop()).toBeUndefined();
  });

  it("keeps stored snapshots independent of later segment mutations", () => {
    const stack = createComposerUndoStack();
    const segments: ComposerSegment[] = [{ kind: "mention", path: "src/a.ts" }];
    const snapshot: ComposerSnapshot = {
      message: "before",
      segments: segments.map((s) => ({ ...s })),
    };
    stack.push(snapshot);
    segments[0] = { kind: "mention", path: "mutated.ts" };
    expect(stack.pop()).toEqual({
      message: "before",
      segments: [{ kind: "mention", path: "src/a.ts" }],
    });
  });

  it("returns undefined when popping an empty stack", () => {
    const stack = createComposerUndoStack();
    expect(stack.pop()).toBeUndefined();
  });
});
