<template>
  <div v-if="text.trim()" class="collapsible-text">
    <p
      ref="bodyRef"
      class="body"
      :class="{ clamped: !expanded }"
      :style="{ WebkitLineClamp: !expanded ? lines : undefined }"
    >
      {{ displayText }}
    </p>
    <button v-if="canToggle" type="button" class="toggle" @click="expanded = !expanded">
      {{ expanded ? collapseLabel : expandLabel }}
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
    text: string;
    lines?: number;
    expandLabel?: string;
    collapseLabel?: string;
  }>(),
  {
    lines: 3,
    expandLabel: "More",
    collapseLabel: "Less",
  },
);

/** Soften raw markdown walls of text for card previews. */
const displayText = computed(() =>
  props.text
    .replace(/\r\n/g, "\n")
    .replace(/^#{1,6}\s+/gm, "")
    .replace(/\*\*([^*]+)\*\*/g, "$1")
    .replace(/__([^_]+)__/g, "$1")
    .replace(/`([^`]+)`/g, "$1")
    .replace(/^\s*[-*]\s+/gm, "• ")
    .replace(/\n{3,}/g, "\n\n")
    .trim(),
);

const bodyRef = ref<HTMLElement | null>(null);
const expanded = ref(false);
const canToggle = ref(false);
let observer: ResizeObserver | null = null;

function measure() {
  const el = bodyRef.value;
  if (!el || expanded.value) return;
  const next = el.scrollHeight > el.clientHeight + 1;
  // Avoid pointless reactive writes (ResizeObserver can fire often).
  if (canToggle.value !== next) canToggle.value = next;
}

onMounted(async () => {
  await nextTick();
  measure();
  if (typeof ResizeObserver !== "undefined" && bodyRef.value) {
    observer = new ResizeObserver(() => {
      if (!expanded.value) measure();
    });
    observer.observe(bodyRef.value);
  }
});

onBeforeUnmount(() => {
  observer?.disconnect();
  observer = null;
});

watch(
  () => props.text,
  async () => {
    expanded.value = false;
    canToggle.value = false;
    await nextTick();
    measure();
  },
);
</script>

<style scoped>
.collapsible-text {
  margin-top: 12px;
}

.body {
  margin: 0;
  font-size: 13px;
  line-height: 1.5;
  color: color-mix(in srgb, var(--foreground) 72%, var(--muted-foreground));
  font-family: var(--peek-font-sans);
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}

.body.clamped {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  overflow: hidden;
  white-space: normal;
}

.toggle {
  margin-top: 4px;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--muted-foreground);
  font-size: 12px;
  cursor: pointer;
}

.toggle:hover {
  color: var(--foreground);
}
</style>
