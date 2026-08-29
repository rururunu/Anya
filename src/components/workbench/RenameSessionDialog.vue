<template>
  <DialogRoot :open="open" @update:open="handleOpenChange">
    <DialogPortal>
      <DialogOverlay class="session-rename-overlay" />
      <DialogContent
        class="session-rename-dialog"
        :aria-describedby="undefined"
        @open-auto-focus.prevent="focusInput"
      >
        <DialogTitle class="session-rename-title">{{ copy.title }}</DialogTitle>
        <form ref="formRef" class="session-rename-form" @submit.prevent="save">
          <label class="session-rename-field">
            <span>{{ copy.label }}</span>
            <Input v-model="title" :placeholder="copy.placeholder" maxlength="80" />
          </label>
          <p v-if="error" class="session-rename-error">{{ error }}</p>
          <div class="session-rename-actions">
            <button type="button" class="session-rename-button ghost" @click="close">
              {{ copy.cancel }}
            </button>
            <button type="submit" class="session-rename-button primary" :disabled="saving">
              {{ copy.save }}
            </button>
          </div>
        </form>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>

<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { DialogContent, DialogOverlay, DialogPortal, DialogRoot, DialogTitle } from "reka-ui";
import { Input } from "@/components/ui/input";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import { setChatSessionTitle } from "@/services/ipc";

const settingStore = useSettingStore();
const open = ref(false);
const saving = ref(false);
const error = ref("");
const sessionId = ref("");
const title = ref("");
const formRef = ref<HTMLFormElement | null>(null);
let resolver: ((saved: boolean) => void) | null = null;

const copy = computed(() => {
  const language = settingStore.language;
  return {
    title: tr(language, "session.renameTitle"),
    label: tr(language, "session.renameLabel"),
    placeholder: tr(language, "session.renamePlaceholder"),
    cancel: tr(language, "workspace.cancel"),
    save: tr(language, "workspace.save"),
  };
});

function focusInput() {
  void nextTick(() => {
    formRef.value?.querySelector("input")?.focus();
  });
}

function edit(id: string, currentTitle: string) {
  resolver?.(false);
  sessionId.value = id;
  title.value = currentTitle;
  error.value = "";
  saving.value = false;
  open.value = true;
  return new Promise<boolean>((resolve) => {
    resolver = resolve;
  });
}

function settle(saved: boolean) {
  open.value = false;
  saving.value = false;
  resolver?.(saved);
  resolver = null;
}

function close() {
  if (saving.value) return;
  settle(false);
}

function handleOpenChange(nextOpen: boolean) {
  if (!nextOpen && open.value) close();
}

async function save() {
  if (saving.value || !sessionId.value) return;
  saving.value = true;
  error.value = "";
  try {
    await setChatSessionTitle(sessionId.value, title.value);
    settle(true);
  } catch (cause) {
    error.value = String(cause);
    saving.value = false;
  }
}

defineExpose({ edit });
</script>

<style scoped>
.session-rename-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  background: color-mix(in srgb, #000 48%, transparent);
  backdrop-filter: blur(2px);
}

.session-rename-dialog {
  position: fixed;
  top: 50%;
  left: 50%;
  z-index: 51;
  box-sizing: border-box;
  width: min(400px, calc(100vw - 32px));
  padding: 16px;
  border: 1px solid var(--peek-border, rgba(255, 255, 255, 0.14));
  border-radius: var(--peek-radius-lg, 12px);
  background: var(--peek-dialog-bg, var(--peek-surface, #ffffff));
  color: var(--peek-text, #f3f4f6);
  box-shadow: var(--peek-elev-md);
  transform: translate(-50%, -50%);
  outline: none;
}

.session-rename-title {
  margin: 0 0 14px;
  font-size: var(--peek-font-lg, 14px);
  font-weight: 650;
}

.session-rename-form {
  display: grid;
  gap: 10px;
}

.session-rename-field {
  display: grid;
  gap: 5px;
  color: var(--peek-muted, #b7bcc5);
  font-size: 11px;
  font-weight: 600;
}

.session-rename-error {
  margin: 0;
  color: var(--destructive, var(--peek-danger));
  font-size: 11px;
}

.session-rename-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 6px;
}

.session-rename-button {
  min-width: 72px;
  height: var(--peek-control-row, 30px);
  padding: 0 12px;
  border: 1px solid transparent;
  border-radius: var(--peek-radius-sm, 6px);
  font: inherit;
  font-size: var(--peek-font-sm, 12px);
  font-weight: 550;
  cursor: pointer;
}

.session-rename-button.ghost {
  border-color: var(--peek-border, rgba(255, 255, 255, 0.14));
  background: color-mix(in srgb, var(--peek-text, #f3f4f6) 4%, transparent);
  color: var(--peek-text, #f3f4f6);
}

.session-rename-button.primary {
  background: var(--peek-send-active-bg, var(--peek-accent));
  color: var(--peek-send-active-fg, var(--peek-primary-foreground));
}

.session-rename-button.primary:disabled {
  cursor: default;
  opacity: 0.6;
}
</style>
