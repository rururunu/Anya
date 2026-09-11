<template>
  <div v-if="asset" class="workbench-plugin-backdrop" aria-hidden="true">
    <video
      v-if="asset.kind === 'video'"
      ref="videoEl"
      class="workbench-plugin-backdrop-media"
      :src="playableSrc"
      :muted="true"
      autoplay
      loop
      playsinline
      preload="auto"
    />
    <div
      v-else-if="asset.kind === 'image'"
      class="workbench-plugin-backdrop-media"
      :style="imageStyle"
    />
    <div
      v-else-if="asset.kind === 'lottie'"
      ref="lottieEl"
      class="workbench-plugin-backdrop-media"
    />
    <div class="workbench-plugin-backdrop-overlay" />
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch, type CSSProperties } from "vue";
import type { AnimationItem } from "lottie-web";
import { getAssetOverride } from "@/composables/plugins/assetRegistry";
import { pluginAssetUrl } from "@/services/plugins/ipc";
import { useResolvedBackgroundSrc } from "@/composables/theme/useResolvedBackgroundSrc";

const props = defineProps<{ windowFocused?: boolean }>();

const asset = computed(() => getAssetOverride("workbench.backdrop"));
const playableSrc = computed(() => (asset.value ? pluginAssetUrl(asset.value.source) : ""));
const { resolvedSource } = useResolvedBackgroundSrc(playableSrc);

const videoEl = ref<HTMLVideoElement | null>(null);
const lottieEl = ref<HTMLElement | null>(null);
const reduceMotion = ref(false);
let lottieAnim: AnimationItem | null = null;
let motionQuery: MediaQueryList | null = null;

const imageStyle = computed<CSSProperties>(() => {
  if (!resolvedSource.value) return {};
  return {
    backgroundImage: `url(${JSON.stringify(resolvedSource.value)})`,
    backgroundSize: "cover",
    backgroundPosition: "center",
    backgroundRepeat: "no-repeat",
  };
});

function shouldPlay(): boolean {
  return props.windowFocused !== false && !reduceMotion.value;
}

function syncVideo() {
  const el = videoEl.value;
  if (!el) return;
  try {
    if (shouldPlay()) void el.play().catch(() => {});
    else el.pause();
  } catch {
    /* jsdom has no media playback */
  }
}

function onMotionChange() {
  reduceMotion.value = Boolean(motionQuery?.matches);
}

onMounted(() => {
  motionQuery = window.matchMedia?.("(prefers-reduced-motion: reduce)") ?? null;
  if (!motionQuery) return;
  reduceMotion.value = motionQuery.matches;
  motionQuery.addEventListener("change", onMotionChange);
});

onBeforeUnmount(() => {
  motionQuery?.removeEventListener("change", onMotionChange);
  lottieAnim?.destroy();
  lottieAnim = null;
});

watch([videoEl, playableSrc, () => props.windowFocused, reduceMotion], syncVideo);

watch([lottieEl, asset, () => props.windowFocused, reduceMotion], async ([el, current]) => {
  lottieAnim?.destroy();
  lottieAnim = null;
  if (!el || current?.kind !== "lottie") return;
  const lottie = (await import("lottie-web")).default;
  if (lottieEl.value !== el || getAssetOverride("workbench.backdrop") !== current) return;
  lottieAnim = lottie.loadAnimation({
    container: el,
    renderer: "svg",
    loop: !reduceMotion.value,
    autoplay: shouldPlay(),
    path: pluginAssetUrl(current.source),
  });
  if (!shouldPlay()) lottieAnim.pause();
});
</script>

<style scoped>
.workbench-plugin-backdrop {
  position: absolute;
  inset: 0;
  z-index: 0;
  pointer-events: none;
  overflow: hidden;
}
.workbench-plugin-backdrop-media {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  background-size: cover;
  background-position: center;
}
.workbench-plugin-backdrop-overlay {
  position: absolute;
  inset: 0;
  background: color-mix(in srgb, var(--peek-bg) 8%, transparent);
}
</style>
