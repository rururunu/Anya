<template>
  <button
    v-if="updaterStore.updateAvailable"
    type="button"
    class="sidebar-update-button"
    @click="emit('open')"
  >
    <ArrowUpCircle :size="17" aria-hidden="true" />
    <span class="sidebar-update-label">
      {{ tr(settingStore.language, "updater.updateAvailable") }}
    </span>
    <span class="sidebar-update-version">{{ updaterStore.latestVersion }}</span>
  </button>
</template>

<script setup lang="ts">
import { ArrowUpCircle } from "@lucide/vue";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import { useUpdaterStore } from "@/stores/updater";

const emit = defineEmits<{ open: [] }>();
const settingStore = useSettingStore();
const updaterStore = useUpdaterStore();
</script>

<style scoped>
.sidebar-update-button {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: none;
  width: calc(100% - 4px);
  min-height: 42px;
  margin: 0 2px;
  padding: 0 12px;
  border: 1px solid color-mix(in srgb, var(--peek-accent) 35%, var(--peek-border));
  border-radius: 10px;
  background: color-mix(in srgb, var(--peek-accent) 8%, transparent);
  color: var(--peek-accent);
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
}
.sidebar-update-button:hover {
  background: color-mix(in srgb, var(--peek-accent) 14%, transparent);
}
.sidebar-update-label {
  flex: 1;
}
.sidebar-update-version {
  opacity: 0.8;
  font-size: 11px;
}
</style>
