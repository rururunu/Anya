<template>
  <div class="mermaid-card">
    <header class="mermaid-card-header">
      <span class="mermaid-card-label">{{ labels.diagram }}</span>
      <div class="mermaid-card-actions">
        <button
          type="button"
          class="mermaid-action"
          :title="labels.zoomOut"
          :aria-label="labels.zoomOut"
          @click="zoomBy(1 / 1.2)"
        >
          <ZoomOut :size="13" :stroke-width="2" aria-hidden="true" />
        </button>
        <span class="mermaid-zoom-level">{{ zoomPercent }}%</span>
        <button
          type="button"
          class="mermaid-action"
          :title="labels.zoomIn"
          :aria-label="labels.zoomIn"
          @click="zoomBy(1.2)"
        >
          <ZoomIn :size="13" :stroke-width="2" aria-hidden="true" />
        </button>
        <button
          type="button"
          class="mermaid-action"
          :title="labels.resetView"
          :aria-label="labels.resetView"
          @click="fitToView"
        >
          <Scan :size="13" :stroke-width="2" aria-hidden="true" />
        </button>
        <span class="mermaid-action-divider" aria-hidden="true" />
        <button
          type="button"
          class="mermaid-action"
          :class="{ active: showSource }"
          :title="showSource ? labels.hideSource : labels.showSource"
          :aria-label="showSource ? labels.hideSource : labels.showSource"
          :aria-pressed="showSource"
          @click="toggleSource"
        >
          <FileCode2 :size="13" :stroke-width="2" aria-hidden="true" />
        </button>
      </div>
    </header>

    <div v-if="showSource" class="mermaid-source-panel peek-scrollbar">
      <pre class="mermaid-source-code">{{ sourceText }}</pre>
    </div>
    <div
      v-else
      ref="viewportRef"
      class="mermaid-viewport"
      :class="{ dragging: isDragging, 'is-ready': ready }"
      @wheel.prevent="onWheel"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
      @dblclick.prevent="fitToView"
    >
      <div ref="stageRef" class="mermaid-stage" v-html="svgMarkup" />
      <p v-if="pending" class="mermaid-status">{{ labels.rendering }}</p>
      <p v-else-if="errorMessage" class="mermaid-status error" role="alert">{{ errorMessage }}</p>
      <p v-else class="mermaid-hint">{{ labels.hint }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { FileCode2, Scan, ZoomIn, ZoomOut } from "@lucide/vue";
import { onThemeChange, useTheme } from "@/composables/useTheme";
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import {
  applySvgViewBox,
  clampViewZoom,
  panSvgView,
  trimSvgToContent,
  zoomSvgView,
  type SvgViewBox,
} from "@/services/chat/mermaidSvgView";
import { buildMermaidThemeVariables } from "@/services/chat/mermaidTheme";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";

const props = defineProps<{
  source: string;
}>();

const settingStore = useSettingStore();
const { isDark } = useTheme();
const viewportRef = ref<HTMLDivElement | null>(null);
const stageRef = ref<HTMLDivElement | null>(null);
const svgMarkup = ref("");
const errorMessage = ref("");
const pending = ref(false);
const ready = ref(false);
const isDragging = ref(false);
const showSource = ref(false);
const zoomPercent = ref(100);

let renderCounter = 0;
let mermaidModule: typeof import("mermaid").default | null = null;
let bindFunctions: ((element: HTMLElement) => void) | undefined;
let dragStart = { x: 0, y: 0 };
let baseView: SvgViewBox | null = null;
let currentView: SvgViewBox | null = null;
let disposeThemeListener: (() => void) | null = null;
let resizeObserver: ResizeObserver | null = null;

const labels = computed(() => ({
  diagram: tr(settingStore.language, "mermaid.diagram"),
  zoomIn: tr(settingStore.language, "mermaid.zoomIn"),
  zoomOut: tr(settingStore.language, "mermaid.zoomOut"),
  resetView: tr(settingStore.language, "mermaid.resetView"),
  showSource: tr(settingStore.language, "mermaid.showSource"),
  hideSource: tr(settingStore.language, "mermaid.hideSource"),
  rendering: tr(settingStore.language, "mermaid.rendering"),
  hint: tr(settingStore.language, "mermaid.panZoomHint"),
}));

const sourceText = computed(() => props.source.trim());

function toggleSource() {
  showSource.value = !showSource.value;
  if (!showSource.value) {
    void nextTick(() => {
      if (ready.value) fitToView();
    });
  }
}

async function loadMermaid() {
  if (!mermaidModule) {
    mermaidModule = (await import("mermaid")).default;
  }
  return mermaidModule;
}

function configureMermaid(mermaid: typeof import("mermaid").default) {
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: "loose",
    theme: "base",
    themeVariables: buildMermaidThemeVariables(isDark.value),
    flowchart: {
      useMaxWidth: false,
      htmlLabels: true,
    },
    sequence: {
      useMaxWidth: false,
    },
    gantt: {
      useMaxWidth: false,
    },
  });
}

function getSvgElement(): SVGSVGElement | null {
  return stageRef.value?.querySelector("svg") ?? null;
}

function syncViewToSvg() {
  const svg = getSvgElement();
  if (!svg || !currentView) return;
  applySvgViewBox(svg, currentView);
  if (baseView) {
    zoomPercent.value = Math.round((baseView.width / currentView.width) * 100);
  }
}

async function renderDiagram() {
  const trimmed = props.source.trim();
  if (!trimmed) {
    svgMarkup.value = "";
    errorMessage.value = tr(settingStore.language, "mermaid.empty");
    ready.value = false;
    baseView = null;
    currentView = null;
    return;
  }

  pending.value = true;
  errorMessage.value = "";
  ready.value = false;

  try {
    const mermaid = await loadMermaid();
    configureMermaid(mermaid);
    const id = `mermaid-card-${++renderCounter}`;
    const result = await mermaid.render(id, trimmed);
    svgMarkup.value = result.svg;
    bindFunctions = result.bindFunctions;
    pending.value = false;
    await nextTick();
    if (stageRef.value) {
      bindFunctions?.(stageRef.value);
    }
    const svg = getSvgElement();
    if (!svg) throw new Error("Mermaid SVG was not created.");

    baseView = trimSvgToContent(svg, 24);
    currentView = { ...baseView };
    syncViewToSvg();
    ready.value = true;
    fitToView();
  } catch (error) {
    pending.value = false;
    ready.value = false;
    svgMarkup.value = "";
    baseView = null;
    currentView = null;
    errorMessage.value = error instanceof Error ? error.message : String(error);
    console.error("mermaid render failed:", error);
  }
}

function fitToView() {
  if (!baseView) return;
  currentView = { ...baseView };
  syncViewToSvg();
}

function zoomBy(factor: number, anchorX?: number, anchorY?: number) {
  const viewport = viewportRef.value;
  if (!viewport || !currentView || !baseView) return;

  const ratioX = anchorX == null ? 0.5 : anchorX / Math.max(viewport.clientWidth, 1);
  const ratioY = anchorY == null ? 0.5 : anchorY / Math.max(viewport.clientHeight, 1);
  const next = zoomSvgView(currentView, factor, ratioX, ratioY);
  currentView = clampViewZoom(next, baseView, 0.35, 8);
  syncViewToSvg();
}

function onWheel(event: WheelEvent) {
  if (!ready.value) return;
  const viewport = viewportRef.value;
  if (!viewport) return;
  const rect = viewport.getBoundingClientRect();
  const factor = event.deltaY > 0 ? 1 / 1.14 : 1.14;
  zoomBy(factor, event.clientX - rect.left, event.clientY - rect.top);
}

function onPointerDown(event: PointerEvent) {
  if (!ready.value || event.button !== 0) return;
  if ((event.target as HTMLElement).closest(".mermaid-card-actions")) return;
  isDragging.value = true;
  dragStart = { x: event.clientX, y: event.clientY };
  viewportRef.value?.setPointerCapture(event.pointerId);
}

function onPointerMove(event: PointerEvent) {
  if (!isDragging.value || !currentView) return;
  const viewport = viewportRef.value;
  if (!viewport) return;
  const deltaX = event.clientX - dragStart.x;
  const deltaY = event.clientY - dragStart.y;
  dragStart = { x: event.clientX, y: event.clientY };
  currentView = panSvgView(
    currentView,
    deltaX,
    deltaY,
    viewport.clientWidth,
    viewport.clientHeight,
  );
  syncViewToSvg();
}

function onPointerUp(event: PointerEvent) {
  if (!isDragging.value) return;
  isDragging.value = false;
  viewportRef.value?.releasePointerCapture(event.pointerId);
}

watch(
  () => props.source,
  () => {
    void renderDiagram();
  },
);

watch(isDark, () => {
  void renderDiagram();
});

onMounted(() => {
  void renderDiagram();
  disposeThemeListener = onThemeChange(() => {
    void renderDiagram();
  });
  void nextTick(() => {
    if (!viewportRef.value) return;
    resizeObserver = new ResizeObserver(() => {
      if (ready.value && baseView && currentView) {
        const atBase =
          Math.abs(currentView.width - baseView.width) < 0.5 &&
          Math.abs(currentView.height - baseView.height) < 0.5;
        if (atBase) fitToView();
      }
    });
    resizeObserver.observe(viewportRef.value);
  });
});

onUnmounted(() => {
  disposeThemeListener?.();
  disposeThemeListener = null;
  resizeObserver?.disconnect();
  resizeObserver = null;
});
</script>

<style scoped>
.mermaid-card {
  box-sizing: border-box;
  margin: 0;
  overflow: hidden;
  border: 1px solid var(--peek-code-border);
  border-radius: 10px;
  background: var(--peek-code-bg);
  box-shadow: var(--peek-code-shadow);
}

.mermaid-card-header {
  box-sizing: border-box;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  min-height: 30px;
  padding: 5px 8px 5px 10px;
  border-bottom: 1px solid var(--peek-code-border);
  background: var(--peek-code-toolbar-bg);
}

.mermaid-card-label {
  min-width: 0;
  overflow: hidden;
  color: var(--peek-text);
  font-size: 11px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mermaid-card-actions {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.mermaid-zoom-level {
  min-width: 38px;
  color: var(--peek-muted);
  font-family: var(--font-mono);
  font-size: 10px;
  text-align: center;
}

.mermaid-action {
  box-sizing: border-box;
  display: inline-grid;
  place-items: center;
  width: 26px;
  height: 26px;
  padding: 0;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--peek-code-icon, var(--peek-muted));
  cursor: pointer;
  transition:
    background-color var(--motion-fast, 120ms) ease,
    color var(--motion-fast, 120ms) ease;
}

.mermaid-action:hover {
  background: color-mix(in srgb, var(--peek-text) 8%, transparent);
  color: var(--peek-text);
}

.mermaid-action:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--peek-accent) 55%, transparent);
  outline-offset: 1px;
}

.mermaid-action.active {
  background: color-mix(in srgb, var(--peek-accent) 12%, transparent);
  color: var(--peek-text);
}

.mermaid-action-divider {
  flex: none;
  width: 1px;
  height: 16px;
  margin: 0 2px;
  background: var(--peek-code-border);
}

.mermaid-source-panel {
  height: min(520px, 62vh);
  min-height: 300px;
  overflow: auto;
  padding: 12px 14px;
  background: var(--peek-code-body-bg);
}

.mermaid-source-code {
  margin: 0;
  color: color-mix(in srgb, var(--peek-code-fg) 88%, var(--peek-code-muted));
  font-family: var(--font-mono);
  font-size: 11.5px;
  line-height: 1.7;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  word-break: break-word;
}

.mermaid-viewport {
  position: relative;
  height: min(520px, 62vh);
  min-height: 300px;
  overflow: hidden;
  background:
    radial-gradient(
      circle at 1px 1px,
      color-mix(in srgb, var(--peek-border) 55%, transparent) 1px,
      transparent 0
    ),
    var(--peek-code-body-bg);
  background-size: 18px 18px;
  cursor: grab;
  touch-action: none;
  user-select: none;
}

.mermaid-viewport.dragging {
  cursor: grabbing;
}

.mermaid-stage {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}

.mermaid-stage :deep(svg) {
  display: block;
  width: 100%;
  height: 100%;
  max-width: none;
  max-height: none;
}

.mermaid-stage :deep(.edgeLabel),
.mermaid-stage :deep(.label),
.mermaid-stage :deep(.nodeLabel),
.mermaid-stage :deep(.cluster-label) {
  color: var(--peek-text) !important;
  fill: var(--peek-text) !important;
}

.mermaid-status,
.mermaid-hint {
  position: absolute;
  left: 50%;
  bottom: 10px;
  transform: translateX(-50%);
  margin: 0;
  padding: 4px 10px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--peek-code-bg) 88%, transparent);
  border: 1px solid var(--peek-code-border);
  color: var(--peek-muted);
  font-size: 10px;
  line-height: 1.4;
  pointer-events: none;
  white-space: nowrap;
}

.mermaid-status.error {
  color: var(--peek-danger);
  white-space: pre-wrap;
  max-width: calc(100% - 24px);
  text-align: center;
}

.mermaid-viewport.is-ready .mermaid-hint {
  opacity: 0.72;
}

.mermaid-viewport:not(.is-ready) .mermaid-hint {
  display: none;
}
</style>
