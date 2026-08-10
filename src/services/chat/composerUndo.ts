/**
 * Snapshot undo stack for programmatic composer edits.
 *
 * Native textarea undo covers plain typing/pasting, but edits that bypass the
 * DOM input path — removing a chip with Backspace, truncating a mention on
 * Escape, programmatic setMessage — are invisible to it. The composer pushes a
 * snapshot (message + frozen segments) before each such edit and pops it on
 * Ctrl/Cmd+Z; when the stack is empty the caller falls through to native undo.
 */
import type { ComposerSegment } from "@/services/chat/composerSegments";

export interface ComposerSnapshot {
  message: string;
  /** Frozen segment list; callers must clone before storing. */
  segments: ComposerSegment[];
  /** Segments rendered after the live caret (arrow-key navigation past chips). */
  trailingSegments?: ComposerSegment[];
}

export interface ComposerUndoStack {
  readonly depth: number;
  push(snapshot: ComposerSnapshot): void;
  pop(): ComposerSnapshot | undefined;
}

/** LIFO snapshot stack with a bounded depth (oldest entries evicted). */
export function createComposerUndoStack(cap = 50): ComposerUndoStack {
  const stack: ComposerSnapshot[] = [];
  return {
    get depth() {
      return stack.length;
    },
    push(snapshot) {
      stack.push(snapshot);
      if (stack.length > cap) stack.shift();
    },
    pop() {
      return stack.pop();
    },
  };
}
