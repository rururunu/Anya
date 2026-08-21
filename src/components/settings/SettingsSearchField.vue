<template>
  <form class="settings-search" @submit.prevent="$emit('submit')">
    <Search class="search-icon" aria-hidden="true" />
    <input
      v-model="model"
      type="search"
      class="search-field"
      :placeholder="placeholder"
      :disabled="disabled"
      spellcheck="false"
      autocomplete="off"
      enterkeyhint="search"
    />
    <button
      v-if="model && !loading"
      type="button"
      class="search-action clear"
      :aria-label="clearLabel"
      @click="clear"
    >
      <X class="size-3.5" />
    </button>
    <span v-else-if="loading" class="search-action busy" aria-hidden="true">
      <Loader2 class="size-3.5 animate-spin" />
    </span>
    <button
      v-else
      type="submit"
      class="search-action go"
      :disabled="disabled"
      :aria-label="submitLabel"
    >
      <ArrowRight class="size-3.5" />
    </button>
  </form>
</template>

<script setup lang="ts">
import { ArrowRight, Loader2, Search, X } from "@lucide/vue";

withDefaults(
  defineProps<{
    placeholder: string;
    loading?: boolean;
    disabled?: boolean;
    submitLabel?: string;
    clearLabel?: string;
  }>(),
  {
    loading: false,
    disabled: false,
    submitLabel: "Search",
    clearLabel: "Clear",
  },
);

defineEmits<{
  submit: [];
}>();

const model = defineModel<string>({ required: true });

function clear() {
  model.value = "";
}
</script>

<style scoped>
.settings-search {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  min-height: 40px;
  padding: 0 12px 0 14px;
  border-radius: 12px;
  border: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
  background: color-mix(in srgb, var(--muted-foreground) 6%, var(--background));
  transition:
    border-color 0.15s ease,
    background 0.15s ease,
    box-shadow 0.15s ease;
}

.settings-search:focus-within {
  border-color: color-mix(in srgb, var(--foreground) 18%, var(--border));
  background: color-mix(in srgb, var(--muted-foreground) 3.5%, var(--background));
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--foreground) 5%, transparent);
}

.search-icon {
  width: 15px;
  height: 15px;
  flex-shrink: 0;
  color: var(--muted-foreground);
  opacity: 0.85;
}

.search-field {
  flex: 1;
  min-width: 0;
  height: 38px;
  border: 0;
  background: transparent;
  color: var(--foreground);
  font-size: 13px;
  line-height: 1.3;
  outline: none;
  box-shadow: none;
}

.search-field:focus,
.search-field:focus-visible {
  outline: none;
  box-shadow: none;
}

.search-field::placeholder {
  color: var(--muted-foreground);
  opacity: 0.78;
}

.search-field::-webkit-search-cancel-button,
.search-field::-webkit-search-decoration {
  -webkit-appearance: none;
  appearance: none;
}

.search-field:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.search-action {
  width: 28px;
  height: 28px;
  border: 0;
  border-radius: 999px;
  display: grid;
  place-items: center;
  flex-shrink: 0;
  color: var(--muted-foreground);
  background: transparent;
  cursor: pointer;
  transition:
    background 0.15s ease,
    color 0.15s ease;
}

button.search-action:hover:not(:disabled) {
  color: var(--foreground);
  background: color-mix(in srgb, var(--foreground) 7%, transparent);
}

button.search-action:disabled {
  opacity: 0.45;
  cursor: default;
}

.search-action.busy {
  cursor: default;
}

.search-action.go {
  color: var(--foreground);
  background: color-mix(in srgb, var(--foreground) 6%, transparent);
}

.search-action.go:hover:not(:disabled) {
  background: color-mix(in srgb, var(--foreground) 11%, transparent);
}
</style>
