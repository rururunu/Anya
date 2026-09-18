<template>
  <main class="cu-hud" aria-hidden="true">
    <div class="cu-glow" />
  </main>
</template>

<script setup lang="ts">
import { onMounted } from "vue";

onMounted(() => {
  if (typeof document !== "undefined") {
    document.documentElement.style.removeProperty("zoom");
    document.documentElement.style.setProperty("--ui-zoom", "1");
    document.documentElement.classList.add("peek-window");
  }
});
</script>

<style scoped>
.cu-hud {
  position: fixed;
  inset: 0;
  margin: 0;
  overflow: hidden;
  background: transparent;
  pointer-events: none;
  user-select: none;
}

.cu-glow {
  position: absolute;
  inset: 0;
  box-shadow:
    inset 0 0 40px 8px color-mix(in srgb, var(--peek-accent) 38%, transparent),
    inset 0 0 96px 24px color-mix(in srgb, var(--peek-accent) 18%, transparent),
    inset 0 0 160px 48px color-mix(in srgb, var(--peek-accent) 8%, transparent);
  animation: cu-glow-breathe 2.8s ease-in-out infinite;
}

@keyframes cu-glow-breathe {
  0%,
  100% {
    opacity: 0.78;
  }
  50% {
    opacity: 1;
  }
}

@media (prefers-reduced-motion: reduce) {
  .cu-glow {
    animation: none;
  }
}
</style>
