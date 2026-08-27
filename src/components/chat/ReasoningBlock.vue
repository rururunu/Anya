<template>
  <details
    v-if="showSummary"
    class="reasoning-block"
    :class="{ embedded }"
    :open="isOpen"
    @toggle="handleToggle"
  >
    <summary class="reasoning-summary">
      <ChevronRight class="reasoning-chevron" :class="{ open: isOpen }" :size="12" />
      <span>{{ summaryLabel }}</span>
      <span v-if="!isOpen" class="reasoning-meta">{{ collapsedHint }}</span>
    </summary>
    <div v-if="isOpen" ref="bodyRef" class="reasoning-body peek-scrollbar">{{ displayText }}</div>
  </details>
  <div v-else ref="bodyRef" class="reasoning-block reasoning-continuation" :class="{ embedded }">
    <div class="reasoning-body peek-scrollbar">{{ displayText }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { ChevronRight } from "@lucide/vue";
import { displayReasoningText } from "@/services/chat/reasoningDisplay";
import { textIncludesQuery } from "@/services/chat/conversationFind";
import { useExpandForFind } from "@/composables/chat/useConversationFind";
import type { AppLanguage } from "@/types/setting";
import { tr } from "@/services/i18n";

const props = defineProps<{
  reasoning: string;
  /**
   * True while the parent assistant turn is still live.
   * Keeps this block open for follow-along; only collapses after the turn ends
   * (unless the user pinned it open).
   */
  streaming?: boolean;
  /** Auto-scroll the body as new tokens arrive (usually the latest segment). */
  follow?: boolean;
  language?: AppLanguage;
  /** Nested under the agent work stream: lighter chrome, same collapse rules. */
  embedded?: boolean;
  /** Override summary label (default: thinking process). */
  summaryKey?: "thinkingProcess" | "executionDetails";
  /** When false, render as a continuation chunk without a second header. */
  showSummary?: boolean;
}>();

const isOpen = ref(false);
/** After the turn finishes, honor manual expand/collapse until streaming resumes. */
const userPinned = ref(false);

const showSummary = computed(() => props.showSummary !== false);

const summaryLabel = computed(() => tr(props.language, props.summaryKey ?? "thinkingProcess"));

const collapsedHint = computed(() => {
  const chars = props.reasoning.length;
  return tr(props.language, "chars", { count: chars.toLocaleString() });
});

const displayText = computed(() =>
  displayReasoningText(props.reasoning, {
    // Only truncate the actively followed segment so older chunks stay intact.
    streaming: Boolean(props.streaming && props.follow),
  }),
);

watch(
  () => props.streaming,
  (live) => {
    if (live) {
      userPinned.value = false;
      isOpen.value = true;
      return;
    }
    // Collapse only when the whole turn finishes — never mid-turn just because
    // another tool/content segment became the "latest".
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
  }
}

const bodyRef = ref<HTMLElement | null>(null);

watch(
  () => props.reasoning,
  () => {
    if (props.follow && (isOpen.value || !showSummary.value)) {
      nextTick(() => {
        const el = bodyRef.value;
        if (el) {
          el.scrollTop = el.scrollHeight;
        }
      });
    }
  },
);
</script>

<style scoped>
.reasoning-block {
  width: 100%;
  margin-bottom: 0;
  border: 1px solid color-mix(in srgb, var(--peek-border) 88%, var(--peek-muted));
  border-radius: 8px;
  background: var(--peek-surface);
  isolation: isolate;
  box-sizing: border-box;
}

.reasoning-summary {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  padding: 6px 10px;
  font-family: var(--peek-font-sans);
  font-size: 12px;
  font-weight: 600;
  color: var(--peek-muted);
  list-style: none;
  user-select: none;
}

.reasoning-summary::-webkit-details-marker {
  display: none;
}

.reasoning-chevron {
  flex: none;
  color: var(--peek-faint);
  transition: transform 160ms ease;
}

.reasoning-chevron.open {
  transform: rotate(90deg);
}

.reasoning-meta {
  margin-left: auto;
  font-weight: 500;
  font-size: 11px;
  color: var(--peek-faint);
}

.reasoning-body {
  margin: 0;
  padding: 4px 12px 10px;
  max-height: min(40vh, 280px);
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--peek-font-sans);
  font-size: 13px;
  font-weight: 400;
  line-height: 1.65;
  letter-spacing: 0.01em;
  color: var(--peek-text);
  -webkit-font-smoothing: subpixel-antialiased;
  transform: translateZ(0);
}

/* Nested under process panel: no second card chrome */
.reasoning-block.embedded {
  margin-bottom: 0;
  border: 0;
  border-radius: 0;
  background: transparent;
  isolation: auto;
}

.reasoning-block.embedded .reasoning-summary {
  padding: 3px 2px;
  font-size: 11px;
  font-weight: 550;
}

.reasoning-block.embedded .reasoning-body {
  padding: 2px 2px 6px;
  max-height: min(32vh, 220px);
  font-size: 12px;
  line-height: 1.55;
  color: var(--peek-muted);
}

.reasoning-continuation.embedded .reasoning-body {
  max-height: none;
}
</style>
