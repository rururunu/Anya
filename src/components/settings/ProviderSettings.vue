<template>
  <section class="settings-page">
    <AppConfirmDialog ref="confirmDialogRef" />

    <Transition name="fade-slide" mode="out-in">
      <div v-if="currentView === 'list'" key="list" class="view-container">
        <SettingsPageHeader :title="t('settings.provider.title')" />

        <div class="settings-nav-list">
          <div class="provider-block">
            <button type="button" class="settings-nav-row" @click="startEditDeepSeek">
              <div class="settings-nav-row-left">
                <div class="settings-nav-row-icon">
                  <DeepSeekIcon :size="18" />
                </div>
                <div class="settings-nav-row-copy">
                  <h3>{{ t("settings.provider.deepseek") }}</h3>
                  <p>DeepSeek API</p>
                </div>
              </div>
              <div class="settings-nav-row-right">
                <span
                  class="settings-status-badge"
                  :class="{ 'is-configured': isDeepSeekConfigured }"
                >
                  {{ statusLabel(isDeepSeekConfigured) }}
                </span>
                <ChevronRight class="size-4 text-muted-foreground arrow-icon" />
              </div>
            </button>
            <DeepSeekBalanceCard
              variant="row"
              :report="deepseekBalance"
              :loading="deepseekBalanceLoading"
              :error="deepseekBalanceError"
              :language="settingStore.language"
              :copy="deepseekBalanceCopy"
              @refresh="loadDeepSeekBalance"
            />
          </div>

          <button
            v-for="provider in settingStore.customProviders"
            :key="provider.id"
            type="button"
            class="settings-nav-row"
            @click="startEditCustom(provider.id)"
          >
            <div class="settings-nav-row-left">
              <div class="settings-nav-row-icon">
                <img
                  v-if="peekProviderFavicon(provider.id)"
                  :src="peekProviderFavicon(provider.id)!"
                  alt=""
                  @error="markProviderFaviconBroken(provider.id)"
                />
                <Globe2 v-else class="size-5" />
              </div>
              <div class="settings-nav-row-copy">
                <h3>{{ provider.name }}</h3>
                <p class="truncate max-w-[280px]">{{ customProviderSubtitle(provider) }}</p>
              </div>
            </div>
            <div class="settings-nav-row-right">
              <span
                class="settings-status-badge"
                :class="{ 'is-configured': isCustomConfigured(provider) }"
              >
                {{ statusLabel(isCustomConfigured(provider)) }}
              </span>
              <ChevronRight class="size-4 text-muted-foreground arrow-icon" />
            </div>
          </button>
        </div>

        <div class="add-section">
          <h4 class="settings-section-label">{{ t("settings.provider.presets") }}</h4>
          <div class="settings-nav-list">
            <button type="button" class="settings-nav-row is-add" @click="addBlankCustomProvider">
              <div class="settings-nav-row-left">
                <div class="settings-nav-row-icon is-muted">
                  <Globe2 class="size-5" />
                </div>
                <div class="settings-nav-row-copy">
                  <h3>{{ t("settings.provider.addBlank") }}</h3>
                  <p>OpenAI-compatible API</p>
                </div>
              </div>
              <div class="settings-nav-row-right">
                <span class="settings-status-badge is-add">
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

        <div class="edit-form">
          <div class="field-row">
            <label>{{ t("settings.provider.apiKey") }}</label>
            <SecretInput v-model="deepseekKey" placeholder="sk-..." @blur="onDeepSeekKeyBlur" />
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

          <ProviderModelList
            :models="deepseekModelIds"
            :disabled-models="deepseekDisabledIds"
            :fetching="fetchingDeepSeekModels"
            :can-fetch="canFetchDeepSeekModels"
            :fetch-error="fetchDeepSeekModelsError"
            @fetch="fetchDeepSeekRemoteModels"
            @add="addDeepSeekModel"
            @remove="removeDeepSeekModel"
            @toggle-disabled="toggleDeepSeekModelDisabled"
          />

          <DeepSeekFilesPanel />

          <p
            class="provider-save-status"
            :class="{ 'is-error': saveState === 'error' }"
            role="status"
            aria-live="polite"
          >
            {{ saveStatusLabel }}
          </p>
          <div class="form-actions">
            <Button size="sm" class="h-8 w-full gap-1.5" @click="saveDeepSeekAndGoBack">
              <Check class="size-3.5" />
              {{ t("settings.provider.done") }}
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
              <img
                v-if="editingProviderId && peekProviderFavicon(editingProviderId)"
                :src="peekProviderFavicon(editingProviderId)!"
                alt=""
                class="favicon-img edit-title-icon"
                @error="markProviderFaviconBroken(editingProviderId)"
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

        <div class="edit-form">
          <div class="field-row">
            <label>{{ t("settings.provider.name") }}</label>
            <Input
              v-model="customName"
              :placeholder="t('settings.provider.namePlaceholder')"
              class="h-9 text-sm"
              @blur="saveCustom"
            />
          </div>

          <div class="field-row">
            <label>{{ t("settings.provider.baseUrl") }}</label>
            <Input
              v-model="customUrl"
              :placeholder="t('settings.provider.urlPlaceholder')"
              class="h-9 text-sm font-mono"
              @blur="saveCustom"
            />
          </div>

          <div class="field-row">
            <label>{{ t("settings.provider.websiteUrl") }}</label>
            <Input
              v-model="customWebsiteUrl"
              :placeholder="t('settings.provider.websiteUrlPlaceholder')"
              class="h-9 text-sm"
              @blur="saveCustom"
            />
          </div>

          <div class="field-row">
            <label>{{ t("settings.provider.apiProtocol") }}</label>
            <Select :model-value="customApiProtocol" @update:model-value="onProtocolChange">
              <SelectTrigger class="h-9 w-full text-sm">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="chatCompletions">
                  {{ t("settings.provider.apiProtocolChatCompletions") }}
                </SelectItem>
                <SelectItem value="responses">
                  {{ t("settings.provider.apiProtocolResponses") }}
                </SelectItem>
                <SelectItem value="anthropicMessages">
                  {{ t("settings.provider.apiProtocolAnthropic") }}
                </SelectItem>
              </SelectContent>
            </Select>
            <p class="field-hint">{{ protocolHint }}</p>
          </div>

          <div class="field-row">
            <label>{{ t("settings.provider.apiKey") }}</label>
            <SecretInput v-model="customKey" placeholder="sk-..." @blur="saveCustom" />
          </div>

          <ProviderModelList
            :models="customModelList"
            :disabled-models="customDisabledModels"
            :fetching="fetchingModels"
            :can-fetch="canFetchModels"
            :fetch-error="fetchModelsError"
            :supports-protocol-select="true"
            :model-protocols="customModelProtocols"
            @fetch="fetchRemoteModels"
            @add="addCustomModel"
            @remove="removeCustomModel"
            @toggle-disabled="toggleModelDisabled"
            @protocol-change="onModelProtocolChange"
          />

          <p
            class="provider-save-status"
            :class="{ 'is-error': saveState === 'error' }"
            role="status"
            aria-live="polite"
          >
            {{ saveStatusLabel }}
          </p>
          <div class="form-actions">
            <Button size="sm" class="h-8 w-full gap-1.5" @click="saveCustomAndGoBack">
              <Check class="size-3.5" />
              {{ t("settings.provider.done") }}
            </Button>
          </div>
        </div>
      </div>
    </Transition>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Globe2, ChevronLeft, ChevronRight, Plus, Trash2, Check } from "@lucide/vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import DeepSeekIcon from "@/components/icons/DeepSeekIcon.vue";
import ProviderModelList from "@/components/settings/ProviderModelList.vue";
import DeepSeekFilesPanel from "@/components/settings/DeepSeekFilesPanel.vue";
import DeepSeekBalanceCard from "@/components/settings/DeepSeekBalanceCard.vue";
import { useSettingStore } from "@/stores/setting";
import { useChatModelStore } from "@/stores/chatModel";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SecretInput } from "@/components/ui/secret-input";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import SettingsPageHeader from "@/components/settings/SettingsPageHeader.vue";
import { listCustomProviderModels, listDeepSeekModels, getDeepSeekBalance } from "@/services/ipc";
import { tr } from "@/services/i18n";
import type { SettingsI18nKey } from "@/services/locales/settings";
import type {
  CustomProviderConfig,
  ModelWireProtocol,
  ProviderApiProtocol,
  ProviderModelEntry,
} from "@/types/setting";
import type { DeepSeekBalanceReport } from "@/types/tokenUsage";
import {
  DEFAULT_PROVIDER_API_PROTOCOL,
  normalizeModelProtocol,
  normalizeProviderApiProtocol,
} from "@/types/setting";
import {
  isCustomProviderConfigured,
  looksLikeHttpUrl,
  parseProviderModels,
  serializeProviderModels,
  syncRemoteModels,
} from "@/lib/providerPresets";
import {
  ensureProviderFavicon,
  markProviderFaviconBroken,
  peekProviderFavicon,
  warmProviderFavicons,
} from "@/services/providerFavicon";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

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

const currentView = ref<"list" | "deepseek" | "custom">("list");
const saveState = ref<"saved" | "saving" | "error">("saved");
const saveError = ref("");
const saveStatusLabel = computed(() =>
  saveState.value === "saving"
    ? t("settings.provider.saving")
    : saveState.value === "error"
      ? `${t("settings.provider.saveFailed")}${saveError.value ? `: ${saveError.value}` : ""}`
      : t("settings.provider.autoSaved"),
);
let saveQueue: Promise<void> = Promise.resolve();
let pendingSaves = 0;

async function queueProviderSave(task: () => Promise<void>): Promise<boolean> {
  pendingSaves += 1;
  saveState.value = "saving";
  saveError.value = "";
  const run = saveQueue.then(task);
  saveQueue = run.then(
    () => undefined,
    () => undefined,
  );
  try {
    await run;
    pendingSaves -= 1;
    if (pendingSaves === 0) saveState.value = "saved";
    return true;
  } catch (error) {
    pendingSaves -= 1;
    saveError.value = error instanceof Error ? error.message : String(error);
    saveState.value = "error";
    return false;
  }
}
const editingProviderId = ref<string | null>(null);
const customPresetId = ref<string | undefined>(undefined);

const deepseekKey = ref(settingStore.deepseekApiKey);
const deepseekModels = ref<ProviderModelEntry[]>(cloneModelEntries(settingStore.deepseekModels));
const deepseekModelIds = computed(() => deepseekModels.value.map((entry) => entry.id));
const deepseekDisabledIds = computed(
  () => new Set(deepseekModels.value.filter((entry) => entry.disabled).map((entry) => entry.id)),
);
const fetchingDeepSeekModels = ref(false);
const fetchDeepSeekModelsError = ref("");

function cloneModelEntries(entries: ProviderModelEntry[]): ProviderModelEntry[] {
  return entries.map((entry) => ({ ...entry }));
}

const fetchingModels = ref(false);
const fetchModelsError = ref("");

const customName = ref("");
const customUrl = ref("");
const customWebsiteUrl = ref("");
const customKey = ref("");
const customApiProtocol = ref<ProviderApiProtocol>(DEFAULT_PROVIDER_API_PROTOCOL);
const customModelList = ref<string[]>([]);
const customDisabledModels = ref<Set<string>>(new Set());
const customModelProtocols = ref<Record<string, ModelWireProtocol>>({});

const protocolHint = computed(() => {
  if (
    customApiProtocol.value === "chatCompletions" &&
    /(?:^|\.)x\.ai(?:[:/?#]|$)/i.test(customUrl.value.trim())
  ) {
    return t("settings.provider.apiProtocolGrokHint");
  }
  return t("settings.provider.apiProtocolHint");
});

watch(
  () => settingStore.customProviders,
  (providers) => {
    warmProviderFavicons(providers);
  },
  { immediate: true, deep: true },
);

watch(
  [editingProviderId, customUrl, customWebsiteUrl],
  ([id, url, websiteUrl]) => {
    if (!id) return;
    ensureProviderFavicon(id, url, websiteUrl);
  },
  { immediate: true },
);

const canFetchModels = computed(
  () => looksLikeHttpUrl(customUrl.value) && !!customKey.value.trim(),
);

const canFetchDeepSeekModels = computed(() => !!deepseekKey.value.trim());

const isDeepSeekConfigured = computed(() => {
  return !!settingStore.deepseekApiKey.trim();
});

const deepseekBalance = ref<DeepSeekBalanceReport | null>(null);
const deepseekBalanceLoading = ref(false);
const deepseekBalanceError = ref("");
const deepseekBalanceCopy = computed(() => ({
  title: tr(settingStore.language, "usage.balance.title"),
  available: tr(settingStore.language, "usage.balance.available"),
  granted: tr(settingStore.language, "usage.balance.granted"),
  toppedUp: tr(settingStore.language, "usage.balance.toppedUp"),
  ready: tr(settingStore.language, "usage.balance.ready"),
  empty: tr(settingStore.language, "usage.balance.empty"),
  unconfigured: tr(settingStore.language, "usage.balance.unconfigured"),
  error: tr(settingStore.language, "usage.balance.error"),
  loading: tr(settingStore.language, "usage.balance.loading"),
  refresh: tr(settingStore.language, "usage.balance.refresh"),
}));

async function loadDeepSeekBalance() {
  deepseekBalanceLoading.value = true;
  deepseekBalanceError.value = "";
  try {
    deepseekBalance.value = await getDeepSeekBalance();
  } catch (cause) {
    deepseekBalanceError.value = String(cause);
  } finally {
    deepseekBalanceLoading.value = false;
  }
}

watch(
  [currentView, isDeepSeekConfigured],
  ([view]) => {
    if (view === "list") void loadDeepSeekBalance();
  },
  { immediate: true },
);

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

function isCustomConfigured(provider: CustomProviderConfig) {
  return isCustomProviderConfigured(provider);
}

function toggleModelDisabled(model: string) {
  const next = new Set(customDisabledModels.value);
  if (next.has(model)) {
    next.delete(model);
  } else {
    next.add(model);
  }
  customDisabledModels.value = next;
  void saveCustom();
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

function startEditDeepSeek() {
  saveState.value = "saved";
  deepseekKey.value = settingStore.deepseekApiKey;
  deepseekModels.value = cloneModelEntries(settingStore.deepseekModels);
  fetchDeepSeekModelsError.value = "";
  currentView.value = "deepseek";

  // If no saved models, auto-fetch once from online if API key is configured.
  if (deepseekModels.value.length === 0 && canFetchDeepSeekModels.value) {
    void fetchDeepSeekRemoteModels();
  }
}

async function onDeepSeekKeyBlur() {
  if (!(await saveDeepSeek())) return;
  if (deepseekModels.value.length === 0 && canFetchDeepSeekModels.value) {
    void fetchDeepSeekRemoteModels();
  }
}

function toggleDeepSeekModelDisabled(model: string) {
  const entry = deepseekModels.value.find((item) => item.id === model);
  if (!entry) return;
  entry.disabled = !entry.disabled;
  void saveDeepSeek();
}

async function addDeepSeekModel(modelId: string) {
  const ids = parseProviderModels(modelId);
  if (ids.length === 0) return;
  for (const id of ids) {
    const existing = deepseekModels.value.find((entry) => entry.id === id);
    if (existing) {
      // Re-adding an existing id marks it as user-added and re-enables it.
      existing.custom = true;
      existing.disabled = false;
    } else {
      // User-added models start enabled.
      deepseekModels.value.push({ id, disabled: false, custom: true });
    }
  }
  await saveDeepSeek();
}

async function removeDeepSeekModel(index: number) {
  deepseekModels.value.splice(index, 1);
  await saveDeepSeek();
}

async function fetchDeepSeekRemoteModels() {
  fetchDeepSeekModelsError.value = "";
  if (!canFetchDeepSeekModels.value) {
    fetchDeepSeekModelsError.value = t("settings.provider.fetchModelsFailed");
    return;
  }
  fetchingDeepSeekModels.value = true;
  try {
    const remoteIds = await listDeepSeekModels(deepseekKey.value.trim());
    if (remoteIds.length === 0) {
      fetchDeepSeekModelsError.value = t("settings.provider.fetchModelsFailed");
      return;
    }
    deepseekModels.value = syncRemoteModels(deepseekModels.value, remoteIds);
    await saveDeepSeek();
  } catch (error) {
    fetchDeepSeekModelsError.value =
      error instanceof Error ? error.message : t("settings.provider.fetchModelsFailed");
  } finally {
    fetchingDeepSeekModels.value = false;
  }
}

async function saveDeepSeek() {
  const nextKey = deepseekKey.value.trim();
  const nextModels = cloneModelEntries(deepseekModels.value);
  return queueProviderSave(async () => {
    if (
      nextKey === settingStore.deepseekApiKey &&
      JSON.stringify(nextModels) === JSON.stringify(settingStore.deepseekModels)
    ) {
      return;
    }

    await settingStore.update({
      deepseekApiKey: nextKey,
      deepseekModels: nextModels,
    });
    await chatModelStore.refresh();
  });
}

async function saveDeepSeekAndGoBack() {
  if (await saveDeepSeek()) currentView.value = "list";
}

function startEditCustom(id: string) {
  saveState.value = "saved";
  const provider = settingStore.customProviders.find((p) => p.id === id);
  if (provider) {
    editingProviderId.value = id;
    customPresetId.value = provider.presetId;
    customName.value = provider.name;
    customUrl.value = provider.baseUrl;
    customWebsiteUrl.value = provider.websiteUrl ?? "";
    customKey.value = provider.apiKey;
    customApiProtocol.value = normalizeProviderApiProtocol(provider.apiProtocol);
    customModelList.value = parseProviderModels(provider.models);
    customDisabledModels.value = new Set(parseProviderModels(provider.disabledModels ?? ""));
    customModelProtocols.value = cloneModelProtocols(provider.modelProtocols);
    fetchModelsError.value = "";
    currentView.value = "custom";
  }
}

function newProviderId() {
  return Math.random().toString(36).substring(2, 11);
}

function addBlankCustomProvider() {
  saveState.value = "saved";
  editingProviderId.value = newProviderId();
  customPresetId.value = undefined;
  customName.value = "";
  customUrl.value = "";
  customWebsiteUrl.value = "";
  customKey.value = "";
  customApiProtocol.value = DEFAULT_PROVIDER_API_PROTOCOL;
  customModelList.value = [];
  customDisabledModels.value = new Set();
  customModelProtocols.value = {};
  fetchModelsError.value = "";
  currentView.value = "custom";
}

async function addCustomModel(modelId: string) {
  const ids = parseProviderModels(modelId);
  if (ids.length === 0) return;
  const next = [...customModelList.value];
  const nextDisabled = new Set(customDisabledModels.value);
  for (const id of ids) {
    if (!next.includes(id)) {
      next.push(id);
      // New models start disabled; the user opts in per model.
      nextDisabled.add(id);
    }
  }
  customModelList.value = next;
  customDisabledModels.value = nextDisabled;
  await saveCustom();
}

async function removeCustomModel(index: number) {
  const removed = customModelList.value[index];
  customModelList.value = customModelList.value.filter((_, i) => i !== index);
  if (removed && customDisabledModels.value.has(removed)) {
    const next = new Set(customDisabledModels.value);
    next.delete(removed);
    customDisabledModels.value = next;
  }
  if (removed && customModelProtocols.value[removed]) {
    const next = { ...customModelProtocols.value };
    delete next[removed];
    customModelProtocols.value = next;
  }
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
    const nextDisabled = new Set(customDisabledModels.value);
    for (const id of ids) {
      if (!merged.includes(id)) {
        merged.push(id);
        // New models start disabled; the user opts in per model.
        nextDisabled.add(id);
      }
    }
    customModelList.value = merged;
    customDisabledModels.value = nextDisabled;
    await saveCustom();
  } catch (error) {
    fetchModelsError.value =
      error instanceof Error ? error.message : t("settings.provider.fetchModelsFailed");
  } finally {
    fetchingModels.value = false;
  }
}

async function saveCustom() {
  const providerId = editingProviderId.value;
  if (!providerId) return false;
  const nextName = customName.value.trim() || `Custom - ${providerId}`;
  const nextUrl = customUrl.value.trim();
  const nextWebsiteUrl = customWebsiteUrl.value.trim();
  const nextKey = customKey.value.trim();
  const nextModels = serializeProviderModels(customModelList.value);
  const nextDisabledModels = serializeProviderModels([...customDisabledModels.value]);
  const nextPresetId = customPresetId.value;
  const nextProtocol = customApiProtocol.value;
  const nextModelProtocols = serializeModelProtocols(
    customModelProtocols.value,
    customModelList.value,
  );
  return queueProviderSave(async () => {
    const list = [...settingStore.customProviders];
    const index = list.findIndex((p) => p.id === providerId);

    const updatedProvider: CustomProviderConfig = {
      id: providerId,
      name: nextName,
      baseUrl: nextUrl,
      websiteUrl: nextWebsiteUrl || undefined,
      apiKey: nextKey,
      models: nextModels,
      disabledModels: nextDisabledModels,
      presetId: nextPresetId,
      apiProtocol: nextProtocol,
      modelProtocols: nextModelProtocols,
    };

    if (index !== -1) {
      const current = list[index];
      if (
        current.name === nextName &&
        current.baseUrl === nextUrl &&
        (current.websiteUrl ?? "") === nextWebsiteUrl &&
        current.apiKey === nextKey &&
        current.models === nextModels &&
        (current.disabledModels ?? "") === nextDisabledModels &&
        current.presetId === nextPresetId &&
        normalizeProviderApiProtocol(current.apiProtocol) === nextProtocol &&
        modelProtocolsEqual(current.modelProtocols, nextModelProtocols)
      ) {
        return;
      }
      list[index] = updatedProvider;
    } else {
      list.push(updatedProvider);
    }

    await settingStore.update({ customProviders: list });
    await chatModelStore.refresh();
  });
}

async function saveCustomAndGoBack() {
  if (await saveCustom()) currentView.value = "list";
}

function onProtocolChange(value: unknown) {
  customApiProtocol.value = normalizeProviderApiProtocol(value);
  void saveCustom();
}

function onModelProtocolChange(model: string, value: unknown) {
  const next = { ...customModelProtocols.value };
  const protocol = normalizeModelProtocol(value);
  if (protocol) {
    next[model] = protocol;
  } else {
    delete next[model];
  }
  customModelProtocols.value = next;
  void saveCustom();
}

function cloneModelProtocols(
  source?: Record<string, ModelWireProtocol>,
): Record<string, ModelWireProtocol> {
  const next: Record<string, ModelWireProtocol> = {};
  if (!source) return next;
  for (const [model, protocol] of Object.entries(source)) {
    const normalized = normalizeModelProtocol(protocol);
    if (normalized) next[model] = normalized;
  }
  return next;
}

function serializeModelProtocols(
  source: Record<string, ModelWireProtocol>,
  models: string[],
): Record<string, ModelWireProtocol> | undefined {
  const next: Record<string, ModelWireProtocol> = {};
  for (const model of models) {
    const protocol = source[model];
    if (protocol) next[model] = protocol;
  }
  return Object.keys(next).length > 0 ? next : undefined;
}

function modelProtocolsEqual(
  left?: Record<string, ModelWireProtocol>,
  right?: Record<string, ModelWireProtocol>,
): boolean {
  const a = left ?? {};
  const b = right ?? {};
  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  for (const key of keys) {
    if (a[key] !== b[key]) return false;
  }
  return true;
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

  await saveQueue;
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
  gap: 20px;
  width: 100%;
}

.view-container > .settings-page-header {
  margin-bottom: 0;
}

.view-container > .settings-nav-list :deep(.settings-nav-row),
.add-section .settings-nav-row {
  padding: 14px 16px;
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
  gap: 10px;
  padding-top: 8px;
}

.provider-block {
  display: flex;
  flex-direction: column;
}

.arrow-icon {
  transition: transform 0.2s;
}

.settings-nav-row:hover .arrow-icon {
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
  font-size: 13px;
  height: 32px;
}

header.view-header .header-details h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 700;
}

.edit-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.edit-title-icon {
  flex: none;
  color: var(--primary);
}

.header-details p {
  margin: 4px 0 0;
  color: var(--muted-foreground);
  font-size: 12px;
}

.delete-top-btn {
  font-size: 11px;
  height: 28px;
  margin-top: 4px;
}

.edit-form {
  display: flex;
  flex-direction: column;
  gap: 20px;
  width: 100%;
  padding-top: 20px;
  border-top: 1px solid var(--peek-border);
}

.field-row {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.field-row label {
  font-size: 12px;
  font-weight: 600;
  color: var(--peek-text);
}

.field-hint {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  color: var(--muted-foreground);
}

.edit-form :deep(input) {
  min-height: 36px;
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

.form-actions {
  display: flex;
  gap: 12px;
  width: 100%;
}

.provider-save-status {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  color: var(--muted-foreground);
}

.provider-save-status.is-error {
  color: var(--destructive);
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
