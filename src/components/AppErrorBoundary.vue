<script setup lang="ts">
import { computed, onErrorCaptured, ref } from "vue";
import { createLogger } from "@/services/logger";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";

const log = createLogger("error-boundary");
const settingStore = useSettingStore();

const props = withDefaults(
  defineProps<{
    /** Compact overlay-friendly fallback. */
    compact?: boolean;
  }>(),
  { compact: false },
);

const errorMessage = ref<string | null>(null);
const errorStack = ref<string | null>(null);
const detailsOpen = ref(false);
const generation = ref(0);
const isDev = import.meta.env.DEV;

const title = computed(() => tr(settingStore.language, "error.boundaryTitle"));
const hint = computed(() =>
  tr(settingStore.language, props.compact ? "error.boundaryHintCompact" : "error.boundaryHint"),
);
const retryLabel = computed(() => tr(settingStore.language, "error.boundaryRetry"));
const detailsLabel = computed(() => tr(settingStore.language, "error.boundaryDetails"));

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
    return { message: Object.prototype.toString.call(err), stack: null };
  }
  return { message: String(err), stack: null };
}

onErrorCaptured((err, _instance, info) => {
  const described = describeError(err);
  errorMessage.value = described.message;
  errorStack.value = described.stack;
  detailsOpen.value = false;
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
  return true;
});

function retry() {
  errorMessage.value = null;
  errorStack.value = null;
  detailsOpen.value = false;
  generation.value += 1;
}
</script>

<template>
  <div
    class="error-boundary-root"
    :class="{ compact: props.compact, failed: Boolean(errorMessage) }"
  >
    <div v-if="errorMessage" class="app-error-boundary" :class="{ compact: props.compact }">
      <p class="title">{{ title }}</p>
      <p class="hint">{{ hint }}</p>
      <button type="button" class="retry" @click="retry">{{ retryLabel }}</button>
      <button
        v-if="isDev || errorStack"
        type="button"
        class="details-toggle"
        @click="detailsOpen = !detailsOpen"
      >
        {{ detailsLabel }}
      </button>
      <pre v-if="detailsOpen" class="stack">{{ errorStack || errorMessage }}</pre>
    </div>
    <div v-else :key="generation" class="error-boundary-slot">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.error-boundary-root:not(.failed) {
  display: contents;
}
.error-boundary-root.failed {
  width: 100%;
  height: 100%;
  min-height: 0;
}
.error-boundary-root.failed.compact {
  flex: 1;
  min-width: 0;
}

.error-boundary-slot {
  display: contents;
}

.app-error-boundary {
  display: flex;
  flex-direction: column;
  gap: 10px;
  align-items: flex-start;
  justify-content: center;
  width: 100%;
  height: 100%;
  padding: 24px;
  color: var(--peek-text, #242424);
  background: var(--peek-bg, #f8f8f8);
  box-sizing: border-box;
}

.app-error-boundary.compact {
  padding: 20px 24px;
  min-height: 8rem;
  background: transparent;
}

.title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.hint {
  margin: 0;
  max-width: 36em;
  font-size: 13px;
  line-height: 1.5;
  opacity: 0.78;
}

.stack {
  margin: 0;
  max-width: 100%;
  max-height: 12rem;
  overflow: auto;
  padding: 12px;
  font-size: 11px;
  line-height: 1.4;
  white-space: pre-wrap;
  background: color-mix(in srgb, var(--peek-text, #fff) 6%, transparent);
  border-radius: 8px;
}

.retry,
.details-toggle {
  appearance: none;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
}

.retry {
  margin-top: 4px;
  padding: 7px 14px;
  border: 1px solid color-mix(in srgb, var(--peek-text, #fff) 16%, transparent);
  background: color-mix(in srgb, var(--peek-text, #fff) 8%, transparent);
  color: inherit;
}

.retry:hover {
  background: color-mix(in srgb, var(--peek-text, #fff) 14%, transparent);
}

.details-toggle {
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--peek-muted, #9a9a9a);
  font-size: 12px;
}

.details-toggle:hover {
  color: inherit;
}
</style>
