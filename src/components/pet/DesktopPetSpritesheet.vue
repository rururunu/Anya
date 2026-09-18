<template>
  <canvas ref="canvasEl" class="pet-spritesheet" aria-hidden="true" />
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { pluginAssetUrl } from "@/services/plugins/ipc";
import {
  expressionToSpriteAnimation,
  frameDurationMs,
  frameSourceRect,
  normalizeSpritesheetManifest,
  resolveAnimation,
  resolveSpritesheetUrl,
  type PetSpriteAnimationId,
  type PetSpritesheetManifest,
} from "@/services/pet/spritesheet";

const props = withDefaults(
  defineProps<{
    /** Absolute or anya-plugin URL to pet.json */
    manifestUrl: string;
    expression?: string;
    /** Transient gesture (wave / run*) overrides expression until done or cleared. */
    gesture?: PetSpriteAnimationId | null;
    reducedMotion?: boolean;
  }>(),
  {
    expression: "idle",
    gesture: null,
    reducedMotion: false,
  },
);

const canvasEl = ref<HTMLCanvasElement | null>(null);
let manifest: PetSpritesheetManifest | null = null;
let sheetImage: HTMLImageElement | null = null;
let frameIndex = 0;
let timer: ReturnType<typeof setTimeout> | null = null;
let loadToken = 0;
let preferReduced = false;

function clearTimer() {
  if (timer) {
    clearTimeout(timer);
    timer = null;
  }
}

function currentAnimationId(): PetSpriteAnimationId {
  return expressionToSpriteAnimation(props.expression, props.gesture);
}

function paint() {
  const canvas = canvasEl.value;
  const img = sheetImage;
  const m = manifest;
  if (!canvas || !img || !m) return;
  const anim = resolveAnimation(m, currentAnimationId());
  const { sx, sy, sw, sh } = frameSourceRect(m, anim, frameIndex);
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const cssW = canvas.clientWidth || sw;
  const cssH = canvas.clientHeight || sh;
  const tw = Math.max(1, Math.round(cssW * dpr));
  const th = Math.max(1, Math.round(cssH * dpr));
  if (canvas.width !== tw || canvas.height !== th) {
    canvas.width = tw;
    canvas.height = th;
  }
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.clearRect(0, 0, tw, th);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(img, sx, sy, sw, sh, 0, 0, tw, th);
}

function scheduleNext() {
  clearTimer();
  const m = manifest;
  if (!m || preferReduced || props.reducedMotion) {
    paint();
    return;
  }
  const anim = resolveAnimation(m, currentAnimationId());
  paint();
  const delay = frameDurationMs(anim, frameIndex);
  timer = setTimeout(() => {
    const next = frameIndex + 1;
    if (next >= anim.frames) {
      if (anim.loop === false) {
        frameIndex = Math.max(0, anim.frames - 1);
        paint();
        return;
      }
      frameIndex = 0;
    } else {
      frameIndex = next;
    }
    scheduleNext();
  }, delay);
}

function restartPlayback() {
  frameIndex = 0;
  scheduleNext();
}

async function loadManifest(url: string) {
  const token = ++loadToken;
  clearTimer();
  manifest = null;
  sheetImage = null;
  const resolved = pluginAssetUrl(url);
  const res = await fetch(resolved);
  if (!res.ok) throw new Error(`pet manifest ${res.status}`);
  const json = await res.json();
  if (token !== loadToken) return;
  const m = normalizeSpritesheetManifest(json);
  if (!m) throw new Error("invalid pet manifest");
  const sheetUrl = pluginAssetUrl(resolveSpritesheetUrl(resolved, m.spritesheet));
  const img = new Image();
  img.decoding = "async";
  await new Promise<void>((resolve, reject) => {
    img.onload = () => resolve();
    img.onerror = () => reject(new Error("spritesheet load failed"));
    img.src = sheetUrl;
  });
  if (token !== loadToken) return;
  manifest = m;
  sheetImage = img;
  restartPlayback();
}

function syncReducedMotion() {
  preferReduced =
    props.reducedMotion ||
    (typeof window !== "undefined" &&
      window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true);
}

onMounted(() => {
  syncReducedMotion();
  if (props.manifestUrl) {
    void loadManifest(props.manifestUrl).catch((err) => {
      console.error("DesktopPetSpritesheet", err);
    });
  }
});

watch(
  () => props.manifestUrl,
  (url) => {
    if (!url) return;
    void loadManifest(url).catch((err) => {
      console.error("DesktopPetSpritesheet", err);
    });
  },
);

watch(
  () => [props.expression, props.gesture, props.reducedMotion] as const,
  () => {
    syncReducedMotion();
    restartPlayback();
  },
);

onBeforeUnmount(() => {
  loadToken += 1;
  clearTimer();
  sheetImage = null;
  manifest = null;
});
</script>

<style scoped>
.pet-spritesheet {
  width: 100%;
  height: 100%;
  display: block;
  background: transparent;
  pointer-events: none;
}
</style>
