<template>
  <div class="provider-models-field field-row">
    <div class="models-header-row">
      <label>{{ t("settings.provider.modelsList") }}</label>
      <Button
        type="button"
        size="sm"
        variant="outline"
        class="h-7 gap-1 text-xs"
        :disabled="fetching || !canFetch"
        @click="$emit('fetch')"
      >
        <RefreshCw class="size-3.5" :class="{ spinning: fetching }" />
        {{ fetching ? t("settings.provider.fetchingModels") : t("settings.provider.fetchModels") }}
      </Button>
    </div>
    <p class="field-hint">{{ t("settings.provider.modelsHint") }}</p>
    <p v-if="fetchError" class="oauth-error">{{ fetchError }}</p>
    <div class="models-editor">
      <div class="models-add-row">
        <Input
          ref="modelDraftInputRef"
          v-model="modelDraft"
          :placeholder="t('settings.provider.modelsPlaceholder')"
          class="h-8 min-w-0 flex-1 text-xs font-mono"
          @keydown.enter.exact.prevent="onAdd"
        />
        <Button type="button" size="sm" class="h-8 gap-1 shrink-0" @click="onAdd">
          <Plus class="size-3.5" />
          {{ t("settings.provider.addModel") }}
        </Button>
      </div>

      <ul
        v-if="models.length > 0"
        class="models-list"
        :aria-label="t('settings.provider.modelsList')"
      >
        <li
          v-for="(model, index) in models"
          :key="`${model}-${index}`"
          class="model-item"
          :class="{ 'is-disabled': isModelDisabled(model) }"
        >
          <component
            :is="modelBrandIcon(model)"
            v-if="modelBrandIcon(model)"
            :size="14"
            class="model-row-icon"
            aria-hidden="true"
          />
          <span v-else class="model-row-icon-dot" aria-hidden="true" />
          <code class="model-id">{{ model }}</code>
          <Select
            v-if="supportsProtocolSelect"
            :model-value="modelProtocolValue(model)"
            @update:model-value="(value) => onModelProtocolChange(model, value)"
          >
            <SelectTrigger
              size="sm"
              class="model-protocol-select w-[8.75rem]"
              :aria-label="t('settings.provider.modelProtocol')"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="inherit">
                {{ t("settings.provider.modelProtocolUnset") }}
              </SelectItem>
              <SelectItem value="chatCompletions">
                {{ t("settings.provider.modelProtocolChatCompletions") }}
              </SelectItem>
              <SelectItem value="responses">
                {{ t("settings.provider.modelProtocolResponses") }}
              </SelectItem>
              <SelectItem value="anthropicMessages">
                {{ t("settings.provider.modelProtocolAnthropic") }}
              </SelectItem>
            </SelectContent>
          </Select>
          <div class="model-item-actions">
            <SettingsToggle
              compact
              :model-value="!isModelDisabled(model)"
              :aria-label="
                isModelDisabled(model)
                  ? t('settings.provider.enableModel')
                  : t('settings.provider.disableModel')
              "
              :title="
                isModelDisabled(model)
                  ? t('settings.provider.enableModel')
                  : t('settings.provider.disableModel')
              "
              @update:model-value="() => $emit('toggleDisabled', model)"
            />
            <button
              type="button"
              class="model-remove"
              :aria-label="t('settings.provider.removeModel')"
              @click="$emit('remove', index)"
            >
              <X class="size-3.5" />
            </button>
          </div>
        </li>
      </ul>
      <p v-else class="models-empty">{{ t("settings.provider.modelsEmpty") }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { nextTick, ref } from "vue";
import { Plus, RefreshCw, X } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import SettingsToggle from "@/components/settings/SettingsToggle.vue";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import type { SettingsI18nKey } from "@/services/locales/settings";
import { normalizeModelProtocol, type ModelWireProtocol } from "@/types/setting";
import { getModelIcon } from "@/lib/providerIcons";

const props = withDefaults(
  defineProps<{
    models: string[];
    disabledModels: Set<string>;
    fetching?: boolean;
    canFetch?: boolean;
    fetchError?: string;
    supportsProtocolSelect?: boolean;
    modelProtocols?: Record<string, ModelWireProtocol>;
  }>(),
  {
    fetching: false,
    canFetch: false,
    fetchError: "",
    supportsProtocolSelect: false,
    modelProtocols: () => ({}),
  },
);

const emit = defineEmits<{
  fetch: [];
  add: [modelId: string];
  remove: [index: number];
  toggleDisabled: [modelId: string];
  protocolChange: [modelId: string, protocol: ModelWireProtocol | undefined];
}>();

const settingStore = useSettingStore();
const modelDraft = ref("");
const modelDraftInputRef = ref<{ $el?: HTMLElement } | null>(null);

const t = (key: string) => tr(settingStore.language, key as SettingsI18nKey);

function modelBrandIcon(modelId: string) {
  return getModelIcon({ id: modelId, displayName: undefined });
}

function isModelDisabled(model: string): boolean {
  return props.disabledModels.has(model);
}

function modelProtocolValue(model: string): string {
  return props.modelProtocols[model] ?? "inherit";
}

function onModelProtocolChange(model: string, value: unknown) {
  const protocol = normalizeModelProtocol(value);
  emit("protocolChange", model, protocol);
}

async function focusDraft() {
  await nextTick();
  const instance = modelDraftInputRef.value;
  const root = instance?.$el;
  const input = root instanceof HTMLInputElement ? root : root?.querySelector?.("input");
  input?.focus();
}

async function onAdd() {
  const value = modelDraft.value.trim();
  if (!value) {
    await focusDraft();
    return;
  }
  emit("add", value);
  modelDraft.value = "";
  await focusDraft();
}
</script>

<style scoped>
.field-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-row label {
  font-size: 11px;
  font-weight: 500;
  color: var(--muted-foreground);
}

.field-hint {
  margin: 0;
  font-size: 10px;
  line-height: 1.4;
  color: var(--muted-foreground);
}

.models-header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.spinning {
  animation: provider-spin 0.8s linear infinite;
}

@keyframes provider-spin {
  to {
    transform: rotate(360deg);
  }
}

.oauth-error {
  margin: 4px 0 0;
  font-size: 11px;
  color: var(--destructive, #ef4444);
  line-height: 1.4;
}

.models-editor {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.models-add-row {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}

.models-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.model-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 6px 8px 6px 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: color-mix(in srgb, var(--sidebar) 55%, transparent);
}

.model-row-icon {
  flex: none;
  color: var(--foreground);
  opacity: 0.9;
}

.model-row-icon-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex: none;
  background: color-mix(in srgb, var(--muted-foreground) 50%, transparent);
}

.model-item.is-disabled {
  opacity: 0.55;
}

.model-id {
  min-width: 0;
  flex: 1;
  font-family: var(--font-mono, ui-monospace, monospace);
  font-size: 11px;
  color: var(--foreground);
  overflow-wrap: anywhere;
}

.model-item.is-disabled .model-id {
  text-decoration: line-through;
  text-decoration-color: color-mix(in srgb, var(--muted-foreground) 60%, transparent);
}

.model-item-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.model-protocol-select {
  width: 8.75rem;
  flex: none;
  font-size: 11px;
}

.model-remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--muted-foreground);
  cursor: pointer;
  flex-shrink: 0;
  transition:
    background-color 0.15s,
    color 0.15s;
}

.model-remove:hover {
  background: color-mix(in srgb, var(--destructive, #ef4444) 12%, transparent);
  color: var(--destructive, #ef4444);
}

.models-empty {
  margin: 0;
  padding: 10px;
  border: 1px dashed var(--border);
  border-radius: 8px;
  font-size: 11px;
  line-height: 1.45;
  color: var(--muted-foreground);
  text-align: center;
}
</style>
