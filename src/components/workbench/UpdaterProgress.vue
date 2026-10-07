<template>
  <div v-if="visible" class="update-progress">
    <div class="update-progress-label">
      <span>{{ phaseLabel }}</span>
      <span v-if="hasTotal || downloadComplete">
        {{ percent }}%
        <template v-if="hasTotal">
          · {{ formatBytes(updaterStore.progress.downloadedBytes) }} /
          {{ formatBytes(updaterStore.progress.totalBytes) }}
        </template>
      </span>
      <span v-else-if="updaterStore.progress.downloadedBytes">
        {{ formatBytes(updaterStore.progress.downloadedBytes) }}
      </span>
    </div>
    <div
      class="update-progress-track"
      role="progressbar"
      :aria-label="phaseLabel"
      :aria-valuenow="hasTotal || downloadComplete ? percent : undefined"
      aria-valuemin="0"
      aria-valuemax="100"
    >
      <div
        class="update-progress-fill"
        :class="{ indeterminate: !hasTotal && !downloadComplete }"
        :style="hasTotal || downloadComplete ? { width: `${percent}%` } : undefined"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import { useUpdaterStore } from "@/stores/updater";

const settingStore = useSettingStore();
const updaterStore = useUpdaterStore();
const visible = computed(() =>
  ["downloading", "installing", "relaunching"].includes(updaterStore.progress.phase),
);
const hasTotal = computed(() => updaterStore.progress.totalBytes > 0);
const downloadComplete = computed(() =>
  ["installing", "relaunching"].includes(updaterStore.progress.phase),
);
const percent = computed(() =>
  downloadComplete.value
    ? 100
    : hasTotal.value
      ? Math.min(
          100,
          Math.round(
            (updaterStore.progress.downloadedBytes / updaterStore.progress.totalBytes) * 100,
          ),
        )
      : 0,
);
const phaseLabel = computed(() => {
  if (updaterStore.progress.phase === "relaunching")
    return tr(settingStore.language, "updater.relaunching");
  if (updaterStore.progress.phase === "installing")
    return tr(settingStore.language, "updater.installing");
  return tr(settingStore.language, "updater.downloading");
});

function formatBytes(bytes: number) {
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
</script>

<style scoped>
.update-progress {
  display: grid;
  gap: 8px;
  min-width: 0;
}
.update-progress-label {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  color: var(--peek-muted);
  font-size: 12px;
}
.update-progress-label > span:last-child {
  text-align: right;
}
.update-progress-track {
  height: 7px;
  overflow: hidden;
  border-radius: 999px;
  background: color-mix(in srgb, var(--peek-text) 12%, transparent);
}
.update-progress-fill {
  height: 100%;
  border-radius: inherit;
  background: var(--peek-accent);
  transition: width 180ms ease;
}
.update-progress-fill.indeterminate {
  width: 35%;
  animation: update-loading 1.3s ease-in-out infinite;
}
@keyframes update-loading {
  from {
    transform: translateX(-100%);
  }
  to {
    transform: translateX(300%);
  }
}
</style>
