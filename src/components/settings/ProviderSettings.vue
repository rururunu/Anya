<template>
  <section class="provider-settings">
    <AppConfirmDialog ref="confirmDialogRef" />

    <Transition name="fade-slide" mode="out-in">
      <div v-if="currentView === 'list'" key="list" class="view-container">
        <header class="view-header">
          <div>
            <h2>{{ t("settings.provider.title") }}</h2>
            <p>{{ t("settings.provider.description") }}</p>
          </div>
        </header>

        <div class="cards-list">
          <button type="button" class="provider-nav-card" @click="currentView = 'deepseek'">
            <div class="card-left">
              <div class="icon-wrapper">
                <DeepSeekIcon :size="18" />
              </div>
              <div class="card-text">
                <h3>{{ t("settings.provider.deepseek") }}</h3>
                <p>DeepSeek API</p>
              </div>
            </div>
            <div class="card-right">
              <span class="status-badge" :class="{ configured: isDeepSeekConfigured }">
                {{ statusLabel(isDeepSeekConfigured) }}
              </span>
              <ChevronRight class="size-4 text-muted-foreground arrow-icon" />
            </div>
          </button>

          <button type="button" class="provider-nav-card" @click="openGemini">
            <div class="card-left">
              <div class="icon-wrapper">
                <GeminiIcon :size="18" />
              </div>
              <div class="card-text">
                <h3>{{ t("settings.provider.gemini") }}</h3>
                <p>{{ geminiSubtitle }}</p>
              </div>
            </div>
            <div class="card-right">
              <span class="status-badge" :class="{ configured: isGeminiConfigured }">
                {{ statusLabel(isGeminiConfigured) }}
              </span>
              <ChevronRight class="size-4 text-muted-foreground arrow-icon" />
            </div>
          </button>

          <button
            v-for="provider in settingStore.customProviders"
            :key="provider.id"
            type="button"
            class="provider-nav-card"
            @click="startEditCustom(provider.id)"
          >
            <div class="card-left">
              <div class="icon-wrapper">
                <component
                  :is="customProviderIcon(provider)"
                  v-if="customProviderIcon(provider)"
                  :size="18"
                />
                <Globe2 v-else class="size-5" />
              </div>
              <div class="card-text">
                <h3>{{ provider.name }}</h3>
                <p class="truncate max-w-[280px]">{{ customProviderSubtitle(provider) }}</p>
              </div>
            </div>
            <div class="card-right">
              <span class="status-badge" :class="{ configured: isCustomConfigured(provider) }">
                {{ statusLabel(isCustomConfigured(provider)) }}
              </span>
              <ChevronRight class="size-4 text-muted-foreground arrow-icon" />
            </div>
          </button>
        </div>

        <div class="add-section">
          <h4 class="add-section-title">{{ t("settings.provider.presets") }}</h4>
          <div class="cards-list">
            <button
              v-for="preset in availablePresets"
              :key="preset.id"
              type="button"
              class="provider-nav-card is-add"
              @click="addFromPreset(preset)"
            >
              <div class="card-left">
                <div class="icon-wrapper is-muted">
                  <component :is="presetIcon(preset.id)" v-if="presetIcon(preset.id)" :size="18" />
                  <Plus v-else class="size-4" />
                </div>
                <div class="card-text">
                  <h3>{{ preset.name }}</h3>
                  <p class="truncate max-w-[280px]">{{ preset.baseUrl }}</p>
                </div>
              </div>
              <div class="card-right">
                <span class="status-badge add-badge">
                  <Plus class="size-3" />
                  {{ t("settings.provider.add") }}
                </span>
              </div>
            </button>

            <button type="button" class="provider-nav-card is-add" @click="addBlankCustomProvider">
              <div class="card-left">
                <div class="icon-wrapper is-muted">
                  <Globe2 class="size-5" />
                </div>
                <div class="card-text">
                  <h3>{{ t("settings.provider.addBlank") }}</h3>
                  <p>OpenAI-compatible API</p>
                </div>
              </div>
              <div class="card-right">
                <span class="status-badge add-badge">
                  <Plus class="size-3" />
                  {{ t("settings.provider.add") }}
                </span>
              </div>
            </button>
          </div>
        </div>
      </div>

      <div v-else-if="currentView === 'deepseek'" key="deepseek" class="view-container">
        <div class="back-btn-row">
          <Button
            variant="ghost"
            size="sm"
            class="h-8 gap-1.5 pl-1.5 text-muted-foreground hover:text-foreground back-btn"
            @click="currentView = 'list'"
          >
            <ChevronLeft class="size-4" />
            {{ t("settings.provider.back") }}
          </Button>
        </div>
        <header class="view-header edit-header">
          <div class="header-details">
            <div class="edit-title-row">
              <DeepSeekIcon :size="18" class="edit-title-icon" />
              <h2>{{ t("settings.provider.deepseek") }}</h2>
            </div>
            <p>DeepSeek API</p>
          </div>
        </header>

        <div class="edit-form border-t border-border pt-4">
          <div class="field-row">
            <label>{{ t("settings.provider.apiKey") }}</label>
            <SecretInput v-model="deepseekKey" placeholder="sk-..." @blur="saveDeepSeek" />
            <p class="field-hint">
              {{ t("settings.provider.getApiKey") }}
              <button
                type="button"
                class="provider-key-link"
                @click="openExternalUrl(DEEPSEEK_API_KEYS_URL)"
              >
                {{ DEEPSEEK_API_KEYS_URL }}
              </button>
            </p>
          </div>

          <div class="form-actions">
            <Button size="sm" class="h-8 w-full gap-1.5" @click="saveDeepSeekAndGoBack">
              <Save class="size-3.5" />
              {{ t("settings.provider.save") }}
            </Button>
          </div>
        </div>
      </div>

      <div v-else-if="currentView === 'gemini'" key="gemini" class="view-container">
        <div class="back-btn-row">
          <Button
            variant="ghost"
            size="sm"
            class="h-8 gap-1.5 pl-1.5 text-muted-foreground hover:text-foreground back-btn"
            @click="currentView = 'list'"
          >
            <ChevronLeft class="size-4" />
            {{ t("settings.provider.back") }}
          </Button>
        </div>
        <header class="view-header edit-header">
          <div class="header-details">
            <div class="edit-title-row">
              <GeminiIcon :size="18" class="edit-title-icon" />
              <h2>{{ t("settings.provider.gemini") }}</h2>
            </div>
            <p>{{ t("settings.provider.geminiDescription") }}</p>
          </div>
        </header>

        <div class="edit-form border-t border-border pt-4">
          <div class="oauth-status">
            <p class="oauth-status-label">{{ t("settings.provider.geminiAccount") }}</p>
            <p class="oauth-status-value">
              {{
                isGeminiConfigured
                  ? settingStore.geminiOauth.email || t("settings.provider.configured")
                  : t("settings.provider.notConfigured")
              }}
            </p>
            <p v-if="geminiError" class="oauth-error">{{ geminiError }}</p>
          </div>

          <div class="form-actions">
            <Button
              v-if="!isGeminiConfigured && !geminiBusy"
              size="sm"
              class="h-8 w-full gap-1.5"
              @click="loginGemini"
            >
              {{ t("settings.provider.geminiLogin") }}
            </Button>
            <template v-else-if="!isGeminiConfigured && geminiBusy">
              <Button size="sm" class="h-8 flex-1 gap-1.5" disabled>
                {{ t("settings.provider.geminiLoggingIn") }}
              </Button>
              <Button
                variant="outline"
                size="sm"
                class="h-8 flex-1 gap-1.5"
                @click="cancelGeminiLogin"
              >
                {{ t("settings.provider.geminiCancelLogin") }}
              </Button>
            </template>
            <Button
              v-else
              variant="outline"
              size="sm"
              class="h-8 w-full gap-1.5"
              :disabled="geminiBusy"
              @click="logoutGemini"
            >
              {{ t("settings.provider.geminiLogout") }}
            </Button>
          </div>
        </div>
      </div>

      <div v-else-if="currentView === 'custom'" key="custom" class="view-container">
        <div class="back-btn-row">
          <Button
            variant="ghost"
            size="sm"
            class="h-8 gap-1.5 pl-1.5 text-muted-foreground hover:text-foreground back-btn"
            @click="currentView = 'list'"
          >
            <ChevronLeft class="size-4" />
            {{ t("settings.provider.back") }}
          </Button>
        </div>
        <header class="view-header edit-header">
          <div class="header-details">
            <div class="edit-title-row">
              <component
                :is="customEditIcon"
                v-if="customEditIcon"
                :size="18"
                class="edit-title-icon"
              />
              <Globe2 v-else :size="18" class="edit-title-icon" />
              <h2>
                {{
                  customName.trim() ||
                  (isNewProvider ? t("settings.provider.add") : t("settings.provider.custom"))
                }}
              </h2>
            </div>
            <p>OpenAI-compatible API</p>
          </div>

          <Button
            v-if="!isNewProvider"
            variant="destructive"
            size="sm"
            class="h-8 gap-1.5 delete-top-btn"
            @click="deleteCustom(editingProviderId)"
          >
            <Trash2 class="size-3.5" />
            {{ t("settings.provider.delete") }}
          </Button>
        </header>

        <div class="edit-form border-t border-border pt-4">
          <div class="field-row">
            <label>{{ t("settings.provider.name") }}</label>
            <Input
              v-model="customName"
              :placeholder="t('settings.provider.namePlaceholder')"
              class="h-8 text-xs"
              @blur="saveCustom"
            />
          </div>

          <div class="field-row">
            <label>{{ t("settings.provider.baseUrl") }}</label>
            <Input
              v-model="customUrl"
              :placeholder="t('settings.provider.urlPlaceholder')"
              class="h-8 text-xs font-mono"
              @blur="saveCustom"
            />
          </div>

          <div class="field-row">
            <label>{{ t("settings.provider.apiKey") }}</label>
            <SecretInput v-model="customKey" placeholder="sk-..." @blur="saveCustom" />
          </div>

          <div class="field-row">
            <div class="models-header-row">
              <label>{{ t("settings.provider.modelsList") }}</label>
              <Button
                type="button"
                size="sm"
                variant="outline"
                class="h-7 gap-1 text-xs"
                :disabled="fetchingModels || !canFetchModels"
                @click="fetchRemoteModels"
              >
                <RefreshCw class="size-3.5" :class="{ spinning: fetchingModels }" />
                {{
                  fetchingModels
                    ? t("settings.provider.fetchingModels")
                    : t("settings.provider.fetchModels")
                }}
              </Button>
            </div>
            <p class="field-hint">{{ t("settings.provider.modelsHint") }}</p>
            <p v-if="fetchModelsError" class="oauth-error">{{ fetchModelsError }}</p>
            <div class="models-editor">
              <div class="models-add-row">
                <Input
                  ref="modelDraftInputRef"
                  v-model="customModelDraft"
                  :placeholder="t('settings.provider.modelsPlaceholder')"
                  class="h-8 min-w-0 flex-1 text-xs font-mono"
                  @keydown.enter.exact.prevent="addCustomModel"
                />
                <Button type="button" size="sm" class="h-8 gap-1 shrink-0" @click="addCustomModel">
                  <Plus class="size-3.5" />
                  {{ t("settings.provider.addModel") }}
                </Button>
              </div>

              <ul
                v-if="customModelList.length > 0"
                class="models-list"
                :aria-label="t('settings.provider.modelsList')"
              >
                <li
                  v-for="(model, index) in customModelList"
                  :key="`${model}-${index}`"
                  class="model-item"
                >
                  <code class="model-id">{{ model }}</code>
                  <button
                    type="button"
                    class="model-remove"
                    :aria-label="t('settings.provider.removeModel')"
                    @click="removeCustomModel(index)"
                  >
                    <X class="size-3.5" />
                  </button>
                </li>
              </ul>
              <p v-else class="models-empty">{{ t("settings.provider.modelsEmpty") }}</p>
            </div>
          </div>

          <div class="form-actions">
            <Button size="sm" class="h-8 w-full gap-1.5" @click="saveCustomAndGoBack">
              <Plus v-if="isNewProvider" class="size-3.5" />
              <Save v-else class="size-3.5" />
              {{ isNewProvider ? t("settings.provider.add") : t("settings.provider.save") }}
            </Button>
          </div>
        </div>
      </div>
    </Transition>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import type { Component } from "vue";
import { Globe2, ChevronLeft, ChevronRight, Plus, Trash2, Save, X, RefreshCw } from "@lucide/vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import DeepSeekIcon from "@/components/icons/DeepSeekIcon.vue";
import GeminiIcon from "@/components/icons/GeminiIcon.vue";
import { useSettingStore } from "@/stores/setting";
import { useChatModelStore } from "@/stores/chatModel";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SecretInput } from "@/components/ui/secret-input";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import {
  geminiOauthCancelLogin,
  geminiOauthLogin,
  geminiOauthLogout,
  listCustomProviderModels,
} from "@/services/ipc";
import { tr } from "@/services/i18n";
import type { SettingsI18nKey } from "@/services/locales/settings";
import type { CustomProviderConfig } from "@/types/setting";
import { getProviderIcon } from "@/lib/providerIcons";
import {
  CUSTOM_PROVIDER_PRESETS,
  isCustomProviderConfigured,
  looksLikeHttpUrl,
  parseProviderModels,
  serializeProviderModels,
  type ProviderPreset,
} from "@/lib/providerPresets";

const DEEPSEEK_API_KEYS_URL = "https://platform.deepseek.com/api_keys";

defineProps<{
  query?: string;
}>();

const settingStore = useSettingStore();
const chatModelStore = useChatModelStore();

async function openExternalUrl(url: string) {
  try {
    await openUrl(url);
  } catch (error) {
    console.error("failed to open url in default browser:", url, error);
  }
}

const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const modelDraftInputRef = ref<{ $el?: HTMLElement } | null>(null);

const currentView = ref<"list" | "deepseek" | "gemini" | "custom">("list");
const editingProviderId = ref<string | null>(null);
const customPresetId = ref<string | undefined>(undefined);

const deepseekKey = ref(settingStore.deepseekApiKey);
const geminiBusy = ref(false);
const geminiError = ref("");
const fetchingModels = ref(false);
const fetchModelsError = ref("");

const customName = ref("");
const customUrl = ref("");
const customKey = ref("");
const customModelList = ref<string[]>([]);
const customModelDraft = ref("");
const providerPresets = CUSTOM_PROVIDER_PRESETS;

const availablePresets = computed(() => {
  const used = new Set(
    settingStore.customProviders
      .map((provider) => provider.presetId)
      .filter((id): id is string => !!id),
  );
  return providerPresets.filter((preset) => !used.has(preset.id));
});

const customEditIcon = computed(() => getProviderIcon(null, customPresetId.value));

const canFetchModels = computed(
  () => looksLikeHttpUrl(customUrl.value) && !!customKey.value.trim(),
);

const isDeepSeekConfigured = computed(() => {
  return !!settingStore.deepseekApiKey.trim();
});

const isGeminiConfigured = computed(() => {
  const oauth = settingStore.geminiOauth;
  return !!(oauth?.accessToken?.trim() || oauth?.refreshToken?.trim());
});

const geminiSubtitle = computed(() => {
  if (isGeminiConfigured.value && settingStore.geminiOauth.email) {
    return settingStore.geminiOauth.email;
  }
  return "Antigravity";
});

const isNewProvider = computed(() => {
  if (!editingProviderId.value) return true;
  return !settingStore.customProviders.some((p) => p.id === editingProviderId.value);
});

const t = (key: string) => {
  return tr(settingStore.language, key as SettingsI18nKey);
};

function statusLabel(configured: boolean) {
  return configured ? t("settings.provider.configured") : t("settings.provider.notConfigured");
}

function presetIcon(presetId: string): Component | null {
  return getProviderIcon(null, presetId);
}

function customProviderIcon(provider: CustomProviderConfig): Component | null {
  return getProviderIcon(provider.id, provider.presetId);
}

function isCustomConfigured(provider: CustomProviderConfig) {
  return isCustomProviderConfigured(provider);
}

function customProviderSubtitle(provider: CustomProviderConfig) {
  const url = provider.baseUrl.trim();
  if (looksLikeHttpUrl(url)) return url;
  const models = parseProviderModels(provider.models);
  if (models.length > 0) {
    return `${models.length} model${models.length === 1 ? "" : "s"}`;
  }
  return t("settings.provider.urlPlaceholder");
}

function openGemini() {
  geminiError.value = "";
  currentView.value = "gemini";
}

async function loginGemini() {
  geminiError.value = "";
  geminiBusy.value = true;
  try {
    await geminiOauthLogin();
    await settingStore.load();
    await chatModelStore.refresh();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (!/sign-in was cancelled|sign-in was canceled|access_denied/i.test(message)) {
      geminiError.value = message;
    }
  } finally {
    geminiBusy.value = false;
  }
}

async function cancelGeminiLogin() {
  try {
    await geminiOauthCancelLogin();
  } catch {
    // Login await will surface the cancel/timeout result.
  }
}

async function logoutGemini() {
  geminiError.value = "";
  geminiBusy.value = true;
  try {
    await geminiOauthLogout();
    await settingStore.load();
    await chatModelStore.refresh();
  } catch (error) {
    geminiError.value = error instanceof Error ? error.message : String(error);
  } finally {
    geminiBusy.value = false;
  }
}

async function saveDeepSeek() {
  if (deepseekKey.value.trim() === settingStore.deepseekApiKey) {
    return;
  }
  await settingStore.update({ deepseekApiKey: deepseekKey.value.trim() });
  await chatModelStore.refresh();
}

async function saveDeepSeekAndGoBack() {
  await saveDeepSeek();
  currentView.value = "list";
}

function startEditCustom(id: string) {
  const provider = settingStore.customProviders.find((p) => p.id === id);
  if (provider) {
    editingProviderId.value = id;
    customPresetId.value = provider.presetId;
    customName.value = provider.name;
    customUrl.value = provider.baseUrl;
    customKey.value = provider.apiKey;
    customModelList.value = parseProviderModels(provider.models);
    customModelDraft.value = "";
    fetchModelsError.value = "";
    currentView.value = "custom";
  }
}

function newProviderId() {
  return Math.random().toString(36).substring(2, 11);
}

function addBlankCustomProvider() {
  editingProviderId.value = newProviderId();
  customPresetId.value = undefined;
  customName.value = "";
  customUrl.value = "";
  customKey.value = "";
  customModelList.value = [];
  customModelDraft.value = "";
  fetchModelsError.value = "";
  currentView.value = "custom";
}

function addFromPreset(preset: ProviderPreset) {
  const existing = settingStore.customProviders.find((provider) => provider.presetId === preset.id);
  if (existing) {
    startEditCustom(existing.id);
    return;
  }
  editingProviderId.value = newProviderId();
  customPresetId.value = preset.id;
  customName.value = preset.name;
  customUrl.value = preset.baseUrl;
  customKey.value = "";
  customModelList.value = [...preset.models];
  customModelDraft.value = "";
  fetchModelsError.value = "";
  currentView.value = "custom";
}

async function focusModelDraft() {
  await nextTick();
  const instance = modelDraftInputRef.value as { $el?: HTMLElement } | null | undefined;
  const root = instance?.$el;
  const input = root instanceof HTMLInputElement ? root : root?.querySelector?.("input");
  input?.focus();
}

async function addCustomModel() {
  const ids = parseProviderModels(customModelDraft.value);
  if (ids.length === 0) {
    await focusModelDraft();
    return;
  }
  const next = [...customModelList.value];
  for (const id of ids) {
    if (!next.includes(id)) next.push(id);
  }
  customModelList.value = next;
  customModelDraft.value = "";
  await saveCustom();
  await focusModelDraft();
}

async function removeCustomModel(index: number) {
  customModelList.value = customModelList.value.filter((_, i) => i !== index);
  await saveCustom();
}

async function fetchRemoteModels() {
  fetchModelsError.value = "";
  if (!canFetchModels.value) {
    fetchModelsError.value = t("settings.provider.fetchModelsFailed");
    return;
  }
  fetchingModels.value = true;
  try {
    const ids = await listCustomProviderModels(customUrl.value.trim(), customKey.value.trim());
    if (ids.length === 0) {
      fetchModelsError.value = t("settings.provider.fetchModelsFailed");
      return;
    }
    const merged = [...customModelList.value];
    for (const id of ids) {
      if (!merged.includes(id)) merged.push(id);
    }
    customModelList.value = merged;
    await saveCustom();
  } catch (error) {
    fetchModelsError.value =
      error instanceof Error ? error.message : t("settings.provider.fetchModelsFailed");
  } finally {
    fetchingModels.value = false;
  }
}

async function saveCustom() {
  if (!editingProviderId.value) return;

  const nextName = customName.value.trim() || `Custom - ${editingProviderId.value}`;
  const nextUrl = customUrl.value.trim();
  const nextKey = customKey.value.trim();
  const nextModels = serializeProviderModels(customModelList.value);
  const nextPresetId = customPresetId.value;

  const list = [...settingStore.customProviders];
  const index = list.findIndex((p) => p.id === editingProviderId.value);

  const updatedProvider: CustomProviderConfig = {
    id: editingProviderId.value,
    name: nextName,
    baseUrl: nextUrl,
    apiKey: nextKey,
    models: nextModels,
    presetId: nextPresetId,
  };

  if (index !== -1) {
    const current = list[index];
    if (
      current.name === nextName &&
      current.baseUrl === nextUrl &&
      current.apiKey === nextKey &&
      current.models === nextModels &&
      current.presetId === nextPresetId
    ) {
      return;
    }
    list[index] = updatedProvider;
  } else {
    list.push(updatedProvider);
  }

  await settingStore.update({ customProviders: list });
  await chatModelStore.refresh();
}

async function saveCustomAndGoBack() {
  await saveCustom();
  currentView.value = "list";
}

async function deleteCustom(id: string | null) {
  if (!id) return;
  const provider = settingStore.customProviders.find((p) => p.id === id);
  if (!provider) return;

  const confirmed = await confirmDialogRef.value?.ask({
    title: t("settings.provider.delete"),
    description: t("settings.provider.deleteConfirm"),
    confirmLabel: t("settings.history.deleteLabel"),
    cancelLabel: t("settings.history.cancel"),
  });
  if (!confirmed) return;

  const list = settingStore.customProviders.filter((p) => p.id !== id);
  await settingStore.update({ customProviders: list });
  await chatModelStore.refresh();
  currentView.value = "list";
}
</script>

<style scoped>
.provider-settings {
  display: flex;
  flex-direction: column;
  padding: 12px 16px 20px;
  min-height: 100%;
}

.view-container {
  display: flex;
  flex-direction: column;
  gap: 16px;
  width: 100%;
  max-width: 520px;
  margin: 0 auto;
}

header.view-header {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

header.view-header h2 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

header.view-header p {
  margin: 4px 0 0;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.5;
  max-width: 52ch;
}

.add-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-top: 4px;
}

.add-section-title {
  margin: 0;
  font-size: 11px;
  font-weight: 600;
  color: var(--muted-foreground);
  letter-spacing: 0.02em;
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

.cards-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.provider-nav-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: color-mix(in srgb, var(--sidebar) 55%, transparent);
  cursor: pointer;
  text-align: left;
  transition:
    background-color 0.2s,
    border-color 0.2s,
    transform 0.15s;
  width: 100%;
}

.provider-nav-card.is-add {
  border-style: dashed;
  background: transparent;
}

.provider-nav-card:hover {
  background: color-mix(in srgb, var(--sidebar) 85%, transparent);
  border-color: color-mix(in srgb, var(--primary) 30%, var(--border));
}

.provider-nav-card.is-add:hover {
  background: color-mix(in srgb, var(--primary) 6%, transparent);
}

.provider-nav-card:active {
  transform: scale(0.998);
}

.card-left {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.icon-wrapper {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  background: color-mix(in srgb, var(--primary) 10%, transparent);
  color: var(--primary);
  flex-shrink: 0;
}

.icon-wrapper.is-muted {
  background: color-mix(in srgb, var(--muted) 55%, transparent);
  color: var(--muted-foreground);
}

.card-text {
  min-width: 0;
}

.card-text h3 {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--foreground);
}

.card-text p {
  margin: 1px 0 0;
  font-size: 11px;
  color: var(--muted-foreground);
  font-family: var(--font-mono, ui-monospace, monospace);
}

.card-right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 10px;
  font-weight: 500;
  padding: 2px 7px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--muted) 80%, transparent);
  color: var(--muted-foreground);
  border: 1px solid var(--border);
  white-space: nowrap;
}

.status-badge.configured {
  background: color-mix(in srgb, var(--primary) 10%, transparent);
  color: var(--primary);
  border-color: color-mix(in srgb, var(--primary) 20%, var(--border));
}

.status-badge.add-badge {
  background: transparent;
  color: var(--muted-foreground);
}

.arrow-icon {
  transition: transform 0.2s;
}

.provider-nav-card:hover .arrow-icon {
  transform: translateX(2px);
}

header.view-header.edit-header {
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  align-items: center;
  width: 100%;
  gap: 12px;
}

.header-details {
  min-width: 0;
}

.back-btn {
  font-size: 12px;
  height: 28px;
}

.header-details h2 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.edit-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.edit-title-icon {
  flex: none;
  color: var(--primary);
}

.header-details p {
  margin: 2px 0 0;
  color: var(--muted-foreground);
  font-size: 11px;
}

.delete-top-btn {
  font-size: 11px;
  height: 28px;
  margin-top: 4px;
}

.edit-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
  width: 100%;
}

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

.provider-key-link {
  display: inline;
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--primary);
  font: inherit;
  text-decoration: underline;
  text-underline-offset: 2px;
  cursor: pointer;
  word-break: break-all;
}

.provider-key-link:hover {
  color: color-mix(in srgb, var(--primary) 82%, var(--foreground));
}

.oauth-status {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: color-mix(in srgb, var(--sidebar) 55%, transparent);
}

.oauth-status-label {
  margin: 0;
  font-size: 10px;
  color: var(--muted-foreground);
}

.oauth-status-value {
  margin: 0;
  font-size: 12px;
  font-weight: 500;
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

.model-id {
  min-width: 0;
  flex: 1;
  font-family: var(--font-mono, ui-monospace, monospace);
  font-size: 11px;
  color: var(--foreground);
  overflow-wrap: anywhere;
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

.models-add-row {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}

.form-actions {
  display: flex;
  gap: 12px;
  width: 100%;
}

.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.15s ease-out;
}

.fade-slide-enter-from {
  opacity: 0;
  transform: translateX(4px);
}

.fade-slide-leave-to {
  opacity: 0;
  transform: translateX(-4px);
}
</style>
