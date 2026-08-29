<template>
  <section class="settings-page">
    <SettingsPageHeader :title="t('settings.rag.title')">
      <template #actions>
        <span class="rag-badge" :class="badgeTone">{{ badgeText }}</span>
      </template>
    </SettingsPageHeader>

    <div class="settings-group">
      <div class="settings-card">
        <article class="settings-row">
          <div class="settings-row-copy">
            <h3>
              {{ t("settings.rag.enableLabel") }}
            </h3>
            <p>{{ t("settings.rag.enableHint") }}</p>
          </div>
          <div class="settings-row-control">
            <SettingsToggle
              :model-value="form.enabled"
              :disabled="saving"
              @click.prevent="toggleEnabled"
            />
          </div>
        </article>
      </div>
    </div>

    <div class="settings-group" :class="{ 'is-disabled': !form.enabled }">
      <h3 class="settings-group-title">{{ t("settings.rag.backendLabel") }}</h3>
      <div class="settings-card">
        <article class="settings-row">
          <div class="settings-row-copy">
            <h3>{{ t("settings.rag.backendLabel") }}</h3>
          </div>
          <div class="settings-row-control">
            <div class="settings-seg">
              <button
                type="button"
                :class="{ on: form.backend === 'api' }"
                :disabled="!form.enabled || saving"
                @click="onBackendChange('api')"
              >
                {{ t("settings.rag.backendApi") }}
              </button>
              <button
                type="button"
                :class="{ on: form.backend === 'local' }"
                :disabled="!form.enabled || saving"
                @click="onBackendChange('local')"
              >
                {{ t("settings.rag.backendLocal") }}
              </button>
            </div>
          </div>
        </article>

        <template v-if="form.backend === 'api'">
          <article class="settings-row is-wide">
            <div class="settings-row-copy">
              <h3>
                {{ t("settings.rag.apiBaseUrlLabel") }}
                <SettingsHelpTip :text="t('settings.rag.apiBaseUrlHint')" />
              </h3>
            </div>
            <div class="settings-row-control">
              <Input
                v-model="form.apiBaseUrl"
                class="h-8 text-xs font-mono"
                placeholder="https://api.siliconflow.cn/v1"
                :disabled="!form.enabled || saving"
                @update:model-value="onApiEndpointChange"
              />
            </div>
          </article>

          <article class="settings-row is-wide">
            <div class="settings-row-copy">
              <h3>{{ t("settings.rag.apiKeyLabel") }}</h3>
            </div>
            <div class="settings-row-control">
              <SecretInput
                v-model="form.apiKey"
                placeholder="sk-..."
                @update:model-value="onApiEndpointChange"
              />
            </div>
          </article>

          <article class="settings-row is-wide is-top">
            <div class="settings-row-copy">
              <h3>{{ t("settings.rag.apiModelLabel") }}</h3>
            </div>
            <div class="settings-row-control is-stack">
              <Select
                v-if="modelSelectOptions.length > 0"
                :model-value="form.apiModel"
                :disabled="!form.enabled || saving"
                @update:model-value="onApiModelSelect"
              >
                <SelectTrigger class="w-full">
                  <SelectValue :placeholder="t('settings.rag.apiModelPlaceholder')" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem v-for="id in modelSelectOptions" :key="id" :value="id">
                    {{ id }}
                  </SelectItem>
                </SelectContent>
              </Select>
              <Input
                v-else
                v-model="form.apiModel"
                class="h-8 text-xs font-mono"
                :placeholder="t('settings.rag.apiModelPlaceholder')"
                :disabled="!form.enabled || saving"
              />
              <div class="rag-inline-actions">
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  class="h-7 gap-1 text-xs"
                  :disabled="!form.enabled || fetchingModels || !canFetchModels || saving"
                  @click="fetchModels"
                >
                  <RefreshCw class="size-3.5" :class="{ spinning: fetchingModels }" />
                  {{
                    fetchingModels
                      ? t("settings.rag.fetchingModels")
                      : t("settings.rag.fetchModels")
                  }}
                </Button>
                <span v-if="fetchNotice" class="rag-inline-msg" :class="fetchNotice.tone">
                  {{ fetchNotice.text }}
                </span>
              </div>
            </div>
          </article>

          <article class="settings-row">
            <div class="settings-row-copy">
              <h3>{{ t("settings.rag.testConnection") }}</h3>
            </div>
            <div class="settings-row-control is-stack">
              <Button
                type="button"
                size="sm"
                variant="outline"
                class="h-8 gap-1.5 self-end"
                :disabled="!form.enabled || testing || !canTest || saving"
                @click="testConnection"
              >
                <Plug class="size-3.5" />
                {{ testing ? t("settings.rag.testing") : t("settings.rag.testConnection") }}
              </Button>
              <span v-if="testNotice" class="rag-inline-msg" :class="testNotice.tone">
                {{ testNotice.text }}
              </span>
            </div>
          </article>
        </template>

        <article v-else class="settings-row is-wide">
          <div class="settings-row-copy">
            <h3>
              {{ t("settings.rag.modelLabel") }}
              <SettingsHelpTip :text="t('settings.rag.modelHint')" />
            </h3>
          </div>
          <div class="settings-row-control">
            <Select
              :model-value="form.model"
              :disabled="!form.enabled || saving"
              @update:model-value="onModelChange"
            >
              <SelectTrigger class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="o in modelOptions" :key="o.value" :value="o.value">
                  {{ o.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </article>
      </div>
    </div>

    <div v-if="engineNotice" class="rag-banner" :class="engineNotice.tone">
      <LoaderCircle v-if="status.status === 'downloading'" :size="14" class="spinning" />
      <CheckCircle2 v-else-if="engineNotice.tone === 'ok'" :size="14" />
      <CircleAlert v-else :size="14" />
      <span>{{ engineNotice.text }}</span>
    </div>

    <div class="rag-actions">
      <Button size="sm" class="h-8 gap-1.5" :disabled="!canSave" @click="save">
        <Save class="size-3.5" />
        {{ saving ? t("settings.rag.saving") : t("settings.rag.save") }}
      </Button>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { CheckCircle2, CircleAlert, LoaderCircle, Plug, RefreshCw, Save } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SecretInput } from "@/components/ui/secret-input";
import SettingsHelpTip from "@/components/settings/SettingsHelpTip.vue";
import SettingsPageHeader from "@/components/settings/SettingsPageHeader.vue";
import SettingsToggle from "@/components/settings/SettingsToggle.vue";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  fetchSemanticSearchModels,
  getSemanticSearchStatus,
  setSemanticSearch,
  testSemanticSearchApi,
} from "@/services/ipc";
import { tr } from "@/services/i18n";
import type { SettingsI18nKey } from "@/services/locales/settings";
import { useSettingStore } from "@/stores/setting";
import { localizedOptionLabel, semanticSearchModelOptions } from "@/types/setting";
import type {
  SemanticSearchBackend,
  SemanticSearchConfig,
  SemanticSearchModel,
  SemanticSearchState,
} from "@/types/setting";

type Notice = { tone: "ok" | "warn" | "error"; text: string };

const settingStore = useSettingStore();

const form = reactive({
  enabled: false,
  backend: "api" as SemanticSearchBackend,
  model: "multilingual-e5-small" as SemanticSearchModel,
  apiBaseUrl: "",
  apiKey: "",
  apiModel: "",
});

const status = ref<SemanticSearchState>({ status: "idle" });
const saving = ref(false);
const testing = ref(false);
const fetchingModels = ref(false);
const fetchedModels = ref<string[]>([]);
const lastFetchEndpoint = ref("");
const fetchNotice = ref<Notice | null>(null);
const testNotice = ref<Notice | null>(null);
let pollTimer: ReturnType<typeof setInterval> | null = null;

function endpointKey() {
  return `${form.apiBaseUrl.trim()}|${form.apiKey}`;
}

const t = (key: SettingsI18nKey, values?: Record<string, string | number>) =>
  tr(settingStore.language, key, values ?? {});

const modelOptions = computed(() =>
  semanticSearchModelOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const persisted = computed((): SemanticSearchConfig => ({
  enabled: settingStore.semanticSearchEnabled,
  backend: settingStore.semanticSearchBackend,
  model: settingStore.semanticSearchModel,
  apiBaseUrl: settingStore.semanticSearchApiBaseUrl,
  apiKey: settingStore.semanticSearchApiKey,
  apiModel: settingStore.semanticSearchApiModel,
}));

const dirty = computed(() => {
  const saved = persisted.value;
  return (
    form.enabled !== saved.enabled ||
    form.backend !== saved.backend ||
    form.model !== saved.model ||
    form.apiBaseUrl.trim() !== saved.apiBaseUrl.trim() ||
    form.apiKey !== saved.apiKey ||
    form.apiModel.trim() !== saved.apiModel.trim()
  );
});

const canFetchModels = computed(() => !!form.apiBaseUrl.trim() && !!form.apiKey.trim());
const canTest = computed(() => canFetchModels.value && !!form.apiModel.trim());
const apiComplete = computed(() => form.backend !== "api" || canTest.value);
const canSave = computed(() => form.enabled && dirty.value && !saving.value && apiComplete.value);

const modelSelectOptions = computed(() => {
  const ids = [...fetchedModels.value];
  const current = form.apiModel.trim();
  if (current && !ids.includes(current)) ids.unshift(current);
  return ids;
});

const badgeText = computed(() => {
  if (!form.enabled && !persisted.value.enabled) return t("settings.rag.idle");
  if (status.value.status === "downloading") return t("settings.rag.downloading");
  if (dirty.value) return t("settings.rag.unsaved");
  switch (status.value.status) {
    case "ready":
      return t("settings.rag.ready");
    case "error":
      return t("settings.rag.error");
    default:
      return form.enabled ? t("settings.rag.on") : t("settings.rag.idle");
  }
});

const badgeTone = computed(() => {
  if (status.value.status === "downloading") return "warn";
  if (dirty.value) return "warn";
  if (!form.enabled) return "muted";
  switch (status.value.status) {
    case "ready":
      return "ok";
    case "error":
      return "error";
    default:
      return "ok";
  }
});

const engineNotice = computed<Notice | null>(() => {
  if (status.value.status === "downloading") {
    return { tone: "warn", text: t("settings.rag.downloading") };
  }
  if (status.value.status === "error") {
    return { tone: "error", text: status.value.message || t("settings.rag.error") };
  }
  if (form.enabled && form.backend === "api" && !apiComplete.value) {
    return { tone: "warn", text: t("settings.rag.incomplete") };
  }
  return null;
});

function onBackendChange(value: SemanticSearchBackend) {
  if (form.backend === value) return;
  form.backend = value;
  testNotice.value = null;
}

function onModelChange(value: unknown) {
  if (typeof value === "string") {
    form.model = value as SemanticSearchModel;
  }
}

function onApiModelSelect(value: unknown) {
  if (typeof value === "string") form.apiModel = value;
}

function onApiEndpointChange() {
  if (endpointKey() !== lastFetchEndpoint.value) {
    fetchedModels.value = [];
  }
  fetchNotice.value = null;
  testNotice.value = null;
}

function toConfig(): SemanticSearchConfig {
  return {
    enabled: form.enabled,
    backend: form.backend,
    model: form.model,
    apiBaseUrl: form.apiBaseUrl.trim(),
    apiKey: form.apiKey.trim(),
    apiModel: form.apiModel.trim(),
  };
}

function syncForm(config: SemanticSearchConfig) {
  form.enabled = config.enabled;
  form.backend = config.backend;
  form.model = config.model;
  form.apiBaseUrl = config.apiBaseUrl;
  form.apiKey = config.apiKey;
  form.apiModel = config.apiModel;
}

async function apply(next: Partial<SemanticSearchConfig>) {
  const previous = { ...persisted.value };
  const config = { ...toConfig(), ...next };
  syncForm(config);
  saving.value = true;
  try {
    status.value = await setSemanticSearch(config);
    settingStore.semanticSearchEnabled = config.enabled;
    settingStore.semanticSearchBackend = config.backend;
    settingStore.semanticSearchModel = config.model;
    settingStore.semanticSearchApiBaseUrl = config.apiBaseUrl;
    settingStore.semanticSearchApiKey = config.apiKey;
    settingStore.semanticSearchApiModel = config.apiModel;
    schedulePoll();
  } catch (error) {
    syncForm(previous);
    status.value = { status: "error", message: String(error) };
  } finally {
    saving.value = false;
  }
}

async function save() {
  if (!canSave.value) return;
  await apply({});
}

async function toggleEnabled() {
  if (saving.value) return;
  const next = !form.enabled;
  if (!next) {
    await apply({ enabled: false });
    return;
  }
  form.enabled = true;
  const saved = persisted.value;
  const matchesSaved =
    form.backend === saved.backend &&
    form.model === saved.model &&
    form.apiBaseUrl.trim() === saved.apiBaseUrl.trim() &&
    form.apiKey === saved.apiKey &&
    form.apiModel.trim() === saved.apiModel.trim();
  if (matchesSaved && apiComplete.value) {
    await apply({ enabled: true });
  }
}

async function testConnection() {
  testing.value = true;
  testNotice.value = null;
  try {
    const result = await testSemanticSearchApi(
      form.apiBaseUrl.trim(),
      form.apiKey.trim(),
      form.apiModel.trim(),
    );
    testNotice.value = {
      tone: "ok",
      text: t("settings.rag.testOkDetail", { dim: result.dim }),
    };
  } catch (error) {
    testNotice.value = { tone: "error", text: String(error) };
  } finally {
    testing.value = false;
  }
}

async function fetchModels() {
  fetchingModels.value = true;
  fetchNotice.value = null;
  try {
    const models = await fetchSemanticSearchModels(form.apiBaseUrl.trim(), form.apiKey.trim());
    fetchedModels.value = models;
    lastFetchEndpoint.value = endpointKey();
    fetchNotice.value = {
      tone: "ok",
      text: t("settings.rag.fetchOk", { count: models.length }),
    };
    if (!form.apiModel.trim() && models[0]) {
      form.apiModel = models[0];
    }
  } catch (error) {
    fetchedModels.value = [];
    fetchNotice.value = {
      tone: "error",
      text: `${t("settings.rag.fetchFail")}：${String(error)}`,
    };
  } finally {
    fetchingModels.value = false;
  }
}

function schedulePoll() {
  if (status.value.status === "downloading") {
    if (!pollTimer) {
      pollTimer = setInterval(async () => {
        status.value = await getSemanticSearchStatus();
        if (status.value.status !== "downloading" && pollTimer) {
          clearInterval(pollTimer);
          pollTimer = null;
        }
      }, 1500);
    }
  } else if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

watch(
  () => form.backend,
  () => {
    testNotice.value = null;
  },
);

onMounted(async () => {
  syncForm(persisted.value);
  status.value = await getSemanticSearchStatus();
  schedulePoll();
});

onUnmounted(() => {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
});
</script>

<style scoped>
.is-disabled {
  opacity: 0.55;
  pointer-events: none;
}

.rag-badge {
  display: inline-flex;
  align-items: center;
  height: 22px;
  padding: 0 8px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  background: color-mix(in srgb, var(--peek-text) 6%, transparent);
  color: var(--peek-muted);
}

.rag-badge.ok {
  color: var(--peek-success, #18794e);
  background: color-mix(in srgb, var(--peek-success, #18794e) 12%, transparent);
}

.rag-badge.warn {
  color: var(--peek-warning, #8a6500);
  background: color-mix(in srgb, var(--peek-warning, #8a6500) 12%, transparent);
}

.rag-badge.error {
  color: var(--peek-danger, #c42b1c);
  background: color-mix(in srgb, var(--peek-danger, #c42b1c) 12%, transparent);
}

.rag-inline-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.rag-inline-msg {
  font-size: 11px;
  line-height: 1.4;
}

.rag-inline-msg.ok {
  color: var(--peek-success, #18794e);
}

.rag-inline-msg.warn {
  color: var(--peek-warning, #8a6500);
}

.rag-inline-msg.error {
  color: var(--peek-danger, #c42b1c);
}

.rag-banner {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin: 0 0 14px;
  padding: 9px 12px;
  border-radius: 8px;
  font-size: 12px;
  line-height: 1.45;
}

.rag-banner.ok {
  color: var(--peek-success, #18794e);
  background: color-mix(in srgb, var(--peek-success, #18794e) 10%, transparent);
}

.rag-banner.warn {
  color: var(--peek-warning, #8a6500);
  background: color-mix(in srgb, var(--peek-warning, #8a6500) 10%, transparent);
}

.rag-banner.error {
  color: var(--peek-danger, #c42b1c);
  background: color-mix(in srgb, var(--peek-danger, #c42b1c) 10%, transparent);
}

.rag-actions {
  display: flex;
  justify-content: flex-end;
}

.self-end {
  align-self: flex-end;
}

.spinning {
  animation: rag-spin 0.8s linear infinite;
}

@keyframes rag-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
