<script setup lang="ts">
/** 未配置提供商时，在对话里直接填写 DeepSeek API Key。 */
import { computed, ref } from "vue";
import { SecretInput } from "@/components/ui/secret-input";
import { tr } from "@/services/i18n";
import { useChatModelStore } from "@/stores/chatModel";
import { useSettingStore } from "@/stores/setting";

const emit = defineEmits<{
  openProviders: [];
  saved: [];
}>();

const settingStore = useSettingStore();
const chatModelStore = useChatModelStore();
const apiKey = ref(settingStore.deepseekApiKey);
const saving = ref(false);
const saved = ref(false);
const failed = ref(false);
const saveLabel = computed(() => {
  if (saved.value) return tr(settingStore.language, "configureProviderSaved");
  if (saving.value) return tr(settingStore.language, "configureProviderSaving");
  return tr(settingStore.language, "configureProviderSave");
});

function onInput(value: string) {
  apiKey.value = value;
  saved.value = false;
  failed.value = false;
}

async function save() {
  const next = apiKey.value.trim();
  if (!next || saving.value) return;
  saving.value = true;
  failed.value = false;
  try {
    await settingStore.update({ deepseekApiKey: next });
    await chatModelStore.refresh();
    saved.value = true;
    emit("saved");
  } catch {
    failed.value = true;
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <form class="provider-setup-card" @submit.prevent="save">
    <p class="provider-setup-text">
      {{ tr(settingStore.language, "configureProviderHint") }}
    </p>
    <div class="provider-setup-key">
      <SecretInput
        :model-value="apiKey"
        :placeholder="tr(settingStore.language, 'configureProviderKeyPlaceholder')"
        :disabled="saving"
        @update:model-value="onInput"
      />
    </div>
    <button type="submit" class="provider-setup-save" :disabled="saving || !apiKey.trim()">
      {{ saveLabel }}
    </button>
    <p v-if="failed" class="provider-setup-error" role="alert">
      {{ tr(settingStore.language, "configureProviderSaveFailed") }}
    </p>
    <button type="button" class="provider-setup-more" @click="emit('openProviders')">
      {{ tr(settingStore.language, "configureProviderMore") }}
    </button>
  </form>
</template>

<style scoped>
.provider-setup-card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 10px;
  width: min(100%, 420px);
  margin: 0;
  padding: 14px;
  border: 1px solid color-mix(in srgb, var(--peek-text) 8%, transparent);
  border-radius: 16px;
  background: color-mix(in srgb, var(--peek-text) 3%, var(--peek-surface, #fff));
}

.provider-setup-text {
  margin: 0;
  font-size: 13px;
  line-height: 1.55;
  color: var(--peek-text);
}

.provider-setup-key {
  width: 100%;
}

.provider-setup-save {
  appearance: none;
  height: 32px;
  margin: 0;
  padding: 0 14px;
  border: 0;
  border-radius: 999px;
  background: var(--peek-text);
  color: var(--peek-bg, #fff);
  font-size: 12.5px;
  font-weight: 560;
  cursor: pointer;
}

.provider-setup-save:hover:not(:disabled) {
  opacity: 0.92;
}

.provider-setup-save:disabled {
  opacity: 0.45;
  cursor: default;
}

.provider-setup-error {
  margin: 0;
  color: var(--destructive, #b42318);
  font-size: 12px;
  line-height: 1.4;
}

.provider-setup-more {
  appearance: none;
  height: 28px;
  margin: 2px 0 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: color-mix(in srgb, var(--peek-text) 62%, transparent);
  font-size: 12.5px;
  font-weight: 500;
  cursor: pointer;
}

.provider-setup-more:hover {
  color: var(--peek-text);
}
</style>
