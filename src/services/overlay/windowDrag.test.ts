// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { onWindowDragMouseDown } from "./windowDrag";

const startDragging = vi.hoisted(() => vi.fn().mockResolvedValue(undefined));
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({ startDragging }),
}));

afterEach(() => {
  document.body.replaceChildren();
  startDragging.mockClear();
});

function panel() {
  const root = document.createElement("div");
  root.setAttribute("data-tauri-drag-region", "");
  root.addEventListener("mousedown", onWindowDragMouseDown);
  root.innerHTML =
    '<div class="composer-dock" data-tauri-drag-region="false"><div class="input-footer"><span class="footer-space"></span></div></div><div class="header-space"></div>';
  document.body.append(root);
  return root;
}

describe("overlay drag boundaries", () => {
  it("does not start native dragging from the bottom bar or its empty descendants", () => {
    const root = panel();
    for (const selector of [".composer-dock", ".input-footer", ".footer-space"]) {
      root
        .querySelector(selector)!
        .dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
    }
    expect(startDragging).not.toHaveBeenCalled();
  });

  it("still allows dragging the window header", () => {
    const root = panel();
    root
      .querySelector(".header-space")!
      .dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
    expect(startDragging).toHaveBeenCalledTimes(1);
  });

  it("respects a click already handled by the input and avoids a second drag", () => {
    const root = panel();
    const header = root.querySelector(".header-space")!;
    header.addEventListener("mousedown", (event) => event.preventDefault());
    header.dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }),
    );
    expect(startDragging).not.toHaveBeenCalled();
  });
});
