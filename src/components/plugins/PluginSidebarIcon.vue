<template>
  <img
    v-if="isUrl && !imgFailed"
    :src="resolvedSrc"
    class="plugin-sidebar-icon-img"
    :style="imgStyle"
    alt=""
    @error="imgFailed = true"
  />
  <component v-else :is="fallbackIcon" :size="size" :stroke-width="1.75" aria-hidden="true" />
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  Book,
  Bot,
  Cat,
  Clipboard,
  Code,
  FileText,
  Film,
  Image,
  Palette,
  Puzzle,
  Sparkles,
  StickyNote,
  Terminal,
  Video,
  Wrench,
  Zap,
} from "@lucide/vue";
import { isPluginIconUrl } from "@/composables/plugins/pluginIcon";
import { pluginAssetUrl } from "@/services/plugins/ipc";

const GLYPHS = {
  book: Book,
  bot: Bot,
  cat: Cat,
  clipboard: Clipboard,
  code: Code,
  document: FileText,
  file: FileText,
  "file-text": FileText,
  film: Film,
  image: Image,
  palette: Palette,
  puzzle: Puzzle,
  sparkles: Sparkles,
  "sticky-note": StickyNote,
  terminal: Terminal,
  "square-terminal": Terminal,
  video: Video,
  wrench: Wrench,
  zap: Zap,
} as const;

const props = withDefaults(
  defineProps<{
    name?: string;
    size?: number;
  }>(),
  { size: 15 },
);

const imgFailed = ref(false);
watch(
  () => props.name,
  () => {
    imgFailed.value = false;
  },
);

const isUrl = computed(() => isPluginIconUrl(props.name ?? ""));
const resolvedSrc = computed(() => pluginAssetUrl(props.name ?? ""));
const imgStyle = computed(() => ({ width: `${props.size}px`, height: `${props.size}px` }));

const fallbackIcon = computed(() => {
  const key = (props.name ?? "").trim().toLowerCase().replace(/_/g, "-");
  return GLYPHS[key as keyof typeof GLYPHS] ?? Puzzle;
});
</script>

<style scoped>
.plugin-sidebar-icon-img {
  display: block;
  object-fit: contain;
  border-radius: 12px;
}
</style>
