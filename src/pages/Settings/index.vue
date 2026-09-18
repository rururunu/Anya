<template>
  <div
    class="settings-workbench"
    :class="{
      embedded: props.embedded,
      'is-glass': settingStore.chromeFrostedGlass,
      'has-custom-bg': hasCustomBg,
    }"
  >
    <AppConfirmDialog ref="approvalConfirmDialogRef" />
    <div v-if="!props.embedded" class="glass-chrome" aria-hidden="true" />
    <header v-if="!props.embedded" class="titlebar">
      <div class="titlebar-drag" data-tauri-drag-region @mousedown="onWindowDragMouseDown">
        <Settings2 class="titlebar-icon" :size="15" />
        <span class="titlebar-title" data-tauri-drag-region>{{ t.title }}</span>
      </div>

      <div class="titlebar-actions">
        <button
          type="button"
          class="titlebar-btn"
          :aria-label="t.minimize"
          @mousedown.prevent="minimize"
        >
          <Minus class="size-3.5" />
        </button>
        <button
          type="button"
          class="titlebar-btn close"
          :aria-label="t.close"
          @mousedown.prevent="close"
        >
          <X class="size-3.5" />
        </button>
      </div>
    </header>

    <div class="settings-body">
      <SidebarProvider
        class="settings-layout h-full min-h-0 w-full !min-h-0 [&_[data-slot=sidebar-wrapper]]:h-full [&_[data-slot=sidebar-wrapper]]:min-h-0"
      >
        <Sidebar collapsible="none" class="settings-nav">
          <SidebarContent class="settings-nav-content peek-scrollbar">
            <input
              v-model="settingsQuery"
              class="settings-search-input"
              type="search"
              :placeholder="settingStore.language === 'zh-CN' ? '搜索设置…' : 'Search settings…'"
              :aria-label="settingStore.language === 'zh-CN' ? '搜索设置' : 'Search settings'"
              @keydown.esc="settingsQuery = ''"
            />
            <p v-if="!filteredSections.length" class="settings-search-empty" role="status">
              {{ settingStore.language === "zh-CN" ? "没有匹配的设置" : "No matching settings" }}
            </p>
            <SidebarGroup
              v-for="section in filteredSections"
              :key="section.id"
              class="settings-nav-group"
            >
              <SidebarGroupLabel class="settings-section-label">
                {{ section.label }}
              </SidebarGroupLabel>
              <SidebarMenu>
                <SidebarMenuItem v-for="navItem in section.categories" :key="navItem.id">
                  <SidebarMenuButton
                    class="settings-nav-item"
                    :is-active="activeCategory === navItem.id"
                    :title="navItem.label"
                    @click="activeCategory = navItem.id as CategoryId"
                  >
                    <component :is="navItem.icon" class="size-4 shrink-0" />
                    <span class="settings-nav-label">{{ navItem.label }}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </SidebarMenu>
            </SidebarGroup>
          </SidebarContent>
        </Sidebar>

        <SidebarInset class="settings-content-pane peek-pane">
          <div class="settings-scroll peek-scrollbar">
            <!-- No JS Transition: animated out-in + GSAP can leave this pane blank forever
                 when done() never fires (seen as white-screen/freeze on some WebView2 installs). -->
            <div :key="activeCategory" class="settings-panel">
              <p
                v-if="settingsQuery.trim() && !filteredSections.length"
                class="settings-search-empty"
                role="status"
              >
                {{
                  settingStore.language === "zh-CN"
                    ? "没有匹配的设置，请换一个关键词。"
                    : "No matching settings. Try another keyword."
                }}
              </p>
              <WorkspaceSettings v-else-if="activeCategory === 'workspace'" />
              <ProviderSettings v-else-if="activeCategory === 'provider'" />
              <ImageSettings v-else-if="activeCategory === 'image'" />
              <RagSettings v-else-if="activeCategory === 'rag'" />
              <HistorySettings
                v-else-if="activeCategory === 'history'"
                :expanded-history-groups="expandedHistoryGroups"
                @toggle-history-group="toggleHistoryGroup"
              />
              <ArchiveSettings v-else-if="activeCategory === 'archive'" />
              <TokenUsageSettings v-else-if="activeCategory === 'usage'" />
              <AboutSettings
                v-else-if="activeCategory === 'about'"
                :name="appName"
                :version="appVersion"
                :identifier="appIdentifier"
              />
              <PetSettings v-else-if="activeCategory === 'pet'" />
              <SettingFieldList
                v-else
                :items="visibleItems"
                :searching="Boolean(settingsQuery.trim())"
                :page-title="fieldPageTitle"
                :page-description="fieldPageDescription"
                :empty-text="t.empty"
                v-model:api-key-draft="apiKeyDraft"
                v-model:mem0-api-key-draft="mem0ApiKeyDraft"
                v-model:mem0-user-id-draft="mem0UserIdDraft"
                v-model:mem0-base-url-draft="mem0BaseUrlDraft"
                v-model:serper-api-key-draft="serperApiKeyDraft"
                v-model:tavily-api-key-draft="tavilyApiKeyDraft"
                @toggle="onToggle"
                @slider-change="onSliderChange"
                @color-scheme-change="onColorSchemeChange"
                @language-change="onLanguageChange"
                @zoom-change="onZoomChange"
                @reasoning-effort-change="onReasoningEffortChange"
                @reasoning-language-change="onReasoningLanguageChange"
                @tool-approval-mode-change="onToolApprovalModeChange"
                @agent-work-display-change="onAgentWorkDisplayChange"
                @web-search-provider-change="onWebSearchProviderChange"
                @default-model-change="onDefaultModelChange"
                @multimodal-model-change="onMultimodalModelChange"
                @image-model-change="onImageModelChange"
                @save-api-key="saveApiKey"
                @save-memory-settings="saveMemorySettings"
                @save-web-search-settings="saveWebSearchSettings"
              >
                <template #after-first-group>
                  <CustomThemeManager
                    v-if="activeCategory === 'appearance' && !settingsQuery.trim()"
                  />
                </template>
              </SettingFieldList>
            </div>
          </div>
        </SidebarInset>
      </SidebarProvider>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch, type Component } from "vue";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  Archive,
  Bot,
  BrainCircuit,
  Shield,
  Folders,
  Globe2,
  History,
  Image as ImageIcon,
  Info,
  Minus,
  Palette,
  Pin,
  Search,
  Server,
  Settings2,
  Sparkles,
  X,
  BarChart3,
} from "@lucide/vue";
import WorkspaceSettings from "@/components/workspace/WorkspaceSettings.vue";
import HistorySettings from "@/components/settings/HistorySettings.vue";
import ArchiveSettings from "@/components/settings/ArchiveSettings.vue";
import TokenUsageSettings from "@/components/settings/TokenUsageSettings.vue";
import AboutSettings from "@/components/settings/AboutSettings.vue";
import ProviderSettings from "@/components/settings/ProviderSettings.vue";
import RagSettings from "@/components/settings/RagSettings.vue";
import PetSettings from "@/components/settings/PetSettings.vue";
import ImageSettings from "@/components/settings/ImageSettings.vue";
import SettingFieldList from "@/components/settings/SettingFieldList.vue";
import CustomThemeManager from "@/components/settings/CustomThemeManager.vue";
import { onWindowDragMouseDown } from "@/services/overlay/windowDrag";
import { applyOpacity, reloadAppearanceWindow } from "@/services/overlay/appearance";
import { gsapSettingsNavMount, gsapSettingsNavUnmount } from "@/services/motion/gsapPresets";
import { getAppInfo, relaunchApp, webviewGpuDisabled } from "@/services/ipc";
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
} from "@/components/ui/sidebar";
import { useSettingStore } from "@/stores/setting";
import { useChatModelStore } from "@/stores/chatModel";
import { tr } from "@/services/i18n";
import { fullApprovalConfirmOptions } from "@/services/chat/fullApprovalConfirm";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import type { SettingsI18nKey } from "@/services/locales/settings";
import {
  buildSettingDefinitions,
  type CategoryId,
  type SettingDefinition,
} from "@/pages/Settings/settingsDefinitions";
import {
  DEFAULT_SETTINGS_CATEGORY,
  type AppLanguage,
  type ColorScheme,
  type ReasoningLanguage,
  type ModelSelection,
  type WebSearchProvider,
  type ToolApprovalMode,
  type AgentWorkDisplay,
  isReasoningEffort,
} from "@/types/setting";

const settingStore = useSettingStore();
const chatModelStore = useChatModelStore();
const approvalConfirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const appWindow = getCurrentWebviewWindow();
const props = withDefaults(
  defineProps<{
    embedded?: boolean;
    /** When set (e.g. deep-link from chat CTA), open this category. */
    category?: CategoryId;
  }>(),
  {
    embedded: false,
  },
);

const hasCustomBg = computed(() => {
  const currentThemeId = settingStore.colorScheme;
  const customTheme = settingStore.customThemes.find((t) => t.id === currentThemeId);
  return Boolean(customTheme?.background?.image || settingStore.customBackground?.image);
});

const SETTINGS_BASE_WIDTH = 880;
const SETTINGS_BASE_HEIGHT = 620;

async function resizeSettingsWindow() {
  const zoom = (settingStore.zoom || 100) / 100;
  const scaledWidth = SETTINGS_BASE_WIDTH * zoom;
  const scaledHeight = SETTINGS_BASE_HEIGHT * zoom;
  await appWindow.setSize(new LogicalSize(scaledWidth, scaledHeight));
}

function resolveSettingsCategory(category?: CategoryId): CategoryId {
  if (!category || category === "mcp" || category === "skills") {
    return DEFAULT_SETTINGS_CATEGORY;
  }
  return category;
}

const activeCategory = ref<CategoryId>(resolveSettingsCategory(props.category));

watch(
  () => props.category,
  (category) => {
    if (category) activeCategory.value = resolveSettingsCategory(category);
  },
);

const appName = ref("-");
const appVersion = ref("-");
const appIdentifier = ref("-");
const apiKeyDraft = ref("");
const mem0ApiKeyDraft = ref("");
const mem0UserIdDraft = ref("");
const mem0BaseUrlDraft = ref("");
const serperApiKeyDraft = ref("");
const tavilyApiKeyDraft = ref("");
let settingsNavEl: Element | null = null;
const expandedHistoryGroups = ref<Record<string, boolean>>({});

function isHistoryGroupExpanded(groupId: string) {
  return expandedHistoryGroups.value[groupId] !== false;
}

function toggleHistoryGroup(groupId: string) {
  expandedHistoryGroups.value = {
    ...expandedHistoryGroups.value,
    [groupId]: !isHistoryGroupExpanded(groupId),
  };
}

const t = computed(() => {
  const language = settingStore.language;
  return {
    title: tr(language, "settings.title"),
    minimize: tr(language, "settings.minimize"),
    close: tr(language, "settings.close"),
    sidebarLabel: tr(language, "settings.sidebarLabel"),
    empty: tr(language, "settings.empty"),
    categories: {
      appearance: tr(language, "settings.categories.appearance"),
      ai: tr(language, "settings.categories.ai"),
      image: tr(language, "settings.categories.image"),
      memory: tr(language, "settings.categories.memory"),
      search: tr(language, "settings.categories.search"),
      agent: tr(language, "settings.categories.agent"),
      pinTools: tr(language, "settings.categories.pinTools"),
      workspace: tr(language, "settings.categories.workspace"),
      history: tr(language, "settings.categories.history"),
      archive: tr(language, "settings.categories.archive"),
      usage: tr(language, "settings.categories.usage"),
      about: tr(language, "settings.categories.about"),
      provider: tr(language, "settings.categories.provider"),
      rag: tr(language, "settings.categories.rag"),
      pet: tr(language, "settings.categories.pet"),
    },
  };
});

const categories = computed(() => [
  { id: "ai" as const, label: t.value.categories.ai, icon: Bot },
  { id: "image" as const, label: t.value.categories.image, icon: ImageIcon },
  { id: "provider" as const, label: t.value.categories.provider, icon: Server },
  {
    id: "workspace" as const,
    label: t.value.categories.workspace,
    icon: Folders,
  },
  {
    id: "pet" as const,
    label: t.value.categories.pet,
    icon: Sparkles,
  },
  { id: "agent" as const, label: t.value.categories.agent, icon: Shield },
  { id: "history" as const, label: t.value.categories.history, icon: History },
  { id: "archive" as const, label: t.value.categories.archive, icon: Archive },
  { id: "usage" as const, label: t.value.categories.usage, icon: BarChart3 },
  { id: "pinTools" as const, label: t.value.categories.pinTools, icon: Pin },
  {
    id: "memory" as const,
    label: t.value.categories.memory,
    icon: BrainCircuit,
  },
  { id: "search" as const, label: t.value.categories.search, icon: Globe2 },
  { id: "rag" as const, label: t.value.categories.rag, icon: Search },
  {
    id: "appearance" as const,
    label: t.value.categories.appearance,
    icon: Palette,
  },
  { id: "about" as const, label: t.value.categories.about, icon: Info },
]);

type SettingsNavItem = { id: string; label: string; icon?: Component };
const settingsQuery = ref("");
const matchesSetting = (item: SettingDefinition) =>
  [item.title, item.description, item.path, ...item.keywords]
    .join(" ")
    .toLocaleLowerCase()
    .includes(settingsQuery.value.trim().toLocaleLowerCase());
const filteredSections = computed(() =>
  categorySections.value
    .map((section) => ({
      ...section,
      categories: section.categories.filter(
        (category) =>
          !settingsQuery.value.trim() ||
          category.label
            .toLocaleLowerCase()
            .includes(settingsQuery.value.trim().toLocaleLowerCase()) ||
          settingDefinitions.value.some(
            (item) => item.category === category.id && matchesSetting(item),
          ),
      ),
    }))
    .filter((section) => section.categories.length),
);

const categorySections = computed(() => {
  const byId = new Map(categories.value.map((category) => [category.id, category]));
  type SettingsNavId = (typeof categories.value)[number]["id"];
  const section = (
    id: string,
    label: string,
    ids: SettingsNavId[],
  ): { id: string; label: string; categories: SettingsNavItem[] } => ({
    id,
    label,
    categories: ids.flatMap((categoryId) => {
      const category = byId.get(categoryId);
      return category ? [category] : [];
    }),
  });
  const language = settingStore.language;
  return [
    section("general", tr(language, "settings.sections.general"), [
      "appearance",
      "pet",
      "workspace",
    ]),
    section("intelligence", tr(language, "settings.sections.intelligence"), [
      "provider",
      "ai",
      "image",
      "pinTools",
      "agent",
      "memory",
      "search",
      "rag",
    ]),
    section("data", tr(language, "settings.sections.data"), ["history", "archive", "usage"]),
    section("system", tr(language, "settings.sections.system"), ["about"]),
  ];
});

const settingDefinitions = computed<SettingDefinition[]>(() =>
  buildSettingDefinitions(settingStore.language, {
    appName: appName.value,
    appVersion: appVersion.value,
    appIdentifier: appIdentifier.value,
  }),
);

const visibleItems = computed(() =>
  settingDefinitions.value.filter(
    (item) =>
      item.category === activeCategory.value &&
      (!settingsQuery.value.trim() ||
        matchesSetting(item) ||
        categories.value
          .find((category) => category.id === activeCategory.value)
          ?.label.toLocaleLowerCase()
          .includes(settingsQuery.value.trim().toLocaleLowerCase())),
  ),
);
watch(filteredSections, (sections) => {
  if (!settingsQuery.value.trim()) return;
  const results = sections.flatMap((section) => section.categories);
  if (results.length && !results.some((category) => category.id === activeCategory.value)) {
    activeCategory.value = results[0].id as CategoryId;
  }
});

const fieldPageTitle = computed(() => {
  const id = activeCategory.value;
  return t.value.categories[id as keyof typeof t.value.categories] ?? "";
});

const FIELD_LIST_PAGE_DESC_KEYS = new Set([
  "appearance",
  "ai",
  "agent",
  "memory",
  "search",
  "pinTools",
]);

const fieldPageDescription = computed(() => {
  const id = activeCategory.value;
  if (!FIELD_LIST_PAGE_DESC_KEYS.has(id)) return "";
  return tr(settingStore.language, `settings.pages.${id}.description` as SettingsI18nKey);
});

function minimize() {
  void appWindow.minimize();
}

function close() {
  void appWindow.hide();
}

function onColorSchemeChange(value: unknown) {
  if (typeof value !== "string") {
    return;
  }
  const scheme = value.startsWith("builtin:")
    ? value.slice("builtin:".length)
    : value.startsWith("custom:")
      ? value.slice("custom:".length)
      : value;
  if (!scheme || scheme === settingStore.colorScheme) {
    return;
  }
  void (async () => {
    await settingStore.update({
      colorScheme: scheme as ColorScheme,
    });
    reloadAppearanceWindow();
  })();
}

function onLanguageChange(value: unknown) {
  if (typeof value !== "string") {
    return;
  }
  void settingStore.update({ language: value as AppLanguage });
}

function onZoomChange(value: unknown) {
  if (typeof value !== "string") {
    return;
  }
  const zoomVal = parseFloat(value);
  if (isNaN(zoomVal)) {
    return;
  }
  void settingStore.update({ zoom: zoomVal });
}

function onReasoningEffortChange(value: unknown) {
  if (typeof value !== "string" || !isReasoningEffort(value)) {
    return;
  }
  void settingStore.update({ reasoningEffort: value });
}

function onReasoningLanguageChange(value: unknown) {
  if (typeof value !== "string") {
    return;
  }
  void settingStore.update({ reasoningLanguage: value as ReasoningLanguage });
}

let opacityDebounceTimer: ReturnType<typeof setTimeout> | null = null;

function onSliderChange(id: string, value: number) {
  if (id === "opacity") {
    void applyOpacity(value);
    if (opacityDebounceTimer) clearTimeout(opacityDebounceTimer);
    opacityDebounceTimer = setTimeout(() => {
      void settingStore.update({ opacity: value });
    }, 120);
  }
}

async function saveApiKey() {
  if (apiKeyDraft.value === settingStore.deepseekApiKey) {
    return;
  }
  await settingStore.update({ deepseekApiKey: apiKeyDraft.value.trim() });
  await chatModelStore.refresh();
}

function isModelSelection(value: unknown): value is ModelSelection {
  if (!value || typeof value !== "object") return false;
  const selection = value as Partial<ModelSelection>;
  return typeof selection.id === "string" && typeof selection.provider === "string";
}

function onDefaultModelChange(value: unknown) {
  if (!isModelSelection(value) || !value.id.trim()) return;
  void settingStore.update({
    chatModel: value.id,
    chatModelProvider: value.provider,
  });
}

function onMultimodalModelChange(value: unknown) {
  if (!isModelSelection(value) || !value.id.trim()) return;
  void settingStore.update({
    multimodalModel: value.id,
    multimodalModelProvider: value.provider,
  });
}

function onImageModelChange(value: unknown) {
  if (!isModelSelection(value) || !value.id.trim()) return;
  void settingStore.update({
    imageModel: value.id,
    imageModelProvider: value.provider,
  });
}

function onToggle(id: string) {
  if (id === "hardwareAccelerationEnabled") {
    void settingStore.update({
      hardwareAccelerationEnabled: !settingStore.hardwareAccelerationEnabled,
    });
  }
  if (id === "chromeFrostedGlass") {
    const enabling = !settingStore.chromeFrostedGlass;
    void (async () => {
      await settingStore.update({ chromeFrostedGlass: enabling });
      // Software WebView2 cannot blend transparent chrome with DWM Acrylic.
      if (enabling && (await webviewGpuDisabled())) {
        await relaunchApp();
      }
    })();
  }
  if (id === "memoryEnabled") {
    void settingStore.update({ memoryEnabled: !settingStore.memoryEnabled });
  }
  if (id === "webSearchEnabled") {
    void settingStore.update({
      webSearchEnabled: !settingStore.webSearchEnabled,
      serperApiKey: serperApiKeyDraft.value.trim(),
      tavilyApiKey: tavilyApiKeyDraft.value.trim(),
    });
  }
  if (id === "lspEnabled") {
    void settingStore.update({ lspEnabled: !settingStore.lspEnabled });
  }
  if (id === "passToolReasoning") {
    void settingStore.update({
      passToolReasoning: !settingStore.passToolReasoning,
    });
  }
  if (id === "continueThinkingAfterTools") {
    void settingStore.update({
      continueThinkingAfterTools: !settingStore.continueThinkingAfterTools,
    });
  }
  if (id === "showReasoning") {
    void settingStore.update({ showReasoning: !settingStore.showReasoning });
  }
  if (id === "multimodalSplitAnalysis") {
    void settingStore.update({
      multimodalSplitAnalysis: !settingStore.multimodalSplitAnalysis,
    });
  }
  if (id === "largeContextEnabled") {
    void settingStore.update({
      largeContextEnabled: !settingStore.largeContextEnabled,
    });
  }
  if (id === "pixpinPinAiEnabled") {
    void settingStore.update({
      pixpinPinAiEnabled: !settingStore.pixpinPinAiEnabled,
    });
  }
  if (id === "snipastePinAiEnabled") {
    void settingStore.update({
      snipastePinAiEnabled: !settingStore.snipastePinAiEnabled,
    });
  }
  if (id === "minimalCoding") {
    void settingStore.update({
      minimalCoding: !settingStore.minimalCoding,
    });
  }
}

async function onToolApprovalModeChange(value: unknown) {
  if (value !== "ask" && value !== "auto" && value !== "alwaysAllow") return;
  if (value === settingStore.toolApprovalMode) return;
  if (value === "alwaysAllow") {
    const confirmed = await approvalConfirmDialogRef.value?.ask(
      fullApprovalConfirmOptions(settingStore.language),
    );
    if (!confirmed) return;
  }
  void settingStore.update({ toolApprovalMode: value as ToolApprovalMode });
}

function onAgentWorkDisplayChange(value: unknown) {
  if (value !== "detailed" && value !== "compact") return;
  void settingStore.update({ agentWorkDisplay: value as AgentWorkDisplay });
}

function saveMemorySettings() {
  void settingStore.update({
    mem0ApiKey: mem0ApiKeyDraft.value.trim(),
    mem0UserId: mem0UserIdDraft.value.trim() || "peek-user",
    mem0BaseUrl: mem0BaseUrlDraft.value.trim() || "https://api.mem0.ai/v1",
  });
}

function saveWebSearchSettings() {
  void settingStore.update({
    serperApiKey: serperApiKeyDraft.value.trim(),
    tavilyApiKey: tavilyApiKeyDraft.value.trim(),
  });
}

function onWebSearchProviderChange(value: unknown) {
  if (value !== "serper" && value !== "tavily") return;
  void settingStore.update({
    webSearchProvider: value as WebSearchProvider,
    serperApiKey: serperApiKeyDraft.value.trim(),
    tavilyApiKey: tavilyApiKeyDraft.value.trim(),
  });
}

onMounted(async () => {
  apiKeyDraft.value = settingStore.deepseekApiKey;
  mem0ApiKeyDraft.value = settingStore.mem0ApiKey;
  mem0UserIdDraft.value = settingStore.mem0UserId;
  mem0BaseUrlDraft.value = settingStore.mem0BaseUrl;
  serperApiKeyDraft.value = settingStore.serperApiKey;
  tavilyApiKeyDraft.value = settingStore.tavilyApiKey;
  void chatModelStore.fetch();

  const info = await getAppInfo();
  appName.value = info.name;
  appVersion.value = info.version;
  appIdentifier.value = info.identifier;

  if (!props.embedded) {
    await resizeSettingsWindow();
  }

  await nextTick();
  const navEl = document.querySelector(".settings-nav");
  settingsNavEl = navEl;
  if (navEl) gsapSettingsNavMount(navEl);
});

onUnmounted(() => {
  if (opacityDebounceTimer) {
    clearTimeout(opacityDebounceTimer);
    opacityDebounceTimer = null;
  }
  gsapSettingsNavUnmount(settingsNavEl);
  settingsNavEl = null;
});

watch(
  () => settingStore.zoom,
  async () => {
    if (!props.embedded) {
      await resizeSettingsWindow();
    }
  },
);
</script>

<style scoped>
.settings-search-input {
  box-sizing: border-box;
  width: calc(100% - 20px);
  margin: 12px 10px 4px;
  padding: 9px 10px;
  border: 1px solid var(--peek-border);
  border-radius: 8px;
  background: var(--peek-surface);
  color: var(--peek-text);
  font-size: 13px;
}
.settings-search-input:focus-visible {
  outline: 2px solid var(--peek-accent);
  outline-offset: 2px;
}
.settings-search-empty {
  padding: 10px 14px;
  font-size: 13px;
  color: var(--peek-muted);
}
.settings-workbench {
  --settings-chrome-bg: color-mix(in srgb, var(--peek-sidebar) 92%, var(--peek-bg));
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--settings-chrome-bg);
  color: var(--peek-text);
  font-family: var(--font-sans);
  container-type: inline-size;
  container-name: settings;
}

.settings-workbench.embedded {
  --settings-chrome-bg: var(--workbench-chrome-bg, var(--peek-sidebar));
}

.titlebar {
  flex: none;
  height: 42px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--settings-chrome-bg);
  user-select: none;
}

.titlebar-drag {
  min-width: 0;
  height: 100%;
  display: flex;
  flex: 1;
  align-items: center;
  gap: 9px;
  padding: 0 14px;
}

.titlebar-icon {
  flex: none;
  color: var(--peek-muted);
}

.titlebar-title {
  overflow: hidden;
  font-size: 13px;
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.titlebar-actions {
  height: 100%;
  display: flex;
  align-items: center;
}

.settings-body {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  background: var(--settings-chrome-bg);
}

:global(html.chrome-frosted-glass) .settings-workbench,
:global(html.chrome-frosted-glass) .settings-workbench.embedded,
:global(html.chrome-frosted-glass) .settings-body {
  background: transparent;
}

.settings-nav {
  width: var(--nav-col, 250px) !important;
  min-width: var(--nav-col, 250px) !important;
  max-width: var(--nav-col, 250px) !important;
  flex: none;
  background: var(--settings-chrome-bg);
  transition:
    width 160ms ease,
    min-width 160ms ease;
}

.settings-content-pane {
  position: relative;
  z-index: 1;
  min-width: 0;
  min-height: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: visible;
  background: var(--peek-list-bg) !important;
}

.settings-workbench.is-glass,
.settings-workbench.is-glass.embedded,
.settings-workbench.is-glass .settings-body,
.settings-workbench.is-glass .titlebar,
.settings-workbench.is-glass .settings-nav,
.settings-workbench.is-glass .settings-content-pane,
.settings-workbench.is-glass :deep([data-slot="sidebar"]),
.settings-workbench.is-glass :deep([data-slot="sidebar-wrapper"]),
.settings-workbench.has-custom-bg,
.settings-workbench.has-custom-bg.embedded,
.settings-workbench.has-custom-bg .settings-body,
.settings-workbench.has-custom-bg .titlebar,
.settings-workbench.has-custom-bg .settings-nav,
.settings-workbench.has-custom-bg .settings-content-pane,
.settings-workbench.has-custom-bg :deep([data-slot="sidebar"]),
.settings-workbench.has-custom-bg :deep([data-slot="sidebar-wrapper"]) {
  background: transparent !important;
  box-shadow: none;
}

.settings-workbench :deep([data-slot="sidebar-wrapper"]) {
  min-height: 0;
  height: 100%;
  overflow: visible;
}

.settings-workbench.is-glass:not(.embedded) {
  position: relative;
}

.settings-workbench.is-glass:not(.embedded) .glass-chrome {
  display: block;
  position: absolute;
  inset: 0;
  z-index: 0;
  pointer-events: none;
  background: var(--workbench-glass-fill);
  backdrop-filter: blur(20px) saturate(1.25);
  -webkit-backdrop-filter: blur(20px) saturate(1.25);
}

.settings-workbench.is-glass:not(.embedded) .titlebar,
.settings-workbench.is-glass:not(.embedded) .settings-body {
  position: relative;
  z-index: 1;
}

.glass-chrome {
  display: none;
}

.settings-scroll {
  flex: 1;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  padding-right: 1px;
  display: flex;
  flex-direction: column;
  border-radius: var(--peek-radius-lg) 0 0 0;
}

.settings-nav-content {
  overflow-y: auto;
  padding: 5px 4px 8px;
}

.settings-nav-group {
  padding: 0 4px 4px;
}

.settings-section-label {
  height: 25px;
  padding: 0 8px;
  color: var(--peek-faint);
  font-size: var(--peek-font-xs);
  font-weight: 650;
}

.settings-nav :deep([data-slot="sidebar-menu-button"]),
.settings-nav-item {
  position: relative;
  height: var(--peek-control-row);
  gap: 8px;
  padding: 0 8px 0 10px;
  border-radius: var(--peek-radius-sm);
  background: transparent;
  color: var(--peek-muted);
  font-size: var(--peek-font-sm);
  letter-spacing: 0;
}

.settings-nav :deep([data-slot="sidebar-menu-button"]:hover) {
  background: var(--peek-row-hover);
  color: var(--peek-text);
}

.settings-nav :deep([data-slot="sidebar-menu-button"][data-active="true"]) {
  background: var(--peek-row-active);
  color: var(--peek-text);
  font-weight: 600;
}

.settings-nav :deep([data-slot="sidebar-menu-button"][data-active="true"]::before) {
  content: "";
  position: absolute;
  left: 0;
  top: 6px;
  bottom: 6px;
  width: 2px;
  border-radius: 1px;
  background: var(--peek-accent);
}

.settings-nav-label {
  display: inline-block;
  min-width: 2.75em;
  font-variant-numeric: tabular-nums;
}

.settings-panel {
  flex: 1;
  min-height: 100%;
  width: 100%;
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: stretch;
}

.titlebar-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 42px;
  height: 100%;
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--peek-muted);
  cursor: default;
}

.titlebar-btn:hover {
  background: var(--peek-hover-bg);
  color: var(--peek-text);
}

.titlebar-btn.close:hover {
  background: var(--peek-danger);
  color: var(--peek-primary-foreground, #fff);
}
</style>
