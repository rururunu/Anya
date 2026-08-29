<template>
  <AppErrorBoundary>
    <RouterView v-slot="{ Component }">
      <Suspense>
        <component :is="Component" class="h-full w-full" />
        <template #fallback>
          <!-- Workbench: solid fill under HTML splash. Peek/overlay: transparent —
               never paint the workbench boot logo into an 84px Alt+Alt window. -->
          <div v-if="isWorkbench" class="route-boot-fill" aria-hidden="true" />
          <div v-else-if="isPeek" class="route-peek-fill" aria-hidden="true" />
          <div v-else class="route-loading" />
        </template>
      </Suspense>
    </RouterView>
  </AppErrorBoundary>
  <AppTooltipLayer />
</template>

<script setup lang="ts">
import { RouterView } from "vue-router";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import AppErrorBoundary from "@/components/AppErrorBoundary.vue";
import AppTooltipLayer from "@/components/ui/AppTooltipLayer.vue";

const windowLabel = getCurrentWebviewWindow().label;
const isWorkbench = windowLabel === "workbench";
const isPeek =
  windowLabel === "overlay" ||
  windowLabel.startsWith("overlay-") ||
  windowLabel.startsWith("overlay-preview-");
</script>

<style scoped>
.route-loading,
.route-boot-fill {
  width: 100%;
  height: 100%;
  background: var(--peek-bg, #f8f8f8);
}

.route-peek-fill {
  width: 100%;
  height: 100%;
  background: transparent;
}
</style>
