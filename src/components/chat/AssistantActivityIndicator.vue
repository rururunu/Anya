<template>
  <div class="assistant-activity" role="status" aria-live="polite">
    <component
      :is="icon ?? LoaderCircle"
      class="activity-icon"
      :class="{ spin: !icon }"
      :size="14"
      aria-hidden="true"
    />
    <span class="activity-label">{{ label }}</span>
  </div>
</template>

<script setup lang="ts">
import type { Component } from "vue";
import { LoaderCircle } from "@lucide/vue";

defineProps<{
  label: string;
  icon?: Component;
}>();
</script>

<style scoped>
.assistant-activity {
  display: inline-flex;
  align-items: center;
  align-self: flex-start;
  gap: 8px;
  min-height: 24px;
  color: var(--peek-muted);
  font-size: var(--peek-font-sm, 12px);
  line-height: 24px;
}

.activity-icon {
  flex: none;
  color: var(--peek-accent);
}

.activity-icon.spin {
  animation: activity-spin 1.1s linear infinite;
}

.activity-label {
  background: linear-gradient(
    90deg,
    var(--peek-muted) 0%,
    color-mix(in srgb, var(--peek-text) 72%, var(--peek-muted)) 45%,
    var(--peek-muted) 90%
  );
  background-size: 200% 100%;
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
  animation: activity-shimmer 1.8s linear infinite;
}

@keyframes activity-spin {
  to {
    transform: rotate(360deg);
  }
}

@keyframes activity-shimmer {
  0% {
    background-position: 100% 0;
  }
  100% {
    background-position: -100% 0;
  }
}

@media (prefers-reduced-motion: reduce) {
  .activity-icon.spin {
    animation-duration: 2.4s;
  }
  .activity-label {
    animation: none;
    color: var(--peek-muted);
    background: none;
    -webkit-background-clip: unset;
    background-clip: unset;
  }
}
</style>
