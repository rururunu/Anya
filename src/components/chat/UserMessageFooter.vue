<script setup lang="ts">
import { computed, ref } from "vue";
import { Bot, ChevronDown, Plus } from "@lucide/vue";
import { PopoverContent, PopoverPortal, PopoverRoot, PopoverTrigger } from "reka-ui";
import ContextUsageRing from "@/components/chat/ContextUsageRing.vue";
import ModelPicker from "@/components/chat/input/ModelPicker.vue";
import OptionPicker from "@/components/chat/input/OptionPicker.vue";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import { useComposerFooterDisplay } from "@/composables/chat/useComposerFooterDisplay";
import { fullApprovalConfirmOptions } from "@/services/chat/fullApprovalConfirm";
import { tr } from "@/services/i18n";

const props = defineProps<{
  sessionId: string;
  canSend: boolean;
  busy: boolean;
}>();

const emit = defineEmits<{
  attach: [];
  send: [];
}>();

const modeOpen = ref(false);
const modelOpen = ref(false);
const thinkingOpen = ref(false);
const approvalOpen = ref(false);
const approvalConfirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);

const {
  language,
  chatMode,
  chatModeLabel,
  chatModeIcon,
  chatModeOptions,
  modelDisplayName,
  modelProviderIcon,
  chatModel,
  chatModelProvider,
  availableModels,
  modelPickerProvider,
  pickerIndex,
  showThinkingTier,
  thinkingTierLabel,
  thinkingOptions,
  thinkingSelectedId,
  approvalModeLabel,
  approvalIcon,
  approvalOptions,
  toolApprovalMode,
  showApproval,
  conversationTokenCount,
  conversationTokenTitle,
  cacheHitPercent,
  cacheHitTitle,
  contextUsage,
  formatTokenCount,
  chatModelStore,
  selectChatMode,
  selectApprovalMode,
  selectModel,
  applyThinkingTier,
  ensureModels,
} = useComposerFooterDisplay(() => props.sessionId, {
  confirmFullApproval: async () =>
    Boolean(await approvalConfirmDialogRef.value?.ask(fullApprovalConfirmOptions(language.value))),
});

const thinkingChipTitle = computed(() =>
  tr(language.value, "currentThinkingTier", { tier: thinkingTierLabel.value }),
);

function onModelOpen(open: boolean) {
  modelOpen.value = open;
  if (open) {
    pickerIndex.value = 0;
    ensureModels();
  } else {
    modelPickerProvider.value = null;
  }
}

function onThinkingOpen(open: boolean) {
  thinkingOpen.value = open;
  if (open) {
    pickerIndex.value = Math.max(
      0,
      thinkingOptions.value.findIndex((option) => option.id === thinkingSelectedId.value),
    );
  }
}
</script>

<template>
  <div class="user-message-footer" data-tauri-drag-region="false" @mousedown.stop @click.stop>
    <AppConfirmDialog ref="approvalConfirmDialogRef" />
    <div class="input-footer-primary">
      <button
        type="button"
        class="attach-trigger-btn"
        data-tauri-drag-region="false"
        :title="tr(language, 'chatInput.attachResources')"
        :aria-label="tr(language, 'chatInput.attachResources')"
        @click="emit('attach')"
      >
        <Plus :size="18" :stroke-width="2" aria-hidden="true" />
      </button>

      <PopoverRoot v-model:open="modeOpen">
        <PopoverTrigger as-child>
          <button
            type="button"
            class="model-badge footer-chip"
            :class="{ open: modeOpen }"
            data-tauri-drag-region="false"
            :title="chatModeLabel"
            :aria-label="chatModeLabel"
          >
            <component :is="chatModeIcon" :size="13" class="footer-chip-icon" />
            <span class="model-name">{{ chatModeLabel }}</span>
            <ChevronDown :size="11" class="model-chevron" />
          </button>
        </PopoverTrigger>
        <PopoverPortal>
          <PopoverContent class="user-footer-picker" side="bottom" align="start" :side-offset="6">
            <OptionPicker
              compact
              :options="chatModeOptions"
              :selected-id="chatMode"
              :selected-index="pickerIndex"
              :ariaLabel="tr(language, 'chooseChatMode')"
              @hover="pickerIndex = $event"
              @select="
                (id) => {
                  selectChatMode(id);
                  modeOpen = false;
                }
              "
            />
          </PopoverContent>
        </PopoverPortal>
      </PopoverRoot>

      <PopoverRoot v-if="showApproval" v-model:open="approvalOpen">
        <PopoverTrigger as-child>
          <button
            type="button"
            class="model-badge footer-chip"
            :class="{ open: approvalOpen, 'is-full-approve': toolApprovalMode === 'alwaysAllow' }"
            data-tauri-drag-region="false"
            :title="approvalModeLabel"
            :aria-label="approvalModeLabel"
          >
            <component :is="approvalIcon" :size="13" class="footer-chip-icon" />
            <span class="model-name">{{ approvalModeLabel }}</span>
            <ChevronDown :size="11" class="model-chevron" />
          </button>
        </PopoverTrigger>
        <PopoverPortal>
          <PopoverContent
            class="user-footer-picker user-footer-picker-approval"
            side="bottom"
            align="start"
            :side-offset="6"
          >
            <OptionPicker
              compact
              :options="approvalOptions"
              :selected-id="toolApprovalMode"
              :selected-index="pickerIndex"
              :ariaLabel="tr(language, 'toolApprovalMode')"
              @hover="pickerIndex = $event"
              @select="
                (id) => {
                  selectApprovalMode(id);
                  approvalOpen = false;
                }
              "
            />
          </PopoverContent>
        </PopoverPortal>
      </PopoverRoot>
    </div>

    <div class="input-footer-actions">
      <div class="composer-model-controls">
        <PopoverRoot :open="modelOpen" @update:open="onModelOpen">
          <PopoverTrigger as-child>
            <button
              type="button"
              class="model-badge footer-chip model-control"
              :class="{ open: modelOpen }"
              data-tauri-drag-region="false"
              :title="modelDisplayName"
              :aria-label="modelDisplayName"
            >
              <span class="footer-chip-icon-slot" aria-hidden="true">
                <component :is="modelProviderIcon || Bot" :size="13" class="footer-chip-icon" />
              </span>
              <span class="model-name">{{ modelDisplayName }}</span>
              <ChevronDown :size="11" class="model-chevron" />
            </button>
          </PopoverTrigger>
          <PopoverPortal>
            <PopoverContent
              class="user-footer-picker user-footer-picker-model"
              side="bottom"
              align="start"
              :side-offset="6"
            >
              <ModelPicker
                :models="availableModels"
                :selected-model-id="chatModel"
                :selected-provider="chatModelProvider"
                :selected-index="pickerIndex"
                :active-provider="modelPickerProvider"
                :loading="chatModelStore.loading"
                :refreshing="chatModelStore.refreshing"
                :error="chatModelStore.error"
                :loading-text="tr(language, 'loadingModels')"
                :empty-text="tr(language, 'noModels')"
                :refresh-text="tr(language, 'refreshModels')"
                :back-text="tr(language, 'backToProviders')"
                :model-count-text="tr(language, 'providerModelCount')"
                :ariaLabel="tr(language, 'chooseModel')"
                :thinking-options="thinkingOptions"
                :thinking-selected-id="thinkingSelectedId"
                :thinking-title="tr(language, 'thinkingTierLabel')"
                @hover="pickerIndex = $event"
                @select="
                  (entry) => {
                    selectModel(entry);
                    modelOpen = false;
                  }
                "
                @select-group="modelPickerProvider = $event"
                @back="modelPickerProvider = null"
                @refresh="void chatModelStore.reload()"
                @select-thinking="applyThinkingTier"
              />
            </PopoverContent>
          </PopoverPortal>
        </PopoverRoot>

        <PopoverRoot v-if="showThinkingTier" :open="thinkingOpen" @update:open="onThinkingOpen">
          <PopoverTrigger as-child>
            <button
              type="button"
              class="model-badge footer-chip thinking-control"
              :class="{ open: thinkingOpen }"
              data-tauri-drag-region="false"
              :title="thinkingChipTitle"
              :aria-label="thinkingChipTitle"
            >
              <span class="model-name">{{ thinkingTierLabel }}</span>
            </button>
          </PopoverTrigger>
          <PopoverPortal>
            <PopoverContent
              class="user-footer-picker user-footer-picker-thinking"
              side="bottom"
              align="start"
              :side-offset="6"
            >
              <OptionPicker
                compact
                :options="thinkingOptions"
                :selected-id="thinkingSelectedId"
                :selected-index="pickerIndex"
                :ariaLabel="tr(language, 'chooseThinkingTier')"
                @hover="pickerIndex = $event"
                @select="
                  (id) => {
                    applyThinkingTier(id);
                    thinkingOpen = false;
                  }
                "
              />
            </PopoverContent>
          </PopoverPortal>
        </PopoverRoot>
      </div>

      <span
        v-if="conversationTokenCount || cacheHitPercent != null"
        class="conversation-token-meta"
      >
        <span
          v-if="conversationTokenCount"
          class="conversation-token-count"
          :title="conversationTokenTitle"
        >
          {{ formatTokenCount(conversationTokenCount, language) }}
        </span>
        <span
          v-if="conversationTokenCount && cacheHitPercent != null"
          class="conversation-token-sep"
          aria-hidden="true"
        >
          |
        </span>
        <span v-if="cacheHitPercent != null" class="conversation-cache-hit" :title="cacheHitTitle">
          {{ cacheHitPercent }}%
        </span>
      </span>

      <ContextUsageRing v-if="contextUsage.contextWindowTokens > 0" :usage="contextUsage" />

      <button
        type="button"
        class="send-btn"
        :class="{ active: canSend }"
        data-tauri-drag-region="false"
        :aria-label="tr(language, 'send')"
        :disabled="!canSend || busy"
        @click.stop="emit('send')"
      >
        <svg viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path
            d="M8 13V3M3.5 7.5L8 3L12.5 7.5"
            stroke="currentColor"
            stroke-width="1.35"
            stroke-linejoin="round"
          />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.user-message-footer,
.input-footer-primary,
.input-footer-actions {
  display: flex;
  align-items: center;
}
.user-message-footer {
  width: 100%;
  min-width: 0;
  min-height: 30px;
  justify-content: space-between;
  gap: 8px;
  padding-top: 4px;
  container-type: inline-size;
  container-name: user-composer;
}
.input-footer-primary {
  flex: 1;
  min-width: 0;
  gap: 3px;
  flex-wrap: wrap;
  overflow: visible;
}
.input-footer-actions {
  flex: none;
  min-width: 0;
  gap: 8px;
  flex-wrap: nowrap;
}
.composer-model-controls {
  display: flex;
  flex: none;
  align-items: center;
  gap: 0;
  min-width: 0;
}
.attach-trigger-btn {
  flex: none;
  width: var(--peek-control-icon, 28px);
  height: var(--peek-control-icon, 28px);
  margin-left: -6px;
  margin-right: 6px;
  border: 0;
  background: transparent;
  color: var(--peek-muted);
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: color var(--motion-fast, 110ms) ease;
}
.attach-trigger-btn:hover {
  color: var(--peek-text);
}
.attach-trigger-btn:focus-visible {
  outline: 2px solid var(--peek-focus);
  outline-offset: 2px;
}
.send-btn {
  flex: none;
  width: 34px;
  height: 34px;
  border: 0;
  border-radius: 50%;
  background: var(--peek-send-bg);
  color: var(--peek-send-fg);
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: default;
  transform: translateZ(0);
  transition:
    background 120ms ease,
    color 120ms ease,
    transform 140ms cubic-bezier(0.22, 1, 0.36, 1);
}
.send-btn.active:hover:not(:disabled) {
  transform: scale(1.03);
}
.send-btn.active:active:not(:disabled) {
  transform: scale(0.97);
}
.footer-chip {
  height: var(--peek-control-icon, 28px);
  padding: 0 8px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--peek-muted);
  box-shadow: none;
  font-family: var(--peek-font-sans);
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.01em;
  line-height: 1.2;
  cursor: pointer;
  transition:
    color var(--motion-fast, 110ms) ease,
    opacity var(--motion-fast, 110ms) ease;
}
.footer-chip:hover,
.footer-chip:focus-visible,
.footer-chip:active {
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 6%, transparent);
  border: 0;
  box-shadow: none;
  outline: none;
}
.footer-chip.open {
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-accent) 10%, var(--peek-surface));
}
.footer-chip:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--peek-accent) 45%, transparent);
  outline-offset: 2px;
}
.footer-chip.is-full-approve,
.footer-chip.is-full-approve:hover,
.footer-chip.is-full-approve.open,
.footer-chip.is-full-approve:focus-visible,
.footer-chip.is-full-approve:active {
  color: var(--peek-danger, #ef4444);
}
.footer-chip.is-full-approve .footer-chip-icon {
  opacity: 1;
  color: var(--peek-danger, #ef4444);
}
.footer-chip-icon {
  flex: none;
  opacity: 0.78;
  transition:
    opacity 140ms ease,
    color 140ms ease;
}
.footer-chip:hover .footer-chip-icon,
.footer-chip.open .footer-chip-icon {
  opacity: 1;
}
.footer-chip-icon-slot {
  flex: none;
  width: 13px;
  height: 13px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.model-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  max-width: 164px;
  min-width: 0;
  user-select: none;
}
.model-control {
  padding-right: 4px;
}
.thinking-control {
  padding-inline: 4px;
}
.model-name {
  display: block;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.5;
  padding-block: 1px;
}
.model-chevron {
  flex: none;
  opacity: 0.45;
}
@container user-composer (max-width: 600px) {
  .composer-model-controls {
    gap: 4px;
  }
  .input-footer-primary .model-name,
  .input-footer-primary .model-chevron,
  .model-control .model-name,
  .model-control .model-chevron {
    display: none;
  }
  .input-footer-primary .footer-chip,
  .model-control {
    width: var(--peek-control-icon, 28px);
    max-width: none;
    padding: 0;
    justify-content: center;
    gap: 0;
  }
}
.conversation-token-meta {
  display: inline-flex;
  align-items: baseline;
  gap: 5px;
  flex: none;
  min-width: 0;
  color: var(--peek-faint);
  font-size: 10px;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
  user-select: none;
}
.conversation-token-sep {
  flex: none;
  opacity: 0.45;
}
.send-btn svg {
  width: 18px;
  height: 18px;
}
.send-btn:focus-visible {
  outline: 2px solid var(--peek-accent);
  outline-offset: 3px;
}
.send-btn.active {
  background: var(--peek-send-active-bg, var(--peek-send-bg));
  color: var(--peek-send-active-fg, var(--peek-send-fg));
  cursor: pointer;
}
.send-btn:disabled {
  opacity: 0.45;
  cursor: default;
  transform: none;
}
@media (prefers-reduced-motion: reduce) {
  .attach-trigger-btn,
  .attach-trigger-btn:hover,
  .attach-trigger-btn:active,
  .send-btn,
  .send-btn.active:hover:not(:disabled),
  .send-btn.active:active:not(:disabled) {
    transition: none;
    transform: none;
  }
}
</style>

<style>
.user-footer-picker {
  z-index: 60;
  width: min(260px, 82vw);
  padding: 0;
  border: 0;
  background: transparent;
  outline: none;
}
.user-footer-picker-model {
  width: min(320px, 86vw);
}
.user-footer-picker-approval {
  width: min(160px, 70vw);
}
.user-footer-picker-thinking {
  width: min(160px, 70vw);
}
.user-footer-picker .command-list {
  border: 1px solid color-mix(in srgb, var(--peek-text) 12%, transparent);
  border-radius: 12px;
  background: var(--peek-surface, var(--peek-list-bg));
  box-shadow: 0 10px 28px color-mix(in srgb, #000 16%, transparent);
  max-height: min(320px, 44vh);
}
</style>
