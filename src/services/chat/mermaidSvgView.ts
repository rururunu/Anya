export type SvgViewBox = {
  x: number;
  y: number;
  width: number;
  height: number;
};

const MIN_VIEW_SIZE = 1;

export function readSvgViewBox(svg: SVGSVGElement): SvgViewBox {
  const viewBox = svg.viewBox.baseVal;
  if (viewBox.width > 0 && viewBox.height > 0) {
    return {
      x: viewBox.x,
      y: viewBox.y,
      width: viewBox.width,
      height: viewBox.height,
    };
  }
  const width = Number.parseFloat(svg.getAttribute("width") ?? "") || svg.clientWidth || 1;
  const height = Number.parseFloat(svg.getAttribute("height") ?? "") || svg.clientHeight || 1;
  return { x: 0, y: 0, width, height };
}

/** Trim huge Mermaid margins so fit-to-view uses real diagram bounds. */
export function trimSvgToContent(svg: SVGSVGElement, padding = 20): SvgViewBox {
  const bbox = measureSvgContentBox(svg);
  if (!bbox) return readSvgViewBox(svg);

  const view = {
    x: bbox.x - padding,
    y: bbox.y - padding,
    width: bbox.width + padding * 2,
    height: bbox.height + padding * 2,
  };

  svg.setAttribute("viewBox", `${view.x} ${view.y} ${view.width} ${view.height}`);
  svg.setAttribute("preserveAspectRatio", "xMidYMid meet");
  svg.removeAttribute("width");
  svg.removeAttribute("height");
  svg.style.width = "100%";
  svg.style.height = "100%";
  svg.style.display = "block";
  svg.style.maxWidth = "none";

  return view;
}

function measureSvgContentBox(svg: SVGSVGElement): DOMRect | null {
  const graphics = svg.querySelectorAll<SVGGraphicsElement>(
    "g, rect, path, text, line, polyline, polygon, circle, ellipse",
  );
  let minX = Number.POSITIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let maxY = Number.NEGATIVE_INFINITY;
  let found = false;

  for (const element of graphics) {
    let bbox: DOMRect;
    try {
      bbox = element.getBBox();
    } catch {
      continue;
    }
    if (
      !Number.isFinite(bbox.width) ||
      !Number.isFinite(bbox.height) ||
      bbox.width <= 0 ||
      bbox.height <= 0
    ) {
      continue;
    }
    minX = Math.min(minX, bbox.x);
    minY = Math.min(minY, bbox.y);
    maxX = Math.max(maxX, bbox.x + bbox.width);
    maxY = Math.max(maxY, bbox.y + bbox.height);
    found = true;
  }

  if (!found) return null;
  return new DOMRect(minX, minY, maxX - minX, maxY - minY);
}

export function applySvgViewBox(svg: SVGSVGElement, view: SvgViewBox) {
  svg.setAttribute(
    "viewBox",
    `${view.x} ${view.y} ${Math.max(view.width, MIN_VIEW_SIZE)} ${Math.max(view.height, MIN_VIEW_SIZE)}`,
  );
}

export function clampViewZoom(
  view: SvgViewBox,
  base: SvgViewBox,
  minZoom = 0.25,
  maxZoom = 6,
): SvgViewBox {
  const zoom = base.width / Math.max(view.width, MIN_VIEW_SIZE);
  const clampedZoom = Math.min(maxZoom, Math.max(minZoom, zoom));
  if (clampedZoom === zoom) return view;

  const centerX = view.x + view.width / 2;
  const centerY = view.y + view.height / 2;
  const width = base.width / clampedZoom;
  const height = base.height / clampedZoom;
  return {
    x: centerX - width / 2,
    y: centerY - height / 2,
    width,
    height,
  };
}

export function zoomSvgView(
  view: SvgViewBox,
  factor: number,
  anchorRatioX: number,
  anchorRatioY: number,
): SvgViewBox {
  const safeFactor = factor > 0 ? factor : 1;
  const anchorX = view.x + view.width * anchorRatioX;
  const anchorY = view.y + view.height * anchorRatioY;
  const width = view.width / safeFactor;
  const height = view.height / safeFactor;
  return {
    x: anchorX - width * anchorRatioX,
    y: anchorY - height * anchorRatioY,
    width: Math.max(width, MIN_VIEW_SIZE),
    height: Math.max(height, MIN_VIEW_SIZE),
  };
}

export function panSvgView(
  view: SvgViewBox,
  deltaX: number,
  deltaY: number,
  viewportWidth: number,
  viewportHeight: number,
): SvgViewBox {
  const scaleX = view.width / Math.max(viewportWidth, 1);
  const scaleY = view.height / Math.max(viewportHeight, 1);
  return {
    ...view,
    x: view.x - deltaX * scaleX,
    y: view.y - deltaY * scaleY,
  };
}
