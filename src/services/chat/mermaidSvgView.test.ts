// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { panSvgView, trimSvgToContent, zoomSvgView } from "@/services/chat/mermaidSvgView";

describe("mermaidSvgView", () => {
  it("trims svg viewBox to content bounds", () => {
    document.body.innerHTML = `
      <svg width="2000" height="1200" viewBox="0 0 2000 1200">
        <g>
          <rect x="100" y="80" width="300" height="180" />
        </g>
      </svg>
    `;
    const svg = document.querySelector("svg") as SVGSVGElement;
    const rect = svg.querySelector("rect") as SVGGraphicsElement;
    rect.getBBox = () => new DOMRect(100, 80, 300, 180);
    const view = trimSvgToContent(svg, 10);
    expect(view.width).toBeLessThan(400);
    expect(view.height).toBeLessThan(260);
    expect(svg.getAttribute("viewBox")).toContain("90");
    document.body.innerHTML = "";
  });

  it("zooms around an anchor ratio", () => {
    const view = { x: 0, y: 0, width: 100, height: 50 };
    const zoomed = zoomSvgView(view, 2, 0.5, 0.5);
    expect(zoomed.width).toBe(50);
    expect(zoomed.height).toBe(25);
    expect(zoomed.x).toBe(25);
    expect(zoomed.y).toBe(12.5);
  });

  it("zooms out when factor is below 1", () => {
    const view = { x: 0, y: 0, width: 100, height: 50 };
    const zoomed = zoomSvgView(view, 1 / 1.2, 0.5, 0.5);
    expect(zoomed.width).toBeGreaterThan(100);
    expect(zoomed.height).toBeGreaterThan(50);
  });
});
