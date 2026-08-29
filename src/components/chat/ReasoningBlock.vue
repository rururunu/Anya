<template>
  <details
    v-if="showSummary"
    class="reasoning-block"
    :class="{ embedded }"
    :data-state="streaming && follow ? 'running' : 'ok'"
    :open="isOpen"
    @toggle="handleToggle"
  >
    <summary class="reasoning-summary">
      <Brain class="reasoning-icon" :size="14" :stroke-width="1.75" aria-hidden="true" />
      <span class="reasoning-title">{{ summaryLabel }}</span>
      <span v-if="inlineSummary" class="reasoning-separator" aria-hidden="true" />
      <span
        v-if="inlineSummary"
        ref="summaryRef"
        class="reasoning-inline-summary"
        :data-follow-end="streaming && follow && !isOpen ? '' : undefined"
      >
        {{ inlineSummary }}
      </span>
      <ChevronRight
        class="reasoning-chevron"
        :class="{ open: isOpen }"
        :size="12"
        aria-hidden="true"
      />
    </summary>
    <div v-if="isOpen" ref="bodyRef" class="reasoning-body peek-scrollbar">{{ displayText }}</div>
  </details>
  <div v-else ref="bodyRef" class="reasoning-block reasoning-continuation" :class="{ embedded }">
    <div class="reasoning-body peek-scrollbar">{{ displayText }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch } from "vue";
import { Brain, ChevronRight } from "@lucide/vue";
import { displayReasoningText } from "@/services/chat/reasoningDisplay";
import { firstLine, latestLine } from "@/services/chat/textLines";
import { textIncludesQuery } from "@/services/chat/conversationFind";
import { useExpandForFind } from "@/composables/chat/useConversationFind";
import type { AppLanguage } from "@/types/setting";
import { tr } from "@/services/i18n";

const props = defineProps<{
  reasoning: string;
  /**
   * True while the parent assistant turn is still live.
   * Collapsed row shows the latest line; expand for full text.
   */
  streaming?: boolean;
  /** Auto-scroll the body as new tokens arrive (usually the latest segment). */
  follow?: boolean;
  language?: AppLanguage;
  /** Nested under the agent work stream: lighter chrome, same collapse rules. */
  embedded?: boolean;
  /** Override summary label (default: think). */
  summaryKey?: "think" | "executionDetails";
  /** When false, render as a continuation chunk without a second header. */
  showSummary?: boolean;
}>();

const isOpen = ref(false);
/** After the turn finishes, honor manual expand/collapse until streaming resumes. */
const userPinned = ref(false);

const showSummary = computed(() => props.showSummary !== false);

const summaryLabel = computed(() => tr(props.language, props.summaryKey ?? "think"));

const inlineSummary = computed(() => {
  const text = props.reasoning.trim();
  if (!text) return "";
  if (props.streaming && props.follow) return latestLine(text);
  return firstLine(text);
});

const displayText = computed(() =>
  displayReasoningText(props.reasoning, {
    streaming: Boolean(props.streaming && props.follow),
  }),
);

watch(
  () => props.streaming,
  (live) => {
    if (live) {
      if (!userPinned.value) {
        isOpen.value = false;
      }
      return;
    }
    if (!userPinned.value) {
      isOpen.value = false;
    }
  },
  { immediate: true },
);

useExpandForFind(
  (query) => textIncludesQuery(props.reasoning, query),
  () => {
    isOpen.value = true;
    if (!props.streaming) userPinned.value = true;
  },
  () => props.streaming,
);

function handleToggle(event: Event) {
  const target = event.currentTarget as HTMLDetailsElement | null;
  if (!target) {
    return;
  }
  isOpen.value = target.open;
  if (!props.streaming) {
    userPinned.value = target.open;
  } else {
    userPinned.value = target.open;
  }
}

const bodyRef = ref<HTMLElement | null>(null);
const summaryRef = ref<HTMLSpanElement | null>(null);
let summaryScrollRaf = 0;

function syncSummaryScroll() {
  if (summaryScrollRaf) cancelAnimationFrame(summaryScrollRaf);
  summaryScrollRaf = requestAnimationFrame(() => {
    summaryScrollRaf = 0;
    const element = summaryRef.value;
    if (!element) return;
    element.scrollLeft =
      props.streaming && props.follow && !isOpen.value
        ? element.scrollWidth - element.clientWidth
        : 0;
  });
}

watch([inlineSummary, () => props.streaming, () => props.follow, isOpen], () => {
  syncSummaryScroll();
});

watch(
  () => props.reasoning,
  () => {
    if (props.follow && isOpen.value) {
      nextTick(() => {
        const el = bodyRef.value;
        if (el) {
          el.scrollTop = el.scrollHeight;
        }
      });
    }
    syncSummaryScroll();
  },
);

onUnmounted(() => {
  if (summaryScrollRaf) cancelAnimationFrame(summaryScrollRaf);
});
</script>

<style scoped>
.reasoning-block {
  width: 100%;
  margin-bottom: 0;
  border: 0;
  border-radius: 0;
  background: transparent;
  box-sizing: border-box;
}

.reasoning-summary {
  position: relative;
  display: flex;
  align-items: center;
  gap: 6px;
  min-height: 24px;
  overflow: hidden;
  cursor: pointer;
  padding: 0 2px;
  font-family: var(--peek-font-sans);
  font-size: var(--peek-font-sm, 12px);
  font-weight: 400;
  color: var(--peek-muted);
  list-style: none;
  user-select: none;
}

.reasoning-block[data-state="running"] .reasoning-summary::after {
  content: "";
  position: absolute;
  inset-block: 0;
  left: 0;
  width: 300px;
  background: linear-gradient(
    90deg,
    transparent 0%,
    color-mix(in srgb, var(--peek-list-bg) 60%, transparent) 55%,
    transparent 100%
  );
  animation: reasoning-row-sweep 2.6s ease-out infinite;
  pointer-events: none;
}

@keyframes reasoning-row-sweep {
  0% {
    left: -300px;
  }
  90%,
  100% {
    left: 100%;
  }
}

.reasoning-summary::-webkit-details-marker {
  display: none;
}

.reasoning-icon {
  flex: none;
  color: var(--peek-faint);
}

.reasoning-title {
  flex: none;
  color: var(--peek-text);
}

.reasoning-separator {
  flex: none;
  width: 2px;
  height: 2px;
  margin: 0 2px;
  border-radius: 50%;
  background: var(--peek-faint);
}

.reasoning-inline-summary {
  min-width: 0;
  flex: 1 1 auto;
  overflow: hidden;
  color: var(--peek-faint);
  font-size: var(--peek-font-sm, 12px);
  line-height: 24px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.reasoning-inline-summary[data-follow-end] {
  text-overflow: clip;
}

.reasoning-chevron {
  flex: none;
  margin-left: auto;
  color: var(--peek-faint);
  opacity: 0;
  transition:
    transform 140ms ease,
    opacity 120ms ease;
}

.reasoning-summary:hover .reasoning-chevron,
.reasoning-chevron.open {
  opacity: 1;
}

.reasoning-chevron.open {
  transform: rotate(90deg);
}

.reasoning-body {
  margin: 0;
  padding: 4px 0 6px 22px;
  max-height: min(40vh, 280px);
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--peek-font-sans);
  font-size: var(--peek-font-sm, 12px);
  font-weight: 400;
  line-height: 1.6;
  color: var(--peek-muted);
}

.reasoning-block.embedded .reasoning-summary {
  min-height: 24px;
  padding: 0 2px;
}

.reasoning-block.embedded .reasoning-body {
  padding: 4px 0 6px 22px;
  max-height: min(32vh, 220px);
  font-size: var(--peek-font-sm, 12px);
}

.reasoning-continuation.embedded .reasoning-body {
  max-height: none;
  padding-left: 22px;
}

@media (prefers-reduced-motion: reduce) {
  .reasoning-block[data-state="running"] .reasoning-summary::after {
    animation: none;
  }
}
</style>
