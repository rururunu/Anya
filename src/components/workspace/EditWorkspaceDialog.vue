<template>
  <DialogRoot :open="open" @update:open="handleOpenChange">
    <DialogPortal>
      <DialogOverlay class="workspace-edit-overlay" />
      <DialogContent
        class="workspace-edit-dialog"
        :aria-describedby="undefined"
        @open-auto-focus.prevent="focusName"
      >
        <DialogTitle class="workspace-edit-title">{{ copy.title }}</DialogTitle>
        <form ref="formRef" class="workspace-edit-form" @submit.prevent="save">
          <label class="workspace-edit-field">
            <span>{{ copy.name }}</span>
            <Input v-model="name" :placeholder="copy.namePlaceholder" maxlength="120" />
          </label>
          <label class="workspace-edit-field">
            <span>{{ copy.description }}</span>
            <textarea
              v-model="description"
              class="workspace-edit-notes peek-scrollbar"
              rows="3"
              maxlength="400"
              :placeholder="copy.descriptionPlaceholder"
            />
          </label>
          <p class="workspace-edit-path">
            <span>{{ copy.path }}</span>
            <code :title="path">{{ path }}</code>
          </p>
          <p v-if="error" class="workspace-edit-error">{{ error }}</p>
          <div class="workspace-edit-actions">
            <button type="button" class="workspace-edit-button ghost" @click="close">
              {{ copy.cancel }}
            </button>
            <button type="submit" class="workspace-edit-button primary" :disabled="saving">
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
import { updateWorkspace, type Workspace } from "@/commands/workspace";

const settingStore = useSettingStore();
const open = ref(false);
const saving = ref(false);
const error = ref("");
const workspaceId = ref("");
const name = ref("");
const description = ref("");
const path = ref("");
const formRef = ref<HTMLFormElement | null>(null);
let resolver: ((saved: boolean) => void) | null = null;

const copy = computed(() => {
  const language = settingStore.language;
  return {
    title: tr(language, "workspace.editTitle"),
    name: tr(language, "workspace.name"),
    namePlaceholder: tr(language, "workspace.namePlaceholder"),
    description: tr(language, "workspace.descriptionLabel"),
    descriptionPlaceholder: tr(language, "workspace.descriptionPlaceholder"),
    path: tr(language, "workspace.path"),
    cancel: tr(language, "workspace.cancel"),
    save: tr(language, "workspace.save"),
  };
});

function focusName() {
  void nextTick(() => {
    formRef.value?.querySelector("input")?.focus();
  });
}

function edit(workspace: Workspace) {
  resolver?.(false);
  workspaceId.value = workspace.id;
  name.value = workspace.name;
  description.value = workspace.description ?? "";
  path.value = workspace.root;
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
  if (saving.value || !workspaceId.value) return;
  saving.value = true;
  error.value = "";
  try {
    await updateWorkspace(workspaceId.value, name.value, description.value);
    settle(true);
  } catch (cause) {
    error.value = String(cause);
    saving.value = false;
  }
}

defineExpose({ edit });
</script>

<style>
.workspace-edit-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  background: color-mix(in srgb, #000 48%, transparent);
  backdrop-filter: blur(2px);
}

.workspace-edit-dialog {
  position: fixed;
  top: 50%;
  left: 50%;
  z-index: 51;
  box-sizing: border-box;
  width: min(400px, calc(100vw - 32px));
  padding: 16px;
  border: 1px solid var(--peek-border, rgba(255, 255, 255, 0.14));
  border-radius: var(--peek-radius-lg, 12px);
  background: var(--peek-dialog-bg, var(--peek-surface, #252526));
  color: var(--peek-text, #f3f4f6);
  box-shadow: var(--peek-elev-md);
  transform: translate(-50%, -50%);
  outline: none;
}

.workspace-edit-title {
  margin: 0 0 14px;
  color: var(--peek-text, #f3f4f6);
  font-size: var(--peek-font-lg, 14px);
  font-weight: 650;
  line-height: 1.35;
}

.workspace-edit-form {
  display: grid;
  gap: 10px;
}

.workspace-edit-field {
  display: grid;
  gap: 5px;
  color: var(--peek-muted, #b7bcc5);
  font-size: 11px;
  font-weight: 600;
}

.workspace-edit-notes {
  width: 100%;
  min-height: 72px;
  padding: 7px 9px;
  border: 1px solid var(--border, var(--peek-border));
  border-radius: 8px;
  background: var(--popover, var(--peek-surface));
  color: var(--peek-text);
  font: inherit;
  font-size: 13px;
  line-height: 1.45;
  resize: vertical;
  outline: none;
}

.workspace-edit-notes:focus {
  border-color: color-mix(in srgb, var(--foreground) 25%, var(--border));
}

.workspace-edit-path {
  display: grid;
  gap: 4px;
  margin: 0;
  color: var(--peek-muted);
  font-size: 11px;
  font-weight: 600;
}

.workspace-edit-path code {
  overflow: hidden;
  color: var(--peek-text, #f3f4f6);
  font-family: var(--peek-font-mono, ui-monospace, monospace);
  font-size: 12px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.workspace-edit-error {
  margin: 0;
  color: var(--destructive, var(--peek-danger));
  font-size: 11px;
}

.workspace-edit-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 6px;
}

.workspace-edit-button {
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

.workspace-edit-button.ghost {
  border-color: var(--peek-border, rgba(255, 255, 255, 0.14));
  background: color-mix(in srgb, var(--peek-text, #f3f4f6) 4%, transparent);
  color: var(--peek-text, #f3f4f6);
}

.workspace-edit-button.ghost:hover {
  background: var(--peek-hover-bg, color-mix(in srgb, var(--peek-icon, #e5e7eb) 9%, transparent));
}

.workspace-edit-button.primary {
  background: var(--peek-send-active-bg, var(--peek-accent));
  color: var(--peek-send-active-fg, var(--peek-primary-foreground));
}

.workspace-edit-button.primary:disabled {
  cursor: default;
  opacity: 0.6;
}
</style>
