<template>
  <section class="settings-page">
    <AppConfirmDialog ref="confirmDialogRef" />

    <Transition name="fade-slide" mode="out-in">
      <div v-if="currentView === 'list'" key="list" class="view-container">
        <SettingsPageHeader :title="t('settings.image.title')" />
        <p class="image-lede">{{ t("settings.image.description") }}</p>

        <div class="settings-group">
          <h2 class="settings-group-title">{{ t("settings.image.providersTitle") }}</h2>
          <div class="cards-list">
            <button
              v-for="provider in settingStore.imageProviders"
              :key="provider.id"
              type="button"
              class="provider-nav-card"
              @click="startEdit(provider.id)"
            >
              <div class="card-left">
                <div class="icon-wrapper">
                  <Globe2 class="size-5" />
                </div>
                <div class="card-text">
                  <h3>{{ provider.name }}</h3>
                  <p class="truncate max-w-[280px]">{{ providerSubtitle(provider) }}</p>
                </div>
              </div>
              <div class="card-right">
                <span class="status-badge" :class="{ configured: isConfigured(provider) }">
                  {{
                    isConfigured(provider)
                      ? t("settings.provider.configured")
                      : t("settings.provider.notConfigured")
                  }}
                </span>
                <ChevronRight class="size-4 text-muted-foreground arrow-icon" />
              </div>
            </button>

            <button type="button" class="provider-nav-card is-add" @click="startAdd">
              <div class="card-left">
                <div class="icon-wrapper is-muted">
                  <Globe2 class="size-5" />
                </div>
                <div class="card-text">
                  <h3>{{ t("settings.image.addProvider") }}</h3>
                  <p>{{ t("settings.image.urlPlaceholder") }}</p>
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

        <div class="settings-group">
          <h2 class="settings-group-title">{{ t("settings.image.modelTitle") }}</h2>
          <div class="settings-card">
            <article class="settings-row is-top">
              <div class="settings-row-copy">
                <h3>{{ t("settings.image.modelTitle") }}</h3>
                <p>{{ t("settings.fields.imageModel.description") }}</p>
              </div>
              <div class="settings-row-control is-stack">
                <Select
                  :model-value="selectedModelValue"
                  :disabled="imageModelOptions.length === 0"
                  @update:model-value="onImageModelChange"
                >
                  <SelectTrigger class="w-full">
                    <SelectValue :placeholder="t('settings.image.noModels')" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem
                      v-for="option in imageModelOptions"
                      :key="option.value"
                      :value="option.value"
                      :text-value="option.label"
                    >
                      {{ option.label }}
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </article>
          </div>
        </div>

        <div class="settings-group">
          <h2 class="settings-group-title">{{ t("settings.image.templatesTitle") }}</h2>
          <p class="image-lede is-inline">{{ t("settings.image.templatesHint") }}</p>
          <div class="templates-list">
            <p v-if="templates.length === 0" class="templates-empty">
              {{ t("settings.image.templateEmpty") }}
            </p>
            <article v-for="(item, index) in templates" :key="item.id" class="template-card">
              <div class="template-head">
                <Input
                  :model-value="item.name"
                  :placeholder="t('settings.image.templateNamePlaceholder')"
                  class="h-8 text-xs"
                  @update:model-value="patchTemplate(index, { name: String($event) })"
                  @blur="saveTemplates"
                />
                <button
                  type="button"
                  class="template-delete"
                  :aria-label="t('settings.image.templateDelete')"
                  @click="removeTemplate(index)"
                >
                  <Trash2 class="size-3.5" />
                </button>
              </div>
              <textarea
                :value="item.prompt"
                :placeholder="t('settings.image.templatePromptPlaceholder')"
                class="template-prompt"
                rows="3"
                @change="onPromptChange(index, $event)"
              ></textarea>
              <div class="template-example">
                <img
                  v-if="item.exampleImage"
                  :src="item.exampleImage"
                  alt=""
                  class="example-thumb"
                />
                <div class="example-actions">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    class="h-8"
                    @click="pickExample(index)"
                  >
                    {{ t("settings.image.templatePickImage") }}
                  </Button>
                  <Button
                    v-if="item.exampleImage"
                    type="button"
                    variant="ghost"
                    size="sm"
                    class="h-8"
                    @click="patchTemplate(index, { exampleImage: undefined }, true)"
                  >
                    {{ t("settings.image.templateRemoveImage") }}
                  </Button>
                </div>
                <p class="field-hint">{{ t("settings.image.templateExampleHint") }}</p>
              </div>
            </article>
            <Button type="button" size="sm" class="h-8 gap-1.5 self-start" @click="addTemplate">
              <Plus class="size-3.5" />
              {{ t("settings.image.addTemplate") }}
            </Button>
          </div>
        </div>
      </div>

      <div v-else key="edit" class="view-container">
        <div class="back-btn-row">
          <Button
            variant="ghost"
            size="sm"
            class="h-8 gap-1.5 pl-1.5 text-muted-foreground hover:text-foreground"
            @click="currentView = 'list'"
          >
            <ChevronLeft class="size-4" />
            {{ t("settings.provider.back") }}
          </Button>
        </div>

        <header class="edit-header">
          <div>
            <h2>{{ isNewProvider ? t("settings.image.addProvider") : editName }}</h2>
            <p>{{ t("settings.image.urlPlaceholder") }}</p>
          </div>
          <Button
            v-if="!isNewProvider"
            variant="destructive"
            size="sm"
            class="h-8 gap-1.5"
            @click="deleteEditing"
          >
            <Trash2 class="size-3.5" />
            {{ t("settings.provider.delete") }}
          </Button>
        </header>

        <div class="edit-form">
          <div class="field-row">
            <label>{{ t("settings.provider.name") }}</label>
            <Input
              v-model="editName"
              :placeholder="t('settings.image.namePlaceholder')"
              class="h-8 text-xs"
              @blur="saveEditing"
            />
          </div>
          <div class="field-row">
            <label>{{ t("settings.provider.baseUrl") }}</label>
            <Input
              v-model="editUrl"
              :placeholder="t('settings.image.urlPlaceholder')"
              class="h-8 text-xs font-mono"
              @blur="saveEditing"
            />
          </div>
          <div class="field-row">
            <label>{{ t("settings.provider.apiKey") }}</label>
            <SecretInput v-model="editKey" placeholder="sk-..." @blur="saveEditing" />
          </div>
          <div class="field-row">
            <label>{{ t("settings.provider.modelsList") }}</label>
            <p class="field-hint">{{ t("settings.image.modelsHint") }}</p>
            <div class="models-add-row">
              <Input
                v-model="modelDraft"
                :placeholder="t('settings.provider.modelsPlaceholder')"
                class="h-8 min-w-0 flex-1 text-xs font-mono"
                @keydown.enter.exact.prevent="addModel"
              />
              <Button type="button" size="sm" class="h-8 gap-1 shrink-0" @click="addModel">
                <Plus class="size-3.5" />
                {{ t("settings.provider.addModel") }}
              </Button>
            </div>
            <ul v-if="editModels.length > 0" class="models-list">
              <li
                v-for="(model, index) in editModels"
                :key="`${model}-${index}`"
                class="model-item"
              >
                <code class="model-id">{{ model }}</code>
                <button
                  type="button"
                  class="model-remove"
                  :aria-label="t('settings.provider.removeModel')"
                  @click="removeModel(index)"
                >
                  <X class="size-3.5" />
                </button>
              </li>
            </ul>
            <p v-else class="models-empty">{{ t("settings.provider.modelsEmpty") }}</p>
          </div>
          <Button size="sm" class="h-8 w-full gap-1.5" @click="saveEditingAndBack">
            <Save class="size-3.5" />
            {{ isNewProvider ? t("settings.provider.add") : t("settings.provider.save") }}
          </Button>
        </div>
      </div>
    </Transition>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { ChevronLeft, ChevronRight, Globe2, Plus, Save, Trash2, X } from "@lucide/vue";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SecretInput } from "@/components/ui/secret-input";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import SettingsPageHeader from "@/components/settings/SettingsPageHeader.vue";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { tr } from "@/services/i18n";
import type { SettingsI18nKey } from "@/services/locales/settings";
import type { CustomProviderConfig, ImageStyleTemplate } from "@/types/setting";
import { useSettingStore } from "@/stores/setting";
import { compressImageDataUrl } from "@/services/chat/compressImage";
import {
  decodeImageModelSelection,
  listImageModelChoices,
  newImageStyleTemplateId,
  selectedImageModelChoiceId,
} from "@/services/chat/imageGenMode";
import {
  isCustomProviderConfigured,
  looksLikeHttpUrl,
  parseProviderModels,
  serializeProviderModels,
} from "@/lib/providerPresets";

const settingStore = useSettingStore();
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const currentView = ref<"list" | "edit">("list");
const editingId = ref("");
const isNewProvider = ref(false);
const editName = ref("");
const editUrl = ref("");
const editKey = ref("");
const editModels = ref<string[]>([]);
const modelDraft = ref("");

function t(key: SettingsI18nKey) {
  return tr(settingStore.language, key);
}

function providerSubtitle(provider: CustomProviderConfig) {
  const url = provider.baseUrl.trim();
  if (looksLikeHttpUrl(url)) return url;
  const models = parseProviderModels(provider.models);
  if (models.length > 0) return models.join(", ");
  return t("settings.image.urlPlaceholder");
}

function isConfigured(provider: CustomProviderConfig) {
  return isCustomProviderConfigured(provider);
}

const imageModelOptions = computed(() =>
  listImageModelChoices(settingStore.imageProviders).map((option) => ({
    value: option.id,
    label: option.label ?? option.id,
  })),
);

const selectedModelValue = computed(() =>
  selectedImageModelChoiceId(
    settingStore.imageModelProvider,
    settingStore.imageModel,
    listImageModelChoices(settingStore.imageProviders),
  ),
);

const templates = computed(() => settingStore.imageStyleTemplates);

function bytesToDataUrl(bytes: Uint8Array, mime: string): string {
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return `data:${mime};base64,${btoa(binary)}`;
}

async function pathToCompressedDataUrl(path: string): Promise<string | null> {
  const { readFile } = await import("@tauri-apps/plugin-fs");
  const bytes = await readFile(path);
  const ext = path.split(".").pop()?.toLowerCase() ?? "png";
  const mime =
    ext === "jpg" || ext === "jpeg"
      ? "image/jpeg"
      : ext === "webp"
        ? "image/webp"
        : ext === "gif"
          ? "image/gif"
          : "image/png";
  return compressImageDataUrl(bytesToDataUrl(bytes, mime));
}

async function saveTemplates(next = templates.value) {
  await settingStore.update({ imageStyleTemplates: next });
}

function patchTemplate(index: number, patch: Partial<ImageStyleTemplate>, persist = false) {
  const next = templates.value.map((item, i) => (i === index ? { ...item, ...patch } : item));
  settingStore.imageStyleTemplates = next;
  if (persist) void saveTemplates(next);
}

function onPromptChange(index: number, event: Event) {
  const target = event.target;
  if (!(target instanceof HTMLTextAreaElement)) return;
  patchTemplate(index, { prompt: target.value }, true);
}

async function addTemplate() {
  const next = [
    ...templates.value,
    {
      id: newImageStyleTemplateId(),
      name: `${t("settings.image.addTemplate")} ${templates.value.length + 1}`,
      prompt: "",
    },
  ];
  await saveTemplates(next);
}

async function removeTemplate(index: number) {
  const next = templates.value.filter((_, i) => i !== index);
  await saveTemplates(next);
}

async function pickExample(index: number) {
  const selected = await openFileDialog({
    multiple: false,
    filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }],
  });
  const path = Array.isArray(selected) ? selected[0] : selected;
  if (!path) return;
  const dataUrl = await pathToCompressedDataUrl(path);
  if (!dataUrl) return;
  patchTemplate(index, { exampleImage: dataUrl }, true);
}

async function onImageModelChange(value: unknown) {
  if (typeof value !== "string") return;
  const selected = decodeImageModelSelection(value);
  if (!selected) return;
  await settingStore.update({
    imageModel: selected.model,
    imageModelProvider: selected.provider,
  });
}

function newProviderId() {
  return `img-${Math.random().toString(36).slice(2, 11)}`;
}

function startAdd() {
  editingId.value = newProviderId();
  isNewProvider.value = true;
  editName.value = "";
  editUrl.value = "";
  editKey.value = "";
  editModels.value = ["gpt-image-2"];
  modelDraft.value = "";
  currentView.value = "edit";
}

function startEdit(id: string) {
  const provider = settingStore.imageProviders.find((item) => item.id === id);
  if (!provider) return;
  editingId.value = id;
  isNewProvider.value = false;
  editName.value = provider.name;
  editUrl.value = provider.baseUrl;
  editKey.value = provider.apiKey;
  editModels.value = parseProviderModels(provider.models);
  modelDraft.value = "";
  currentView.value = "edit";
}

function addModel() {
  const ids = parseProviderModels(modelDraft.value);
  if (ids.length === 0) return;
  const next = [...editModels.value];
  for (const id of ids) {
    if (!next.includes(id)) next.push(id);
  }
  editModels.value = next;
  modelDraft.value = "";
  void saveEditing();
}

function removeModel(index: number) {
  editModels.value = editModels.value.filter((_, i) => i !== index);
  void saveEditing();
}

function buildProvider(): CustomProviderConfig {
  return {
    id: editingId.value,
    name: editName.value.trim() || t("settings.image.addProvider"),
    baseUrl: editUrl.value.trim(),
    apiKey: editKey.value.trim(),
    models: serializeProviderModels(editModels.value),
    disabledModels: "",
  };
}

async function ensureModelSelection(list: CustomProviderConfig[]) {
  const currentProvider = settingStore.imageModelProvider.trim();
  const currentModel = settingStore.imageModel.trim();
  const stillValid = list.some((provider) => {
    if (provider.id !== currentProvider) return false;
    const disabled = new Set(parseProviderModels(provider.disabledModels ?? ""));
    return parseProviderModels(provider.models).some(
      (id) => id === currentModel && !disabled.has(id),
    );
  });
  if (stillValid) return;

  for (const provider of list) {
    const disabled = new Set(parseProviderModels(provider.disabledModels ?? ""));
    const id = parseProviderModels(provider.models).find((model) => !disabled.has(model));
    if (id) {
      await settingStore.update({ imageModel: id, imageModelProvider: provider.id });
      return;
    }
  }
  await settingStore.update({ imageModel: "gpt-image-2", imageModelProvider: "" });
}

async function saveEditing() {
  if (!editingId.value) return;
  const next = buildProvider();
  const list = [...settingStore.imageProviders];
  const index = list.findIndex((item) => item.id === editingId.value);
  if (index === -1) {
    list.push(next);
    isNewProvider.value = false;
  } else {
    const current = list[index];
    if (
      current.name === next.name &&
      current.baseUrl === next.baseUrl &&
      current.apiKey === next.apiKey &&
      current.models === next.models
    ) {
      return;
    }
    list[index] = next;
  }
  await settingStore.update({ imageProviders: list });
  await ensureModelSelection(list);
}

async function saveEditingAndBack() {
  await saveEditing();
  currentView.value = "list";
}

async function deleteEditing() {
  const id = editingId.value;
  if (!id || isNewProvider.value) return;
  const confirmed = await confirmDialogRef.value?.ask({
    title: t("settings.provider.delete"),
    description: t("settings.provider.deleteConfirm"),
    confirmLabel: t("settings.history.deleteLabel"),
    cancelLabel: t("settings.history.cancel"),
  });
  if (!confirmed) return;
  const list = settingStore.imageProviders.filter((item) => item.id !== id);
  await settingStore.update({ imageProviders: list });
  await ensureModelSelection(list);
  currentView.value = "list";
}
</script>

<style scoped>
.image-lede {
  margin: -8px 0 18px;
  color: var(--peek-muted);
  font-size: 13px;
  line-height: 1.55;
}
.image-lede.is-inline {
  margin: 0 0 10px;
}
.templates-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.templates-empty {
  margin: 0;
  padding: 12px;
  border: 1px dashed var(--peek-border);
  border-radius: 10px;
  color: var(--peek-muted);
  font-size: 12px;
}
.template-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  border: 1px solid color-mix(in srgb, var(--peek-border) 88%, transparent);
  border-radius: 12px;
  background: var(--peek-list-bg);
}
.template-head {
  display: flex;
  align-items: center;
  gap: 8px;
}
.template-delete {
  display: inline-flex;
  flex: none;
  width: 28px;
  height: 28px;
  align-items: center;
  justify-content: center;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}
.template-prompt {
  width: 100%;
  min-height: 72px;
  padding: 8px 10px;
  border: 1px solid var(--peek-border);
  border-radius: 8px;
  background: transparent;
  color: var(--peek-text);
  font-size: 12px;
  line-height: 1.45;
  resize: vertical;
}
.template-example {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}
.example-thumb {
  width: 48px;
  height: 48px;
  object-fit: cover;
  border-radius: 8px;
  border: 1px solid var(--peek-border);
}
.example-actions {
  display: flex;
  gap: 6px;
}
.view-container {
  display: flex;
  flex-direction: column;
}
.cards-list {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--peek-border) 88%, transparent);
  border-radius: var(--peek-radius-lg, 12px);
  background: var(--peek-list-bg);
}
.provider-nav-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  width: 100%;
  padding: 11px 12px;
  border: 0;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 70%, transparent);
  background: transparent;
  text-align: left;
  cursor: pointer;
}
.provider-nav-card:first-child {
  border-top: 0;
}
.provider-nav-card:hover {
  background: color-mix(in srgb, var(--peek-text) 3.5%, transparent);
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
}
.card-text p {
  margin: 1px 0 0;
  color: var(--peek-muted);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 11px;
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
  padding: 2px 7px;
  border: 1px solid var(--peek-border);
  border-radius: 999px;
  background: color-mix(in srgb, var(--muted) 80%, transparent);
  color: var(--peek-muted);
  font-size: 10px;
  font-weight: 500;
  white-space: nowrap;
}
.status-badge.configured {
  color: var(--peek-text);
}
.arrow-icon {
  transition: transform 0.2s;
}
.provider-nav-card:hover .arrow-icon {
  transform: translateX(2px);
}
.back-btn-row {
  margin-bottom: 8px;
}
.edit-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 16px;
}
.edit-header h2 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}
.edit-header p {
  margin: 2px 0 0;
  color: var(--peek-muted);
  font-size: 11px;
}
.edit-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.field-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.field-row label {
  color: var(--peek-muted);
  font-size: 11px;
  font-weight: 500;
}
.field-hint {
  margin: 0;
  color: var(--peek-muted);
  font-size: 10px;
  line-height: 1.4;
}
.models-add-row {
  display: flex;
  gap: 8px;
}
.models-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.model-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px 6px 10px;
  border: 1px solid var(--peek-border);
  border-radius: 8px;
}
.model-id {
  min-width: 0;
  flex: 1;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 11px;
}
.model-remove {
  display: inline-flex;
  width: 24px;
  height: 24px;
  align-items: center;
  justify-content: center;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}
.models-empty {
  margin: 0;
  padding: 10px;
  border: 1px dashed var(--peek-border);
  border-radius: 8px;
  color: var(--peek-muted);
  font-size: 11px;
  text-align: center;
}
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: opacity 0.15s ease;
}
.fade-slide-enter-from,
.fade-slide-leave-to {
  opacity: 0;
}
</style>
