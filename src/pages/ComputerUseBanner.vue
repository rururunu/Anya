<template>
  <main class="cu-banner-root">
    <div class="cu-card">
      <span class="cu-status">
        <span class="cu-pulse" aria-hidden="true" />
        <span class="cu-text">{{ label }}</span>
      </span>
      <button type="button" class="cu-stop" :disabled="stopping" @click="onStop">
        <span class="cu-stop-glyph" aria-hidden="true" />
        <span class="cu-stop-label">{{ stopLabel }}</span>
      </button>
    </div>
  </main>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useSettingStore } from "@/stores/setting";
import { dismissComputerUseHud } from "@/services/ipc/commands";

const settingStore = useSettingStore();
const stopping = ref(false);

const label = computed(() =>
  settingStore.language === "zh-CN" ? "Anya 正在操控电脑" : "Anya is controlling your computer",
);

const stopLabel = computed(() => (settingStore.language === "zh-CN" ? "结束" : "Stop"));

onMounted(() => {
  if (typeof document !== "undefined") {
    document.documentElement.style.removeProperty("zoom");
    document.documentElement.style.setProperty("--ui-zoom", "1");
    document.documentElement.classList.add("peek-window");
  }
});

async function onStop() {
  if (stopping.value) return;
  stopping.value = true;
  try {
    await dismissComputerUseHud();
  } catch (error) {
    console.error("dismiss_computer_use_hud failed:", error);
    stopping.value = false;
  }
}
</script>

<style scoped>
.cu-banner-root {
  position: fixed;
  inset: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 6px 10px;
  box-sizing: border-box;
  background: transparent;
  user-select: none;
}

.cu-card {
  display: inline-flex;
  align-items: center;
  gap: 14px;
  max-width: 100%;
  height: 44px;
  padding: 0 6px 0 16px;
  border: 1px solid color-mix(in srgb, var(--peek-accent) 28%, var(--peek-border, #d4d4d8));
  border-radius: 999px;
  background: color-mix(in srgb, var(--peek-surface, #fff) 94%, transparent);
  box-shadow:
    0 8px 24px color-mix(in srgb, #000 18%, transparent),
    0 0 0 1px color-mix(in srgb, #fff 40%, transparent) inset;
  backdrop-filter: blur(16px) saturate(1.2);
  -webkit-backdrop-filter: blur(16px) saturate(1.2);
  color: var(--peek-text, #18181b);
  box-sizing: border-box;
}

.cu-status {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.cu-pulse {
  width: 8px;
  height: 8px;
  flex: none;
  border-radius: 50%;
  background: var(--peek-accent, #2563eb);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--peek-accent, #2563eb) 18%, transparent);
  animation: cu-pulse 1.6s ease-out infinite;
}

.cu-text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  font-size: 14px;
  font-weight: 600;
  letter-spacing: 0.01em;
  line-height: 1;
}

.cu-stop {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  flex: none;
  height: 32px;
  margin: 0;
  padding: 0 14px 0 12px;
  border: 1px solid color-mix(in srgb, #ef4444 28%, transparent);
  border-radius: 999px;
  background: color-mix(in srgb, #ef4444 12%, transparent);
  color: #dc2626;
  font: inherit;
  cursor: pointer;
  box-sizing: border-box;
}

.cu-stop-glyph {
  flex: none;
  width: 9px;
  height: 9px;
  border-radius: 2px;
  background: currentColor;
}

.cu-stop-label {
  display: block;
  font-size: 13px;
  font-weight: 650;
  line-height: 1;
  letter-spacing: 0.02em;
}

.cu-stop:hover:not(:disabled) {
  background: color-mix(in srgb, #ef4444 20%, transparent);
  border-color: color-mix(in srgb, #ef4444 42%, transparent);
  color: #b91c1c;
}

.cu-stop:disabled {
  opacity: 0.55;
  cursor: default;
}

@keyframes cu-pulse {
  0% {
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--peek-accent, #2563eb) 22%, transparent);
  }
  70% {
    box-shadow: 0 0 0 10px transparent;
  }
  100% {
    box-shadow: 0 0 0 3px transparent;
  }
}

@media (prefers-reduced-motion: reduce) {
  .cu-pulse {
    animation: none;
  }
}
</style>
