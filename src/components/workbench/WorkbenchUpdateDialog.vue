<template>
  <DialogRoot :open="visible" @update:open="handleOpenChange">
    <DialogPortal>
      <DialogOverlay class="update-dialog-overlay" />
      <DialogContent class="update-dialog" :aria-describedby="undefined">
        <div class="update-dialog-heading">
          <span class="update-dialog-icon"><ArrowUpCircle :size="19" /></span>
          <div>
            <DialogTitle class="update-dialog-title">{{ copy.title }}</DialogTitle>
            <p class="update-dialog-description">{{ copy.description }}</p>
          </div>
        </div>

        <section class="update-notes" :aria-label="copy.notesTitle">
          <h3>{{ copy.notesTitle }}</h3>
          <p>{{ updaterStore.releaseNotes.trim() || copy.noNotes }}</p>
        </section>

        <p v-if="updaterStore.errorMessage" class="update-error" role="alert">
          {{ copy.error }}: {{ updaterStore.errorMessage }}
        </p>

        <UpdaterProgress />

        <div class="update-dialog-actions">
          <button type="button" class="update-dialog-button secondary" @click="visible = false">
            {{ downloading ? copy.close : copy.cancel }}
          </button>
          <button
            v-if="!downloading"
            type="button"
            class="update-dialog-button primary"
            :disabled="!updaterStore.updateAvailable"
            @click="startUpdate"
          >
            {{ copy.confirm }}
          </button>
        </div>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { ArrowUpCircle } from "@lucide/vue";
import { DialogContent, DialogOverlay, DialogPortal, DialogRoot, DialogTitle } from "reka-ui";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import { useUpdaterStore } from "@/stores/updater";
import UpdaterProgress from "./UpdaterProgress.vue";

const settingStore = useSettingStore();
const updaterStore = useUpdaterStore();
const visible = ref(false);
const downloading = computed(
  () =>
    updaterStore.status === "downloading" ||
    updaterStore.progress.phase === "installing" ||
    updaterStore.progress.phase === "relaunching",
);
const copy = computed(() => {
  const language = settingStore.language;
  return {
    title: tr(language, "updater.confirmTitle", { version: updaterStore.latestVersion || "?" }),
    description: tr(language, "updater.confirmDescription"),
    notesTitle: tr(language, "updater.releaseNotes"),
    noNotes: tr(language, "updater.noReleaseNotes"),
    confirm: tr(language, "updater.confirmAction"),
    cancel: tr(language, "updater.cancelAction"),
    close: tr(language, "updater.closeAction"),
    error: tr(language, "updater.error"),
  };
});
function open() {
  visible.value = true;
}
function handleOpenChange(open: boolean) {
  visible.value = open;
}
function startUpdate() {
  void updaterStore.install();
}
defineExpose({ open });
</script>

<style scoped>
.update-dialog-overlay {
  position: fixed;
  inset: 0;
  z-index: 70;
  background: color-mix(in srgb, #000 45%, transparent);
  backdrop-filter: blur(2px);
}
.update-dialog {
  position: fixed;
  top: 50%;
  left: 50%;
  z-index: 71;
  box-sizing: border-box;
  width: min(520px, calc(100vw - 32px));
  max-height: min(620px, calc(100vh - 32px));
  padding: 22px;
  border: 1px solid var(--peek-border);
  border-radius: 14px;
  background: var(--peek-dialog-bg, var(--peek-surface));
  color: var(--peek-text);
  box-shadow: var(--peek-elev-md);
  transform: translate(-50%, -50%);
  outline: none;
  display: flex;
  flex-direction: column;
  gap: 18px;
}
.update-dialog-heading {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}
.update-dialog-icon {
  color: var(--peek-accent);
  padding-top: 2px;
}
.update-dialog-title {
  margin: 0;
  font-size: 17px;
  font-weight: 650;
}
.update-dialog-description {
  margin: 7px 0 0;
  color: var(--peek-muted);
  font-size: 13px;
  line-height: 1.55;
}
.update-notes {
  min-height: 0;
  overflow: auto;
  padding: 14px;
  border: 1px solid var(--peek-border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--peek-text) 3%, transparent);
}
.update-notes h3 {
  margin: 0 0 8px;
  font-size: 12px;
  font-weight: 650;
}
.update-notes p {
  margin: 0;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  color: var(--peek-muted);
  font-size: 12px;
  line-height: 1.6;
}
.update-error {
  margin: 0;
  color: var(--peek-danger);
  font-size: 12px;
  overflow-wrap: anywhere;
}
.update-dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.update-dialog-button {
  min-height: 34px;
  padding: 0 14px;
  border: 1px solid var(--peek-border);
  border-radius: 8px;
  background: transparent;
  color: var(--peek-text);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
.update-dialog-button.primary {
  border-color: var(--peek-accent);
  background: var(--peek-accent);
  color: var(--peek-primary-foreground, white);
}
.update-dialog-button:disabled {
  opacity: 0.55;
  cursor: default;
}
</style>
