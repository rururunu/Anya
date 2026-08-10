// @vitest-environment happy-dom
import { describe, expect, it } from "vitest";
import {
  createComposerTokenElement,
  getComposerSelectionOffsets,
  renderComposerEditable,
  serializeComposerEditable,
  setComposerSelectionOffsets,
  type ComposerTokenMeta,
  type ResolveComposerTokenMeta,
} from "./composerEditableDom";

const resolveMeta: ResolveComposerTokenMeta = (part) => {
  if (part.kind === "mention") {
    return {
      kind: "mention",
      token: part.raw,
      label: `@${part.name}`,
      title: part.path,
      iconUrl: null,
      fallback: part.isDir ? "folder" : "file",
      className: "ce-token-file",
    } satisfies ComposerTokenMeta;
  }
  if (part.kind === "skill") {
    return {
      kind: "skill",
      token: part.raw,
      label: part.id,
      iconUrl: null,
      fallback: "zap",
      className: "ce-token-skill",
    };
  }
  return {
    kind: "mcp",
    token: part.raw,
    label: part.id,
    iconUrl: null,
    fallback: "bot",
    className: "ce-token-mcp",
  };
};

describe("composerEditableDom", () => {
  it("round-trips plain text and tokens", () => {
    const root = document.createElement("div");
    const text = "see @src/a.ts and #skill:docx please";
    renderComposerEditable(root, text, resolveMeta);
    expect(serializeComposerEditable(root)).toBe(text);
    expect(root.querySelectorAll("[data-ce-token]")).toHaveLength(2);
  });

  it("maps caret offsets across atomic tokens", () => {
    const root = document.createElement("div");
    document.body.appendChild(root);
    renderComposerEditable(root, "ab @x.ts cd", resolveMeta);
    setComposerSelectionOffsets(root, 0);
    expect(getComposerSelectionOffsets(root)).toEqual({ start: 0, end: 0 });
    setComposerSelectionOffsets(root, 3);
    expect(getComposerSelectionOffsets(root).start).toBe(3);
    // After "@x.ts" (length 5) → offset 3+5=8
    setComposerSelectionOffsets(root, 8);
    expect(getComposerSelectionOffsets(root).start).toBe(8);
    setComposerSelectionOffsets(root, 11);
    expect(getComposerSelectionOffsets(root).start).toBe(11);
    root.remove();
  });

  it("creates non-editable token marks", () => {
    const el = createComposerTokenElement({
      kind: "skill",
      token: "#skill:docx",
      label: "Docx",
      fallback: "zap",
      className: "ce-token-skill",
    });
    expect(el.contentEditable).toBe("false");
    expect(el.dataset.ceToken).toBe("#skill:docx");
    expect(el.textContent).toContain("Docx");
  });
});
