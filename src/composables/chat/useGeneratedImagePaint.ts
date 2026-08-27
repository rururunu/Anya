/**
 * Freehand region-select paint state for GeneratedImageCard.
 * Keeps canvas/pointer math out of the SFC so the card stays presentation-focused.
 */

import { computed, nextTick, ref, type Ref } from "vue";
import type { NormalizedStroke } from "@/services/chat/imageEditReference";
import { hasUsableStrokes } from "@/services/chat/imageEditReference";

/** Brush radius as a fraction of the shorter displayed edge. */
export const GENERATED_IMAGE_BRUSH_SIZES = {
  fine: 0.018,
  medium: 0.045,
  bold: 0.085,
} as const;

export type GeneratedImageBrushSizeId = keyof typeof GENERATED_IMAGE_BRUSH_SIZES;

const STAGE_SELECTOR = ".generated-image-media.selecting .generated-image-select-stage";

export function useGeneratedImagePaint() {
  const selectingSource = ref("");
  const strokes = ref<NormalizedStroke[]>([]);
  const brushSizeId = ref<GeneratedImageBrushSizeId>("medium");
  const brushRadius = computed(() => GENERATED_IMAGE_BRUSH_SIZES[brushSizeId.value]);
  const hasPaint = computed(() => hasUsableStrokes(strokes.value));

  let pointerId: number | null = null;
  let activeStroke: NormalizedStroke | null = null;
  let imageEl: HTMLImageElement | null = null;
  let paintCanvasEl: HTMLCanvasElement | null = null;

  async function startSelect(source: string) {
    selectingSource.value = source;
    strokes.value = [];
    activeStroke = null;
    pointerId = null;
    await nextTick();
    const stage = document.querySelector(STAGE_SELECTOR);
    if (stage instanceof HTMLElement) bindStage(stage);
  }

  function cancelSelect() {
    selectingSource.value = "";
    strokes.value = [];
    activeStroke = null;
    pointerId = null;
    imageEl = null;
    paintCanvasEl = null;
  }

  function clearStrokes() {
    strokes.value = [];
    activeStroke = null;
    redrawPaintCanvas();
  }

  function bindStage(stage: HTMLElement) {
    const img = stage.querySelector("img");
    const canvas = stage.querySelector("canvas");
    if (!(img instanceof HTMLImageElement) || !(canvas instanceof HTMLCanvasElement)) return;
    imageEl = img;
    paintCanvasEl = canvas;
    syncPaintCanvas();
  }

  function syncPaintCanvasFromStage(event: Event) {
    const img = event.currentTarget;
    if (!(img instanceof HTMLImageElement)) return;
    const stage = img.parentElement;
    if (stage instanceof HTMLElement) bindStage(stage);
  }

  function syncPaintCanvas() {
    const canvas = paintCanvasEl;
    const img = imageEl;
    if (!canvas || !img) return;
    const width = Math.max(1, Math.round(img.clientWidth));
    const height = Math.max(1, Math.round(img.clientHeight));
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
    }
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
    redrawPaintCanvas();
  }

  function paintStrokes(
    ctx: CanvasRenderingContext2D,
    all: NormalizedStroke[],
    width: number,
    height: number,
    shortEdge: number,
  ) {
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.strokeStyle = "#fff";
    ctx.fillStyle = "#fff";
    for (const stroke of all) {
      if (stroke.points.length === 0) continue;
      const radiusPx = Math.max(3, stroke.radius * shortEdge);
      ctx.lineWidth = radiusPx * 2;
      if (stroke.points.length === 1) {
        const point = stroke.points[0]!;
        ctx.beginPath();
        ctx.arc(point.x * width, point.y * height, radiusPx, 0, Math.PI * 2);
        ctx.fill();
        continue;
      }
      ctx.beginPath();
      stroke.points.forEach((point, index) => {
        const x = point.x * width;
        const y = point.y * height;
        if (index === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      });
      ctx.stroke();
    }
  }

  function redrawPaintCanvas() {
    const canvas = paintCanvasEl;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const { width, height } = canvas;
    ctx.clearRect(0, 0, width, height);

    // Full-frame dim; painted strokes punch holes so those areas stay bright.
    ctx.fillStyle = "rgba(0, 0, 0, 0.52)";
    ctx.fillRect(0, 0, width, height);

    const shortEdge = Math.min(width, height);
    ctx.globalCompositeOperation = "destination-out";
    const all = activeStroke ? [...strokes.value, activeStroke] : strokes.value;
    paintStrokes(ctx, all, width, height, shortEdge);
    ctx.globalCompositeOperation = "source-over";

    // Soft lift on the revealed area so it reads as "brighter", not just undimmed.
    const veil = document.createElement("canvas");
    veil.width = width;
    veil.height = height;
    const veilCtx = veil.getContext("2d");
    if (!veilCtx) return;
    veilCtx.fillStyle = "rgba(255, 255, 255, 0.16)";
    veilCtx.fillRect(0, 0, width, height);
    veilCtx.globalCompositeOperation = "destination-in";
    paintStrokes(veilCtx, all, width, height, shortEdge);
    ctx.drawImage(veil, 0, 0);
  }

  function clientToNormalized(event: PointerEvent): { x: number; y: number } | null {
    const img = imageEl;
    if (!img) return null;
    const box = img.getBoundingClientRect();
    if (box.width <= 0 || box.height <= 0) return null;
    return {
      x: Math.min(1, Math.max(0, (event.clientX - box.left) / box.width)),
      y: Math.min(1, Math.max(0, (event.clientY - box.top) / box.height)),
    };
  }

  function onPaintPointerDown(event: PointerEvent, source: string) {
    if (selectingSource.value !== source) return;
    const stage = event.currentTarget;
    if (!(stage instanceof HTMLElement)) return;
    bindStage(stage);
    const point = clientToNormalized(event);
    if (!point) return;
    event.preventDefault();
    stage.setPointerCapture?.(event.pointerId);
    pointerId = event.pointerId;
    activeStroke = { radius: brushRadius.value, points: [point] };
    redrawPaintCanvas();
  }

  function onPaintPointerMove(event: PointerEvent) {
    if (pointerId !== event.pointerId || !activeStroke) return;
    const point = clientToNormalized(event);
    if (!point) return;
    const last = activeStroke.points[activeStroke.points.length - 1];
    if (last) {
      const dx = point.x - last.x;
      const dy = point.y - last.y;
      // Skip near-duplicate points to keep strokes compact.
      if (dx * dx + dy * dy < 0.00002) return;
    }
    activeStroke.points.push(point);
    redrawPaintCanvas();
  }

  function onPaintPointerUp(event: PointerEvent) {
    if (pointerId !== null && event.pointerId !== pointerId) return;
    if (activeStroke && activeStroke.points.length > 0) {
      strokes.value = [...strokes.value, activeStroke];
    }
    activeStroke = null;
    pointerId = null;
    redrawPaintCanvas();
  }

  function onBrushWheel(event: WheelEvent) {
    const order: GeneratedImageBrushSizeId[] = ["fine", "medium", "bold"];
    const index = order.indexOf(brushSizeId.value);
    const next = event.deltaY < 0 ? Math.min(order.length - 1, index + 1) : Math.max(0, index - 1);
    brushSizeId.value = order[next] ?? "medium";
  }

  return {
    selectingSource: selectingSource as Ref<string>,
    strokes: strokes as Ref<NormalizedStroke[]>,
    brushSizeId,
    hasPaint,
    startSelect,
    cancelSelect,
    clearStrokes,
    syncPaintCanvasFromStage,
    onPaintPointerDown,
    onPaintPointerMove,
    onPaintPointerUp,
    onBrushWheel,
  };
}
