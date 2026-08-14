<template>
  <section class="settings-page">
    <SettingsPageHeader :title="pageTitle" />

    <p v-if="items.length === 0" class="text-muted-foreground px-1 py-6 text-sm">
      {{ emptyText }}
    </p>

    <section v-for="group in groups" :key="group.id" class="settings-group">
      <h2 class="settings-group-title">{{ group.title }}</h2>

      <div class="settings-card">
        <article
          v-for="item in group.items"
          :key="item.id"
          class="settings-row"
          :class="{
            'is-wide': item.type === 'collaboration-models' || item.type === 'hotkey-record',
            'is-top':
              item.type === 'collaboration-models' ||
              item.type === 'hotkey-record' ||
              item.type === 'select-model',
            'collaboration-setting': item.type === 'collaboration-models',
          }"
        >
          <div class="settings-row-copy">
            <h3>
              {{ item.title }}
              <SettingsHelpTip :text="item.help" />
            </h3>
            <p v-if="item.description">
              <template
                v-for="(part, index) in descriptionParts(item.description)"
                :key="`${item.id}-desc-${index}`"
              >
                <button
                  v-if="part.type === 'url'"
                  type="button"
                  class="setting-desc-link"
                  @click="openExternalUrl(part.value)"
                >
                  {{ part.value }}
                </button>
                <template v-else>{{ part.value }}</template>
              </template>
            </p>
          </div>

          <div
            class="settings-row-control"
            :class="{
              'is-stack': item.type === 'select-model' || item.type === 'hotkey-record',
              'collaboration-control': item.type === 'collaboration-models',
            }"
          >
            <div v-if="item.type === 'select-color'" class="settings-seg">
              <button
                v-for="option in builtInThemeOptions"
                :key="option.value"
                type="button"
                :class="{ on: selectedThemeValue === option.value }"
                @click="onThemeOptionSelect(option.value)"
              >
                {{ option.label }}
              </button>
            </div>

            <Select
              v-else-if="item.type === 'select-language'"
              :model-value="settingStore.language"
              @update:model-value="(v) => emit('language-change', v)"
            >
              <SelectTrigger class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="option in languageSelectOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>

            <Select
              v-else-if="item.type === 'select-zoom'"
              :model-value="String(settingStore.zoom)"
              @update:model-value="(v) => emit('zoom-change', v)"
            >
              <SelectTrigger class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="option in zoomSelectOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>

            <Select
              v-else-if="item.type === 'select-reasoning-effort'"
              :model-value="settingStore.reasoningEffort"
              @update:model-value="(v) => emit('reasoning-effort-change', v)"
            >
              <SelectTrigger class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="option in reasoningEffortSelectOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>

            <div v-else-if="item.type === 'select-model'" class="space-y-1.5">
              <div class="flex items-center gap-1.5">
                <Select
                  :model-value="resolveModelSelectValue(item.id)"
                  :disabled="chatModelStore.loading || availableModelOptions.length === 0"
                  @update:model-value="(v) => handleModelSelection(item.id, v)"
                >
                  <SelectTrigger class="w-full">
                    <SelectValue :placeholder="modelStatusText">
                      <span
                        v-if="selectedModelOption(item.id)"
                        class="inline-flex min-w-0 items-center gap-1.5"
                      >
                        <component
                          :is="selectedModelOption(item.id)?.icon"
                          v-if="selectedModelOption(item.id)?.icon"
                          class="size-3.5 shrink-0 text-muted-foreground"
                        />
                        <span class="truncate">{{ selectedModelOption(item.id)?.label }}</span>
                      </span>
                    </SelectValue>
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem
                      v-for="option in availableModelOptions"
                      :key="option.value"
                      :value="option.value"
                      :text-value="option.label"
                    >
                      <template v-if="option.icon" #leading>
                        <component
                          :is="option.icon"
                          class="size-3.5 shrink-0 text-muted-foreground"
                        />
                      </template>
                      {{ option.label }}
                    </SelectItem>
                  </SelectContent>
                </Select>
                <Button
                  type="button"
                  variant="outline"
                  size="icon"
                  class="size-8 shrink-0"
                  :disabled="chatModelStore.loading || chatModelStore.refreshing"
                  :title="refreshModelsLabel"
                  :aria-label="refreshModelsLabel"
                  @click="refreshModelList"
                >
                  <RefreshCw
                    class="size-3.5"
                    :class="{ 'animate-spin': chatModelStore.refreshing }"
                  />
                </Button>
              </div>
              <Select
                v-if="selectedModelThinkingTierOptions(item.id).length > 1"
                :model-value="
                  item.id === 'multimodalModel'
                    ? settingStore.multimodalModel
                    : settingStore.chatModel
                "
                :disabled="chatModelStore.loading"
                @update:model-value="(v) => handleThinkingTierSelection(item.id, v)"
              >
                <SelectTrigger class="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem
                    v-for="option in selectedModelThinkingTierOptions(item.id)"
                    :key="option.value"
                    :value="option.value"
                  >
                    {{ option.label }}
                  </SelectItem>
                </SelectContent>
              </Select>
              <p v-if="chatModelStore.error" class="text-[10px] leading-4 text-destructive">
                {{ chatModelStore.error }}
              </p>
            </div>

            <div v-else-if="item.type === 'collaboration-models'" class="collaboration-models">
              <button
                type="button"
                class="setting-toggle"
                :class="{ active: settingStore.multiModelCollaboration }"
                :aria-pressed="settingStore.multiModelCollaboration"
                @click="toggleModelCollaboration"
              >
                <span class="setting-toggle-knob"></span>
              </button>
              <div
                v-if="settingStore.multiModelCollaboration"
                class="collaboration-model-list peek-scrollbar"
              >
                <label
                  v-for="option in availableModelOptions"
                  :key="option.value"
                  class="collaboration-model-option"
                >
                  <input
                    type="checkbox"
                    :checked="settingStore.collaborationModels.includes(option.value)"
                    @change="toggleCollaborationModel(option.value)"
                  />
                  <component
                    :is="option.icon"
                    v-if="option.icon"
                    class="size-3.5 shrink-0 text-muted-foreground"
                  />
                  <span :title="option.label">{{ option.label }}</span>
                </label>
                <p v-if="!availableModelOptions.length" class="text-[10px] text-muted-foreground">
                  {{ modelStatusText }}
                </p>
              </div>
            </div>

            <Select
              v-else-if="item.type === 'select-reasoning-language'"
              :model-value="settingStore.reasoningLanguage"
              @update:model-value="(v) => emit('reasoning-language-change', v)"
            >
              <SelectTrigger class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="option in reasoningLanguageSelectOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>

            <Select
              v-else-if="item.type === 'select-web-search-provider'"
              :model-value="settingStore.webSearchProvider"
              :disabled="!settingStore.webSearchEnabled"
              @update:model-value="(v) => emit('web-search-provider-change', v)"
            >
              <SelectTrigger class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="option in webSearchProviderSelectOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>

            <Select
              v-else-if="item.type === 'select-tool-approval-mode'"
              :model-value="settingStore.toolApprovalMode"
              @update:model-value="(v) => emit('tool-approval-mode-change', v)"
            >
              <SelectTrigger class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="option in toolApprovalModeSelectOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>

            <Select
              v-else-if="item.type === 'select-agent-work-display'"
              :model-value="settingStore.agentWorkDisplay"
              @update:model-value="(v) => emit('agent-work-display-change', v)"
            >
              <SelectTrigger class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="option in agentWorkDisplaySelectOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>

            <SecretInput
              v-else-if="item.type === 'secret'"
              :model-value="apiKeyDraft"
              :placeholder="apiKeyPlaceholder"
              @update:model-value="(v) => emit('update:apiKeyDraft', String(v))"
              @blur="emit('save-api-key')"
            />

            <SecretInput
              v-else-if="item.type === 'memory-secret'"
              :model-value="mem0ApiKeyDraft"
              placeholder="m0-..."
              :disabled="!settingStore.memoryEnabled"
              @update:model-value="(v) => emit('update:mem0ApiKeyDraft', String(v))"
              @blur="emit('save-memory-settings')"
            />

            <SecretInput
              v-else-if="item.type === 'search-secret'"
              :model-value="item.id === 'serperApiKey' ? serperApiKeyDraft : tavilyApiKeyDraft"
              :placeholder="item.id === 'serperApiKey' ? 'serper-...' : 'tvly-...'"
              :disabled="!settingStore.webSearchEnabled"
              @update:model-value="(v) => onSearchSecretInput(item.id, v)"
              @blur="emit('save-web-search-settings')"
            />

            <Input
              v-else-if="item.type === 'memory-text'"
              :model-value="item.id === 'mem0UserId' ? mem0UserIdDraft : mem0BaseUrlDraft"
              :disabled="!settingStore.memoryEnabled"
              @update:model-value="(v) => onMemoryTextInput(item.id, v)"
              @blur="emit('save-memory-settings')"
            />

            <button
              v-else-if="item.type === 'toggle'"
              type="button"
              class="setting-toggle"
              :class="{ active: toggleActive(item.id) }"
              :aria-pressed="toggleActive(item.id)"
              @click="emit('toggle', item.id)"
            >
              <span class="setting-toggle-knob"></span>
            </button>

            <div
              v-else-if="item.type === 'slider'"
              class="flex items-center gap-3 w-full max-w-[200px]"
            >
              <input
                type="range"
                :min="item.min ?? 10"
                :max="item.max ?? 100"
                :step="item.step ?? 5"
                :value="getSettingValue(item.id)"
                @input="(e) => onSliderChange(item.id, e)"
                class="setting-slider h-1.5 w-full cursor-pointer appearance-none rounded-lg bg-border accent-primary focus:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              />
              <span
                class="text-xs font-semibold tabular-nums min-w-[36px] text-right text-muted-foreground select-none"
              >
                {{ getSettingValue(item.id) }}%
              </span>
            </div>

            <HotkeyRecordField
              v-else-if="item.type === 'hotkey-record'"
              :model-value="
                item.id === 'primaryHotkey'
                  ? settingStore.primaryHotkey
                  : settingStore.secondaryHotkey
              "
              :enabled="
                item.id === 'primaryHotkey'
                  ? settingStore.primaryHotkeyEnabled
                  : settingStore.secondaryHotkeyEnabled
              "
              :setting-key="item.id === 'primaryHotkey' ? 'primaryHotkey' : 'secondaryHotkey'"
              :mode="item.id === 'primaryHotkey' ? 'double-modifier' : 'chord'"
              :default-value="item.id === 'primaryHotkey' ? 'Alt' : 'Ctrl+Alt+Space'"
              @update:model-value="
                (value) =>
                  item.id === 'primaryHotkey'
                    ? (settingStore.primaryHotkey = value)
                    : (settingStore.secondaryHotkey = value)
              "
              @update:enabled="
                (value) =>
                  item.id === 'primaryHotkey'
                    ? (settingStore.primaryHotkeyEnabled = value)
                    : (settingStore.secondaryHotkeyEnabled = value)
              "
            />

            <span
              v-else
              class="text-sm"
              :class="{ 'font-mono text-xs break-all': item.id === 'appIdentifier' }"
            >
              {{ item.value }}
            </span>
          </div>
        </article>
      </div>
    </section>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { RefreshCw } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SecretInput } from "@/components/ui/secret-input";
import HotkeyRecordField from "@/components/settings/HotkeyRecordField.vue";
import SettingsHelpTip from "@/components/settings/SettingsHelpTip.vue";
import SettingsPageHeader from "@/components/settings/SettingsPageHeader.vue";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useSettingStore } from "@/stores/setting";
import { useChatModelStore } from "@/stores/chatModel";
import {
  getProviderIcon,
  getModelDisplayLabel,
  isDeepSeekProvider,
  isGeminiProvider,
  resolveCustomPresetId,
} from "@/lib/providerIcons";
import {
  findModelEntry,
  isModelEntrySelected,
  localizeThinkingTierLabel,
} from "@/lib/modelThinking";
import { tr } from "@/services/i18n";
import type { SettingDefinition } from "@/pages/Settings/settingsDefinitions";
import type { ModelSelection } from "@/types/setting";
import {
  colorSchemeOptions,
  languageOptions,
  localizedOptionLabel,
  reasoningEffortOptions,
  reasoningLanguageOptions,
  webSearchProviderOptions,
  toolApprovalModeOptions,
  agentWorkDisplayOptions,
  zoomOptions,
} from "@/types/setting";

type DescriptionPart = { type: "text" | "url"; value: string };

function descriptionParts(text: string): DescriptionPart[] {
  const source = String(text ?? "");
  if (!source) return [{ type: "text", value: "" }];
  const parts: DescriptionPart[] = [];
  const pattern = /https?:\/\/[^\s]+/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(source))) {
    if (match.index > lastIndex) {
      parts.push({ type: "text", value: source.slice(lastIndex, match.index) });
    }
    let url = match[0];
    let trailing = "";
    while (/[.,;:!?)]$/.test(url)) {
      trailing = `${url.slice(-1)}${trailing}`;
      url = url.slice(0, -1);
    }
    if (url) parts.push({ type: "url", value: url });
    if (trailing) parts.push({ type: "text", value: trailing });
    lastIndex = match.index + match[0].length;
  }
  if (lastIndex < source.length) {
    parts.push({ type: "text", value: source.slice(lastIndex) });
  }
  return parts.length ? parts : [{ type: "text", value: source }];
}

async function openExternalUrl(url: string) {
  try {
    await openUrl(url);
  } catch (error) {
    console.error("failed to open url in default browser:", url, error);
  }
}

const props = defineProps<{
  items: SettingDefinition[];
  emptyText: string;
  pageTitle: string;
  apiKeyDraft: string;
  mem0ApiKeyDraft: string;
  mem0UserIdDraft: string;
  mem0BaseUrlDraft: string;
  serperApiKeyDraft: string;
  tavilyApiKeyDraft: string;
}>();

const emit = defineEmits<{
  toggle: [id: string];
  "slider-change": [id: string, value: number];
  "color-scheme-change": [value: unknown];
  "language-change": [value: unknown];
  "zoom-change": [value: unknown];
  "reasoning-effort-change": [value: unknown];
  "reasoning-language-change": [value: unknown];
  "tool-approval-mode-change": [value: unknown];
  "agent-work-display-change": [value: unknown];
  "web-search-provider-change": [value: unknown];
  "default-model-change": [value: unknown];
  "multimodal-model-change": [value: unknown];
  "update:apiKeyDraft": [value: string];
  "save-api-key": [];
  "update:mem0ApiKeyDraft": [value: string];
  "update:mem0UserIdDraft": [value: string];
  "update:mem0BaseUrlDraft": [value: string];
  "save-memory-settings": [];
  "update:serperApiKeyDraft": [value: string];
  "update:tavilyApiKeyDraft": [value: string];
  "save-web-search-settings": [];
}>();

const settingStore = useSettingStore();
const chatModelStore = useChatModelStore();

const selectedThemeValue = computed(() => `builtin:${settingStore.colorScheme}`);

function onThemeOptionSelect(value: string) {
  if (value !== selectedThemeValue.value) {
    emit("color-scheme-change", value);
  }
}

const apiKeyPlaceholder = computed(() => tr(settingStore.language, "settings.apiKeyPlaceholder"));

const groups = computed(() => {
  const map = new Map<string, SettingDefinition[]>();
  for (const item of props.items) {
    const list = map.get(item.group) ?? [];
    list.push(item);
    map.set(item.group, list);
  }
  return Array.from(map.entries()).map(([title, groupItems]) => ({
    id: title,
    title,
    items: groupItems,
  }));
});

const builtInThemeOptions = computed(() =>
  colorSchemeOptions.map((option) => ({
    value: `builtin:${option.value}`,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const languageSelectOptions = computed(() =>
  languageOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const reasoningEffortSelectOptions = computed(() =>
  reasoningEffortOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const reasoningLanguageSelectOptions = computed(() =>
  reasoningLanguageOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const webSearchProviderSelectOptions = computed(() =>
  webSearchProviderOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const toolApprovalModeSelectOptions = computed(() =>
  toolApprovalModeOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const agentWorkDisplaySelectOptions = computed(() =>
  agentWorkDisplayOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const zoomSelectOptions = computed(() =>
  zoomOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, settingStore.language),
  })),
);

const availableModelOptions = computed(() => {
  const models = [...chatModelStore.models];
  const currentSelections: ModelSelection[] = [
    { id: settingStore.chatModel.trim(), provider: settingStore.chatModelProvider.trim() },
    {
      id: settingStore.multimodalModel.trim(),
      provider: settingStore.multimodalModelProvider.trim(),
    },
  ];
  for (const current of currentSelections) {
    if (
      current.id &&
      models.length > 0 &&
      !models.some((model) => isModelEntrySelected(model, current.id, current.provider))
    ) {
      models.unshift({ id: current.id, ownedBy: "", provider: current.provider });
    }
  }
  return models.map((model) => {
    const name = getModelDisplayLabel(model);
    const showOwner =
      !!model.ownedBy && !isDeepSeekProvider(model.provider) && !isGeminiProvider(model.provider);
    return {
      value: encodeModelSelection(model),
      label: showOwner ? `${name} · ${model.ownedBy}` : name,
      icon: getProviderIcon(
        model.provider,
        resolveCustomPresetId(model.provider, settingStore.customProviders),
      ),
      model,
    };
  });
});

function encodeModelSelection(selection: ModelSelection) {
  return JSON.stringify([selection.provider, selection.id]);
}

function modelSelectionForItem(itemId: string): ModelSelection {
  return itemId === "multimodalModel"
    ? { id: settingStore.multimodalModel, provider: settingStore.multimodalModelProvider }
    : { id: settingStore.chatModel, provider: settingStore.chatModelProvider };
}

function resolveModelSelectValue(itemId: string) {
  const selection = modelSelectionForItem(itemId);
  const entry = findModelEntry(chatModelStore.models, selection.id, selection.provider);
  return encodeModelSelection({
    id: entry?.id ?? selection.id,
    provider: entry?.provider ?? selection.provider,
  });
}

function handleModelSelection(itemId: string, value: unknown) {
  if (typeof value !== "string") return;
  try {
    const [provider, id] = JSON.parse(value) as [string, string];
    const selection = { id, provider } satisfies ModelSelection;
    if (itemId === "multimodalModel") emit("multimodal-model-change", selection);
    else emit("default-model-change", selection);
  } catch {
    // Select values are generated by encodeModelSelection.
  }
}

function handleThinkingTierSelection(itemId: string, value: unknown) {
  if (typeof value !== "string") return;
  const current = modelSelectionForItem(itemId);
  const selection = { id: value, provider: current.provider } satisfies ModelSelection;
  if (itemId === "multimodalModel") emit("multimodal-model-change", selection);
  else emit("default-model-change", selection);
}

function selectedModelThinkingTierOptions(itemId: string) {
  const selectedId =
    itemId === "multimodalModel" ? settingStore.multimodalModel : settingStore.chatModel;
  const provider = modelSelectionForItem(itemId).provider;
  const entry = findModelEntry(chatModelStore.models, selectedId, provider);
  if (!entry?.thinkingVariants?.length) {
    return [];
  }
  return entry.thinkingVariants.map((variant) => ({
    value: variant.id,
    label: localizeThinkingTierLabel(variant.label, settingStore.language),
  }));
}

function selectedModelOption(itemId: string) {
  const selectedId =
    itemId === "multimodalModel" ? settingStore.multimodalModel : settingStore.chatModel;
  const provider = modelSelectionForItem(itemId).provider;
  const entry = findModelEntry(chatModelStore.models, selectedId, provider);
  const optionValue = encodeModelSelection({
    id: entry?.id ?? selectedId,
    provider: entry?.provider ?? provider,
  });
  return availableModelOptions.value.find((option) => option.value === optionValue) ?? null;
}

const modelStatusText = computed(() => {
  if (chatModelStore.loading) {
    return tr(settingStore.language, "loadingModels");
  }
  return tr(settingStore.language, "noModels");
});

const refreshModelsLabel = computed(() => tr(settingStore.language, "refreshModels"));

async function refreshModelList() {
  await chatModelStore.reload();
}

function toggleModelCollaboration() {
  void settingStore.update({
    multiModelCollaboration: !settingStore.multiModelCollaboration,
  });
}

function toggleCollaborationModel(model: string) {
  const selected = new Set(settingStore.collaborationModels);
  if (selected.has(model)) selected.delete(model);
  else selected.add(model);
  void settingStore.update({ collaborationModels: [...selected] });
}

function toggleActive(id: string) {
  if (id === "memoryEnabled") return settingStore.memoryEnabled;
  if (id === "webSearchEnabled") return settingStore.webSearchEnabled;
  if (id === "lspEnabled") return settingStore.lspEnabled;
  if (id === "passToolReasoning") return settingStore.passToolReasoning;
  if (id === "continueThinkingAfterTools") return settingStore.continueThinkingAfterTools;
  if (id === "showReasoning") return settingStore.showReasoning;
  if (id === "multimodalSplitAnalysis") return settingStore.multimodalSplitAnalysis;
  if (id === "largeContextEnabled") return settingStore.largeContextEnabled;
  if (id === "hardwareAccelerationEnabled") return settingStore.hardwareAccelerationEnabled;
  if (id === "chromeFrostedGlass") return settingStore.chromeFrostedGlass;
  if (id === "pixpinPinAiEnabled") return settingStore.pixpinPinAiEnabled;
  if (id === "snipastePinAiEnabled") return settingStore.snipastePinAiEnabled;
  if (id === "minimalCoding") return settingStore.minimalCoding;
  return false;
}

function getSettingValue(id: string) {
  if (id === "opacity") {
    return settingStore.opacity;
  }
  return 100;
}

function onSliderChange(id: string, event: Event) {
  const target = event.target as HTMLInputElement;
  const value = parseInt(target.value, 10);
  emit("slider-change", id, value);
}

function onMemoryTextInput(id: string, value: string | number) {
  if (id === "mem0UserId") emit("update:mem0UserIdDraft", String(value));
  if (id === "mem0BaseUrl") emit("update:mem0BaseUrlDraft", String(value));
}

function onSearchSecretInput(id: string, value: string | number) {
  if (id === "serperApiKey") emit("update:serperApiKeyDraft", String(value));
  if (id === "tavilyApiKey") emit("update:tavilyApiKeyDraft", String(value));
}
</script>

<style scoped>
.setting-desc-link {
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

.setting-desc-link:hover {
  color: color-mix(in srgb, var(--primary) 82%, var(--foreground));
}

.setting-group-title {
  margin: 0;
  padding: 7px 10px;
  color: var(--peek-faint);
  font-size: 11px;
  font-weight: 650;
}

.setting-row {
  border-radius: 6px;
  transition: background-color 120ms ease;
}

.setting-toggle {
  position: relative;
  width: 44px;
  height: 24px;
  margin: 0;
  padding: 0;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--muted);
  cursor: default;
  transition:
    background 160ms ease,
    border-color 160ms ease;
}

.setting-toggle.active {
  background: var(--primary);
  border-color: color-mix(in srgb, var(--primary) 70%, white 30%);
}

.setting-toggle-knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: white;
  box-shadow: 0 1px 3px rgb(0 0 0 / 28%);
  transition: transform 160ms ease;
}

.setting-toggle.active .setting-toggle-knob {
  transform: translateX(20px);
}

.collaboration-models {
  display: contents;
}
.collaboration-control {
  display: contents;
}
.settings-row.collaboration-setting {
  grid-template-columns: minmax(0, 1fr) auto;
}
.collaboration-models > .setting-toggle {
  justify-self: end;
}
.collaboration-model-list {
  width: 100%;
  max-height: 216px;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  overflow-y: auto;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: color-mix(in srgb, var(--background) 88%, var(--muted));
}
.collaboration-setting .collaboration-model-list {
  grid-column: 1 / -1;
}
.collaboration-model-option {
  min-width: 0;
  min-height: 34px;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 5px 8px;
  border-right: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
  border-bottom: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
  cursor: pointer;
}
.collaboration-model-option:nth-child(2n) {
  border-right: 0;
}
.collaboration-model-option:nth-last-child(-n + 2) {
  border-bottom: 0;
}
.collaboration-model-option:hover {
  background: var(--accent);
}
.collaboration-model-option input {
  flex: none;
  accent-color: var(--primary);
}
.collaboration-model-option span {
  min-width: 0;
  overflow: hidden;
  color: var(--foreground);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
@media (max-width: 520px) {
  .collaboration-model-list {
    grid-template-columns: minmax(0, 1fr);
  }
  .collaboration-model-option {
    border-right: 0;
  }
  .collaboration-model-option:nth-last-child(-n + 2) {
    border-bottom: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
  }
  .collaboration-model-option:last-child {
    border-bottom: 0;
  }
}
</style>
