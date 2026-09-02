<template>
  <ul
    class="command-list model-picker-list peek-scrollbar"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
  >
    <template v-if="loading && models.length === 0">
      <li class="picker-status">{{ loadingText }}</li>
    </template>
    <template v-else-if="error && models.length === 0">
      <li class="picker-status error">{{ error }}</li>
    </template>
    <template v-else-if="showingGroups && groups.length === 0">
      <li class="picker-status">{{ emptyText }}</li>
    </template>
    <template v-else-if="!showingGroups && modelRows.length === 0">
      <li class="picker-status">{{ emptyText }}</li>
    </template>
    <template v-else-if="showingGroups">
      <TooltipProvider :delay-duration="220">
        <li v-for="group in groups" :key="group.provider" class="model-picker-row">
          <Tooltip>
            <TooltipTrigger as-child>
              <div
                class="command-item model-picker-item model-group-item"
                :class="{
                  active: group.index === selectedIndex,
                  current: group.hasCurrent,
                }"
                role="option"
                :aria-selected="group.index === selectedIndex"
                @mouseenter="$emit('hover', group.index)"
                @mousedown.prevent="$emit('selectGroup', group.provider)"
              >
                <span class="model-group-leading" aria-hidden="true">
                  <component
                    :is="groupBrandIcon(group)"
                    v-if="groupBrandIcon(group)"
                    :size="13"
                    class="model-group-icon"
                  />
                  <img
                    v-else-if="groupFavicon(group)"
                    :src="groupFavicon(group)!"
                    alt=""
                    class="model-group-favicon"
                  />
                  <span v-else class="model-icon-dot" />
                </span>
                <span class="model-name">{{ group.label }}</span>
                <span class="model-meta">{{ groupCountLabel(group.models.length) }}</span>
                <Check v-if="group.hasCurrent" :size="13" class="model-check" aria-hidden="true" />
                <ChevronRight :size="13" class="model-group-chevron" aria-hidden="true" />
              </div>
            </TooltipTrigger>
            <TooltipContent
              v-if="providerHover(group.provider)"
              side="right"
              :side-offset="10"
              class="model-provider-tooltip"
            >
              <ModelProviderTip
                :name="providerHover(group.provider)!.name"
                :detail="providerHover(group.provider)!.detail"
                :brand-icon="providerHover(group.provider)!.brandIcon"
                :favicon="providerHover(group.provider)!.favicon"
              />
            </TooltipContent>
          </Tooltip>
        </li>
      </TooltipProvider>
    </template>
    <template v-else>
      <li
        v-if="hierarchical"
        class="command-item model-picker-back"
        role="option"
        :aria-label="backText"
        @mousedown.prevent="$emit('back')"
      >
        <ChevronLeft :size="13" class="back-icon" />
        <span class="back-label">{{ activeGroupLabel }}</span>
      </li>
      <TooltipProvider :delay-duration="220">
        <li
          v-for="entry in modelRows"
          :key="`${entry.model.provider}:${entry.model.id}`"
          class="model-picker-row"
        >
          <Tooltip>
            <TooltipTrigger as-child>
              <div
                class="command-item model-picker-item"
                :class="{
                  active: entry.index === selectedIndex,
                  current: isModelEntrySelected(entry.model, selectedModelId, selectedProvider),
                }"
                role="option"
                :aria-selected="entry.index === selectedIndex"
                @mouseenter="$emit('hover', entry.index)"
                @mousedown.prevent="$emit('select', entry.model)"
              >
                <component
                  :is="modelIcon(entry.model)"
                  v-if="modelIcon(entry.model)"
                  :size="13"
                  class="model-item-icon"
                  aria-hidden="true"
                />
                <span class="model-name">{{ getModelDisplayLabel(entry.model) }}</span>
                <span v-if="getModelDisplaySubtitle(entry.model)" class="model-meta">
                  {{ getModelDisplaySubtitle(entry.model) }}
                </span>
                <Check
                  v-if="isModelEntrySelected(entry.model, selectedModelId, selectedProvider)"
                  :size="13"
                  class="model-check"
                  aria-hidden="true"
                />
              </div>
            </TooltipTrigger>
            <TooltipContent
              v-if="providerHover(entry.model.provider)"
              side="right"
              :side-offset="10"
              class="model-provider-tooltip"
            >
              <ModelProviderTip
                :name="providerHover(entry.model.provider)!.name"
                :detail="providerHover(entry.model.provider)!.detail"
                :brand-icon="providerHover(entry.model.provider)!.brandIcon"
                :favicon="providerHover(entry.model.provider)!.favicon"
              />
            </TooltipContent>
          </Tooltip>
          <!-- Thinking effort lives right under the model it applies to. -->
          <div
            v-if="
              thinkingOptions.length > 1 &&
              isModelEntrySelected(entry.model, selectedModelId, selectedProvider)
            "
            class="model-thinking-inline"
            @mousedown.prevent
          >
            <ThinkingEffortSlider
              inline
              :options="thinkingOptions"
              :selected-id="thinkingSelectedId"
              :title="thinkingTitle"
              @select="$emit('selectThinking', $event)"
            />
          </div>
        </li>
      </TooltipProvider>
    </template>

    <li
      class="command-item model-picker-refresh"
      :class="{ active: selectedIndex === refreshIndex }"
      role="option"
      :aria-selected="selectedIndex === refreshIndex"
      :aria-disabled="refreshing"
      @mouseenter="$emit('hover', refreshIndex)"
      @mousedown.prevent="!refreshing && $emit('refresh')"
    >
      <RefreshCw :size="12" class="refresh-icon" :class="{ spinning: refreshing }" />
      <span class="refresh-label">{{ refreshText }}</span>
    </li>
  </ul>
</template>

<script setup lang="ts">
import { computed, watch } from "vue";
import { Check, ChevronLeft, ChevronRight, RefreshCw } from "@lucide/vue";
import type { ChatModelInfo } from "@/types/chat";
import {
  DEEPSEEK_PROVIDER,
  GEMINI_PROVIDER,
  getModelDisplayLabel,
  getModelDisplaySubtitle,
  getModelIcon,
  getProviderHoverInfo,
  getProviderIcon,
  groupModelsByProvider,
  type ModelProviderGroup,
} from "@/lib/providerIcons";
import { peekProviderFavicon, warmProviderFavicons } from "@/services/providerFavicon";
import ModelProviderTip from "./ModelProviderTip.vue";
import ThinkingEffortSlider from "./ThinkingEffortSlider.vue";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { isModelEntrySelected } from "@/lib/modelThinking";
import { useSettingStore } from "@/stores/setting";

const props = withDefaults(
  defineProps<{
    models: ChatModelInfo[];
    selectedModelId: string;
    selectedProvider: string;
    selectedIndex: number;
    /** `null` shows the provider list when more than one group exists. */
    activeProvider: string | null;
    loading: boolean;
    refreshing?: boolean;
    error: string | null;
    loadingText: string;
    emptyText: string;
    refreshText: string;
    backText: string;
    modelCountText: string;
    ariaLabel: string;
    /** Thinking effort / tier choices for the selected model; shown as a slider under it. */
    thinkingOptions?: Array<{ id: string; label: string }>;
    thinkingSelectedId?: string;
    thinkingTitle?: string;
  }>(),
  {
    refreshing: false,
    thinkingOptions: () => [],
    thinkingSelectedId: "",
    thinkingTitle: "",
  },
);

defineEmits<{
  hover: [index: number];
  select: [model: ChatModelInfo];
  selectGroup: [provider: string];
  back: [];
  refresh: [];
  selectThinking: [id: string];
}>();

const settingStore = useSettingStore();

type Group = ModelProviderGroup & {
  index: number;
  hasCurrent: boolean;
};

function modelIcon(model: ChatModelInfo) {
  return getModelIcon(model);
}

function groupBrandIcon(group: Pick<ModelProviderGroup, "provider">) {
  if (group.provider === DEEPSEEK_PROVIDER || group.provider === GEMINI_PROVIDER) {
    return getProviderIcon(group.provider);
  }
  return null;
}

function groupFavicon(group: Pick<ModelProviderGroup, "provider">) {
  return peekProviderFavicon(group.provider);
}

function providerHover(providerId: string | null | undefined) {
  return getProviderHoverInfo(providerId, settingStore.customProviders);
}

function groupCountLabel(count: number) {
  return props.modelCountText.replace("{count}", String(count));
}

watch(
  () => settingStore.customProviders,
  (providers) => {
    warmProviderFavicons(providers);
  },
  { immediate: true, deep: true },
);

const grouped = computed(() => groupModelsByProvider(props.models, settingStore.customProviders));

const hierarchical = computed(() => grouped.value.length > 1);

const showingGroups = computed(() => hierarchical.value && !props.activeProvider);

const groups = computed<Group[]>(() =>
  grouped.value.map((group, index) => ({
    ...group,
    index,
    hasCurrent: group.models.some((model) =>
      isModelEntrySelected(model, props.selectedModelId, props.selectedProvider),
    ),
  })),
);

const activeGroupLabel = computed(() => {
  const match = grouped.value.find((group) => group.provider === props.activeProvider);
  return match?.label ?? props.backText;
});

const modelRows = computed(() => {
  const source = props.activeProvider
    ? (grouped.value.find((group) => group.provider === props.activeProvider)?.models ?? [])
    : grouped.value.length === 1
      ? grouped.value[0].models
      : props.models;
  return source.map((model, index) => ({ model, index }));
});

const refreshIndex = computed(() =>
  showingGroups.value ? groups.value.length : modelRows.value.length,
);
</script>

<style scoped>
.command-list {
  --command-row-height: 32px;
  --command-list-padding: 6px;
  --command-list-visible-rows: 12;
  list-style: none;
  margin: 0;
  padding: 4px 0 0;
  border-bottom: 1px solid var(--peek-border);
  background: var(--peek-list-bg);
  flex: none;
  max-height: min(
    calc(
      var(--command-row-height) * var(--command-list-visible-rows) + var(--command-list-padding) +
        34px
    ),
    calc(100vh - 140px)
  );
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.picker-status {
  padding: 8px 12px;
  font-size: 12px;
  line-height: 1.45;
  color: var(--peek-muted);
  pointer-events: none;
}

.picker-status.error {
  color: color-mix(in srgb, var(--destructive) 82%, var(--peek-muted));
}

.model-group-leading {
  flex: none;
  width: 16px;
  height: 16px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--peek-muted);
  opacity: 0.9;
}

.model-group-icon {
  opacity: 0.9;
}

.model-group-favicon {
  width: 13px;
  height: 13px;
  border-radius: 2px;
  object-fit: contain;
}

.model-picker-row {
  display: block;
}

.command-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  height: var(--command-row-height);
  cursor: default;
}

.model-picker-item {
  padding-left: 12px;
}

.model-item-icon {
  flex: none;
  color: var(--peek-muted);
  opacity: 0.9;
}

.command-item.active {
  background: var(--peek-list-active);
}

.model-picker-item.current:not(.active) {
  background: color-mix(in srgb, var(--peek-accent) 7%, transparent);
}

/* Slider strip under the current model: same tint as the current row so they read as one
   block, fixed height so the list rhythm stays even. */
.model-thinking-inline {
  height: 30px;
  overflow: hidden;
  margin-bottom: 2px;
  background: color-mix(in srgb, var(--peek-accent) 7%, transparent);
  border-radius: 0 0 6px 6px;
}

.model-picker-list .model-picker-row:has(.model-thinking-inline) .model-picker-item {
  border-bottom-left-radius: 0;
  border-bottom-right-radius: 0;
}

.model-icon-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--peek-muted) 55%, transparent);
}

.model-name {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  font-weight: 500;
  line-height: 16px;
  color: var(--peek-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.model-meta {
  flex: none;
  max-width: 38%;
  font-family: var(--font-mono);
  font-size: 11px;
  line-height: 14px;
  color: var(--peek-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.model-check {
  flex: none;
  color: var(--peek-accent);
  opacity: 0.95;
}

.model-group-chevron {
  flex: none;
  color: var(--peek-muted);
  opacity: 0.7;
}

.model-picker-back {
  padding-left: 12px;
  color: var(--peek-muted);
  border-bottom: 1px solid color-mix(in srgb, var(--peek-text) 8%, transparent);
}

.model-picker-back:hover {
  color: var(--peek-text);
  background: var(--peek-list-active);
}

.back-icon {
  flex: none;
  opacity: 0.85;
}

.back-label {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.03em;
  text-transform: uppercase;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.model-picker-refresh {
  margin-top: 2px;
  height: 30px;
  padding-left: 12px;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 90%, transparent);
  color: var(--peek-muted);
  gap: 8px;
}

.model-picker-refresh.active {
  color: var(--peek-text);
}

.model-picker-refresh[aria-disabled="true"] {
  opacity: 0.55;
  cursor: default;
}

.refresh-icon {
  flex: none;
  opacity: 0.85;
}

.refresh-label {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  line-height: 14px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.spinning {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
