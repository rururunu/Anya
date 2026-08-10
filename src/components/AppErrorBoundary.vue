<script setup lang="ts">
import { onErrorCaptured, ref } from "vue";
import { createLogger } from "@/services/logger";

const log = createLogger("error-boundary");

const props = withDefaults(
  defineProps<{
    /** Compact overlay-friendly fallback. */
    compact?: boolean;
  }>(),
  { compact: false },
);

const errorMessage = ref<string | null>(null);
const errorStack = ref<string | null>(null);

function describeError(err: unknown): { message: string; stack: string | null } {
  if (err instanceof Error) {
    return { message: err.message || err.name, stack: err.stack ?? null };
  }
  if (typeof err === "string") {
    return { message: err, stack: null };
  }
  if (err && typeof err === "object") {
    const record = err as { message?: unknown; stack?: unknown; name?: unknown };
    if (typeof record.message === "string" && record.message.trim()) {
      return {
        message: record.message,
        stack: typeof record.stack === "string" ? record.stack : null,
      };
    }
    if (typeof record.name === "string" && record.name.trim()) {
      return { message: record.name, stack: null };
    }
    // Never JSON.stringify arbitrary objects (Vue proxies can hang the UI).
    return { message: Object.prototype.toString.call(err), stack: null };
  }
  return { message: String(err), stack: null };
}

onErrorCaptured((err, _instance, info) => {
  const described = describeError(err);
  errorMessage.value = described.message;
  errorStack.value = described.stack;
  log.error("captured", {
    message: described.message,
    info,
    stack: described.stack,
  });
  try {
    sessionStorage.setItem(
      "anya.lastUiError",
      JSON.stringify({
        message: described.message,
        info,
        stack: described.stack,
        at: Date.now(),
      }),
    );
  } catch {
    // ignore quota / private mode
  }
  // Stop propagation — the fallback UI already handled this error.
  return true;
});

function retry() {
  errorMessage.value = null;
  errorStack.value = null;
}
</script>

<template>
  <div v-if="errorMessage" class="app-error-boundary" :class="{ compact: props.compact }">
    <p class="title">Something went wrong</p>
    <p class="message">{{ errorMessage }}</p>
    <!-- Always show stack: overlay/prod builds otherwise hide the only clue. -->
    <pre v-if="errorStack" class="stack">{{ errorStack }}</pre>
    <button type="button" class="retry" @click="retry">Retry</button>
  </div>
  <slot v-else />
</template>

<style scoped>
.app-error-boundary {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  align-items: flex-start;
  justify-content: center;
  width: 100%;
  height: 100%;
  padding: 1.5rem;
  color: #e8e8e8;
  background: #1f1f1f;
  box-sizing: border-box;
}

.app-error-boundary.compact {
  padding: 1rem;
  min-height: 8rem;
}

.title {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
}

.message {
  margin: 0;
  font-size: 0.875rem;
  opacity: 0.85;
  word-break: break-word;
}

.stack {
  margin: 0;
  max-width: 100%;
  max-height: 12rem;
  overflow: auto;
  padding: 0.75rem;
  font-size: 0.7rem;
  line-height: 1.4;
  white-space: pre-wrap;
  background: #151515;
  border-radius: 0.375rem;
}

.retry {
  appearance: none;
  border: 1px solid #4a4a4a;
  background: #2a2a2a;
  color: inherit;
  border-radius: 0.375rem;
  padding: 0.4rem 0.85rem;
  font-size: 0.8125rem;
  cursor: pointer;
}

.retry:hover {
  background: #333;
}
</style>
