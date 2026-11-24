<script setup lang="ts">
/** 空对话：未配置模型时直接填写密钥。 */
import { computed } from "vue";

import MascotFace from "@/components/icons/MascotFace.vue";
import ProviderSetupCard from "@/components/chat/ProviderSetupCard.vue";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";

defineProps<{
  hasProvider: boolean;
  prompt: string;
  composerFocused: boolean;
  lookAt: HTMLElement | null;
}>();

const emit = defineEmits<{
  openProviders: [];
}>();

const settingStore = useSettingStore();
const setupTitle = computed(() => tr(settingStore.language, "onboarding.emptyProviderTitle"));
const hotkey = computed(() => tr(settingStore.language, "onboarding.emptyHotkey"));
</script>

<template>
  <div class="empty-hero-body">
    <div class="empty-conversation-brand" data-onboarding-logo-target aria-hidden="true">
      <MascotFace interactive eyes-only :look-at="composerFocused ? lookAt : null" />
    </div>

    <p v-if="hasProvider" class="empty-conversation-prompt">{{ prompt }}</p>
    <template v-if="!hasProvider">
      <p class="empty-conversation-prompt is-setup">{{ setupTitle }}</p>
      <div class="empty-provider" data-tour="provider-setup">
        <ProviderSetupCard @open-providers="emit('openProviders')" />
      </div>
    </template>

    <p class="empty-hotkey" data-tour="hotkey-hint">{{ hotkey }}</p>
  </div>
</template>

<style scoped>
.empty-hero-body {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  width: 100%;
}

.empty-conversation-brand {
  width: 112px;
  height: 84px;
  flex: none;
}

.empty-conversation-brand :deep(.mascot-face) {
  opacity: 1;
}

.empty-conversation-prompt {
  margin: 0;
  max-width: 18em;
  color: var(--peek-conversation-text, var(--peek-text));
  font-size: 22px;
  font-weight: 560;
  letter-spacing: -0.025em;
  line-height: 1.35;
  text-align: center;
  text-wrap: balance;
}

.empty-conversation-prompt.is-setup {
  font-size: 20px;
}

.empty-provider {
  width: min(100%, 460px);
}

.empty-provider :deep(.provider-setup-card) {
  width: 100%;
}

.empty-hotkey {
  margin: 0;
  max-width: 28em;
  color: var(--peek-muted, color-mix(in srgb, var(--peek-text) 62%, transparent));
  font-size: 12.5px;
  line-height: 1.5;
  text-align: center;
}
</style>
