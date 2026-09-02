<template>
  <div class="assistant-activity" role="status" aria-live="polite">
    <component v-if="icon" :is="icon" class="activity-icon" :size="14" aria-hidden="true" />
    <span v-else class="activity-mascot" aria-hidden="true">
      <MascotFace busy :follow-pointer="false" />
    </span>
    <span class="activity-label">{{ label }}</span>
  </div>
</template>

<script setup lang="ts">
import type { Component } from "vue";
import MascotFace from "@/components/icons/MascotFace.vue";

defineProps<{
  label: string;
  /** Activity-specific icon; when omitted the busy mascot stands in for a spinner. */
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

.activity-mascot {
  flex: none;
  display: block;
  width: 22px;
  height: 22px;
  animation: activity-bob 1.6s ease-in-out infinite;
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

@keyframes activity-bob {
  0%,
  100% {
    transform: translateY(0);
  }
  50% {
    transform: translateY(-2px);
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
  .activity-mascot {
    animation: none;
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
