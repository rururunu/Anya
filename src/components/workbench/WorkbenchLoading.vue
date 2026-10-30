<template>
  <div class="workbench-loading" role="status" :aria-label="label">
    <div class="loading-brand" aria-hidden="true">
      <svg
        class="loading-logo"
        viewBox="152 204 208 128"
        xmlns="http://www.w3.org/2000/svg"
        focusable="false"
      >
        <g class="loading-eyes">
          <rect class="loading-eye" x="191" y="224" width="34" height="88" rx="17" />
          <rect class="loading-eye" x="287" y="224" width="34" height="88" rx="17" />
        </g>
      </svg>
    </div>
    <div class="loading-status">
      <p class="loading-label">{{ label }}</p>
      <span class="loading-progress" aria-hidden="true"><i /></span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";

const settingStore = useSettingStore();

const label = computed(() => tr(settingStore.language, "shell.loading"));
</script>

<style scoped>
.workbench-loading {
  position: fixed;
  z-index: 1000;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background:
    radial-gradient(
      120% 80% at 50% -10%,
      color-mix(in srgb, var(--peek-surface, #ffffff) 70%, transparent) 0%,
      transparent 55%
    ),
    linear-gradient(180deg, var(--peek-bg, #f8f8f8) 0%, var(--peek-sidebar, #f3f3f3) 100%);
  color: var(--peek-text, #242424);
  font-family: var(--peek-font-sans, "Noto Sans SC", "Segoe UI", sans-serif);
  user-select: none;
}

.loading-brand {
  position: relative;
  width: 224px;
  height: 148px;
  display: grid;
  place-items: center;
}

.loading-logo {
  display: block;
  width: 224px;
  height: 148px;
  overflow: visible;
  animation: logo-breathe 2.4s ease-in-out infinite;
}

.loading-eyes {
  animation: loading-scan 1.8s cubic-bezier(0.45, 0, 0.55, 1) infinite;
}

.loading-eye {
  fill: var(--peek-text, #242424);
  transform-box: fill-box;
  transform-origin: center;
  animation: loading-eye-blink 3.2s ease-in-out infinite;
}

.loading-status {
  position: absolute;
  left: 50%;
  bottom: clamp(72px, 14vh, 136px);
  transform: translateX(-50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
}

.loading-label {
  margin: 0;
  max-width: min(80vw, 360px);
  color: color-mix(in srgb, var(--peek-text, #f3f4f6) 62%, transparent);
  font-size: 14px;
  font-weight: 500;
  letter-spacing: 0.02em;
  text-align: center;
  white-space: nowrap;
}

.loading-progress {
  width: 120px;
  height: 2px;
  overflow: hidden;
  border-radius: 999px;
  background: color-mix(in srgb, var(--peek-text, #f3f4f6) 12%, transparent);
}

.loading-progress i {
  display: block;
  width: 36%;
  height: 100%;
  border-radius: inherit;
  background: var(--peek-text, #f3f4f6);
  animation: progress-travel 1.45s ease-in-out infinite;
}

@keyframes logo-breathe {
  0%,
  100% {
    transform: scale(1);
    opacity: 0.92;
  }
  50% {
    transform: scale(1.04);
    opacity: 1;
  }
}

@keyframes loading-scan {
  0%,
  100% {
    transform: translate(-18px, 0) rotate(-2deg);
  }
  50% {
    transform: translate(18px, 0) rotate(2deg);
  }
}

@keyframes loading-eye-blink {
  0%,
  42%,
  48%,
  100% {
    transform: scaleY(0.92);
  }
  45% {
    transform: scaleY(0.08) scaleX(1.08);
  }
}

@keyframes progress-travel {
  0% {
    transform: translateX(-120%);
    opacity: 0.45;
  }
  45% {
    opacity: 1;
  }
  100% {
    transform: translateX(320%);
    opacity: 0.45;
  }
}

@media (prefers-reduced-motion: reduce) {
  .loading-logo,
  .loading-eyes,
  .loading-eye,
  .loading-progress i {
    animation: none;
  }
}
</style>
