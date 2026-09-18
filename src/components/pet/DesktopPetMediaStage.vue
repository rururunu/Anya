<template>
  <div v-if="appearance.kind" class="pet-media-stage" aria-hidden="true">
    <video
      v-if="appearance.kind === 'video'"
      class="pet-media"
      :src="resolvedSource"
      muted
      autoplay
      loop
      playsinline
      preload="auto"
    />
    <img
      v-else-if="appearance.kind === 'image' || appearance.kind === 'svg'"
      class="pet-media"
      :src="resolvedSource"
      alt=""
      draggable="false"
    />
    <iframe
      v-else-if="appearance.kind === 'html'"
      class="pet-media pet-media-html"
      :src="resolvedSource"
      sandbox="allow-scripts"
      referrerpolicy="no-referrer"
      title=""
    />
    <div v-else-if="appearance.kind === 'lottie'" ref="lottieEl" class="pet-media" />
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import type { AnimationItem } from "lottie-web";
import type { PetAppearance } from "@/services/pet/appearance";
import { pluginAssetUrl } from "@/services/plugins/ipc";

const props = defineProps<{
  appearance: PetAppearance;
}>();

const resolvedSource = computed(() =>
  props.appearance.source ? pluginAssetUrl(props.appearance.source) : "",
);

const lottieEl = ref<HTMLElement | null>(null);
let lottieAnim: AnimationItem | null = null;

watch(
  [lottieEl, () => props.appearance],
  async ([el, current]) => {
    lottieAnim?.destroy();
    lottieAnim = null;
    if (!el || current.mode !== "media" || current.kind !== "lottie" || !current.source) {
      return;
    }
    const lottie = (await import("lottie-web")).default;
    if (lottieEl.value !== el || props.appearance !== current) return;
    lottieAnim = lottie.loadAnimation({
      container: el,
      renderer: "svg",
      loop: true,
      autoplay: true,
      path: pluginAssetUrl(current.source),
    });
  },
  { flush: "post" },
);

onBeforeUnmount(() => {
  lottieAnim?.destroy();
  lottieAnim = null;
});
</script>

<style scoped>
.pet-media-stage {
  position: absolute;
  inset: 0;
  z-index: 2;
  pointer-events: none;
  display: grid;
  place-items: center;
}
.pet-media {
  width: 100%;
  height: 100%;
  object-fit: contain;
  border: 0;
  background: transparent;
  image-rendering: pixelated;
}
.pet-media-html {
  border-radius: 18px;
  overflow: hidden;
}
</style>
