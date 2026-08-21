<template>
  <DialogRoot :open="open" @update:open="handleOpenChange">
    <DialogPortal>
      <DialogOverlay class="confirm-overlay" />
      <DialogContent class="confirm-dialog" :aria-describedby="undefined">
        <div class="confirm-body">
          <span class="confirm-icon" :class="options.tone" aria-hidden="true">
            <TriangleAlert v-if="options.tone === 'danger'" :size="16" />
            <Info v-else :size="16" />
          </span>
          <div class="confirm-copy">
            <DialogTitle class="confirm-title">{{ options.title }}</DialogTitle>
            <DialogDescription class="confirm-description">
              {{ options.description }}
            </DialogDescription>
          </div>
        </div>

        <div class="confirm-actions">
          <button type="button" class="confirm-button ghost" @click="settle(false)">
            {{ options.cancelLabel }}
          </button>
          <button
            type="button"
            class="confirm-button"
            :class="options.tone === 'danger' ? 'danger' : 'primary'"
            @click="settle(true)"
          >
            {{ options.confirmLabel }}
          </button>
        </div>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>

<script setup lang="ts">
import { reactive, ref } from "vue";
import { Info, TriangleAlert } from "@lucide/vue";
import {
  DialogContent,
  DialogDescription,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
} from "reka-ui";

export type ConfirmDialogTone = "default" | "danger";

export interface ConfirmDialogOptions {
  title: string;
  description: string;
  confirmLabel: string;
  cancelLabel: string;
  tone?: ConfirmDialogTone;
}

const open = ref(false);
const options = reactive<Required<ConfirmDialogOptions>>({
  title: "",
  description: "",
  confirmLabel: "Confirm",
  cancelLabel: "Cancel",
  tone: "danger",
});
let resolver: ((confirmed: boolean) => void) | null = null;

/** Open the dialog and resolve when the user confirms or cancels. */
function ask(nextOptions: ConfirmDialogOptions) {
  resolver?.(false);
  Object.assign(options, { tone: "danger" }, nextOptions);
  open.value = true;
  return new Promise<boolean>((resolve) => {
    resolver = resolve;
  });
}

/** Close the dialog and settle the pending promise. */
function settle(confirmed: boolean) {
  open.value = false;
  resolver?.(confirmed);
  resolver = null;
}

/** Treat outside-dismiss / Escape as cancel. */
function handleOpenChange(nextOpen: boolean) {
  if (!nextOpen && open.value) settle(false);
}

defineExpose({ ask });
</script>

<style>
.confirm-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  background: color-mix(in srgb, #000 48%, transparent);
  backdrop-filter: blur(2px);
}

.confirm-dialog {
  position: fixed;
  top: 50%;
  left: 50%;
  z-index: 51;
  box-sizing: border-box;
  width: min(360px, calc(100vw - 32px));
  padding: 16px;
  border: 1px solid var(--peek-border, rgba(255, 255, 255, 0.14));
  border-radius: var(--peek-radius-lg, 12px);
  background: var(--peek-dialog-bg, var(--peek-surface, #252526));
  color: var(--peek-text, #f3f4f6);
  box-shadow: var(--peek-elev-md);
  transform: translate(-50%, -50%);
  outline: none;
}

.confirm-dialog .confirm-body {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.confirm-dialog .confirm-icon {
  flex: none;
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  margin-top: 1px;
  border-radius: var(--peek-radius-md, 8px);
  background: color-mix(in srgb, var(--peek-info, #006ab1) 14%, transparent);
  color: var(--peek-info, #006ab1);
}

.confirm-dialog .confirm-icon.danger {
  background: color-mix(in srgb, var(--peek-danger, #f14c4c) 14%, transparent);
  color: var(--peek-danger, #f14c4c);
}

.confirm-dialog .confirm-copy {
  min-width: 0;
  flex: 1;
}

.confirm-dialog .confirm-title {
  margin: 0;
  color: var(--peek-text, #f3f4f6);
  font-size: var(--peek-font-lg, 14px);
  font-weight: 650;
  line-height: 1.35;
}

.confirm-dialog .confirm-description {
  margin: 6px 0 0;
  color: var(--peek-muted, #b7bcc5);
  font-size: var(--peek-font-sm, 12px);
  line-height: 1.55;
  white-space: pre-line;
}

.confirm-dialog .confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 16px;
}

.confirm-dialog .confirm-button {
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

.confirm-dialog .confirm-button.ghost {
  border-color: var(--peek-border, rgba(255, 255, 255, 0.14));
  background: color-mix(in srgb, var(--peek-text, #f3f4f6) 4%, transparent);
  color: var(--peek-text, #f3f4f6);
}

.confirm-dialog .confirm-button.ghost:hover {
  background: var(--peek-hover-bg, color-mix(in srgb, var(--peek-icon, #e5e7eb) 9%, transparent));
}

.confirm-dialog .confirm-button.primary {
  background: var(--peek-send-active-bg, var(--peek-accent));
  color: var(--peek-send-active-fg, var(--peek-primary-foreground));
}

.confirm-dialog .confirm-button.primary:hover {
  filter: brightness(1.06);
}

.confirm-dialog .confirm-button.danger {
  border-color: color-mix(in srgb, var(--peek-danger, #f14c4c) 35%, transparent);
  background: color-mix(in srgb, var(--peek-danger, #f14c4c) 18%, transparent);
  color: color-mix(in srgb, var(--peek-danger, #f14c4c) 88%, var(--peek-text, #f3f4f6));
}

.confirm-dialog .confirm-button.danger:hover {
  background: color-mix(in srgb, var(--peek-danger, #f14c4c) 28%, transparent);
}

[data-theme="light"] .confirm-dialog {
  background: var(--peek-surface, #ffffff);
  color: var(--peek-text, #242424);
  box-shadow: var(--peek-elev-md);
}
</style>
