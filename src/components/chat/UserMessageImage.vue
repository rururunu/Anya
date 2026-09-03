<script setup lang="ts">
/**
 * User-bubble thumbnail. Chat history stores desktop paths as `path:…`; those are not
 * valid <img src> values, and convertFileSrc only covers a narrow assetProtocol scope
 * (`$APPDATA/generated/**`). Companion uploads live under companion-inbox / workspace
 * `.anya/uploads`, so local paths are loaded via plugin-fs → data URL.
 */
import { useResolvedChatImageSrc } from "@/composables/chat/useResolvedChatImageSrc";
import { toRef } from "vue";

const props = defineProps<{
  source: string;
}>();

const emit = defineEmits<{
  preview: [source: string];
}>();

const { resolvedSource } = useResolvedChatImageSrc(toRef(props, "source"));
</script>

<template>
  <button
    type="button"
    class="user-image-btn"
    data-tauri-drag-region="false"
    data-no-drag
    aria-label="Preview image"
    @mousedown.stop
    @click.stop.prevent="emit('preview', source)"
  >
    <img v-if="resolvedSource" :src="resolvedSource" class="user-image" alt="" draggable="false" />
    <span v-else class="user-image user-image-placeholder" aria-hidden="true" />
  </button>
</template>

<style scoped>
.user-image-btn {
  display: block;
  margin: 0;
  padding: 0;
  border: none;
  background: transparent;
  border-radius: var(--peek-user-bubble-radius, 18px);
  overflow: hidden;
  cursor: zoom-in;
  max-width: min(280px, 72vw);
  line-height: 0;
  box-shadow: var(--peek-user-bubble-shadow, none);
  transform: translateZ(0);
  transition: box-shadow 140ms ease;
}

.user-image-btn:hover {
  box-shadow:
    var(--peek-user-bubble-shadow, none),
    0 0 0 1px color-mix(in srgb, var(--peek-accent) 35%, transparent);
}

.user-image {
  display: block;
  width: auto;
  height: auto;
  max-width: min(280px, 72vw);
  max-height: 360px;
  object-fit: contain;
  border-radius: inherit;
  user-select: none;
  background: color-mix(in srgb, var(--color-muted, #888) 12%, transparent);
}

.user-image-placeholder {
  width: 96px;
  height: 96px;
  background: color-mix(in srgb, var(--color-muted, #888) 18%, transparent);
}
</style>
