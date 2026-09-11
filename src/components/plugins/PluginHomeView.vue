<template>
  <section class="plugin-home">
    <AppConfirmDialog ref="confirmDialogRef" />
    <header class="home-header">
      <button
        type="button"
        class="back-btn"
        :aria-label="copy.back"
        :title="copy.back"
        @click="emit('back')"
      >
        <ArrowLeft :size="16" />
      </button>
      <div v-if="plugin" class="home-icon">
        <PluginSidebarIcon :name="iconSrc(plugin)" :size="22" />
      </div>
      <div v-if="plugin" class="home-title">
        <div class="home-name">
          <strong>{{ plugin.name }}</strong>
          <span v-if="!plugin.apiSupported" class="row-warn" :title="copy.apiUnsupported">!</span>
        </div>
        <p class="home-meta">
          {{ plugin.id }} · v{{ plugin.version }} · api {{ plugin.apiVersion }}
        </p>
      </div>
      <div v-if="plugin" class="home-side">
        <label class="row-toggle" :title="plugin.enabled ? copy.disable : copy.enable">
          <input type="checkbox" :checked="plugin.enabled" @change="onToggle" />
          <span class="toggle-track"><span class="toggle-thumb" /></span>
        </label>
        <button
          v-if="plugin.enabled"
          type="button"
          class="home-icon-btn"
          :aria-label="copy.reload"
          :title="copy.reload"
          @click="reload(plugin.id)"
        >
          <RefreshCw :size="15" :stroke-width="1.75" />
        </button>
        <button
          type="button"
          class="home-icon-btn"
          :aria-label="copy.exportPack"
          :title="copy.exportPack"
          @click="exportPack(plugin)"
        >
          <Download :size="15" :stroke-width="1.75" />
        </button>
        <button
          v-if="!plugin.official"
          type="button"
          class="home-icon-btn is-danger"
          :aria-label="copy.remove"
          :title="copy.remove"
          @click="onRemove(plugin.id)"
        >
          <Trash2 :size="15" :stroke-width="1.75" />
        </button>
      </div>
    </header>

    <p v-if="!plugin && pluginsStore.pluginsLoaded" class="home-missing">{{ copy.missing }}</p>

    <template v-else-if="plugin">
      <div v-if="!plugin.enabled" class="home-grant">
        <span>{{ copy.grant }}</span>
        <label v-for="perm in plugin.permissions" :key="perm" class="grant-item">
          <input v-model="pending[plugin.id]" type="checkbox" :value="perm" />
          {{ perm }}
        </label>
        <p v-if="!plugin.permissions.length" class="muted">{{ copy.perms }}: —</p>
      </div>

      <div v-if="plugin.contributes?.window" class="home-actions">
        <button type="button" @click="openWindow(plugin.id)">{{ copy.window }}</button>
        <button type="button" @click="closeWindow(plugin.id)">{{ copy.closeWindow }}</button>
      </div>

      <div v-if="plugin.hasUi" class="home-tabs" role="tablist">
        <button
          type="button"
          class="home-tab"
          :class="{ active: activeTab === 'about' }"
          role="tab"
          @click="activeTab = 'about'"
        >
          {{ copy.about }}
        </button>
        <button
          type="button"
          class="home-tab"
          :class="{ active: activeTab === 'settings' }"
          role="tab"
          @click="activeTab = 'settings'"
        >
          {{ copy.settings }}
        </button>
      </div>

      <div class="home-body">
        <!-- eslint-disable-next-line vue/no-v-html -->
        <div
          v-if="activeTab === 'about' && aboutHtml"
          class="home-about markdown-body"
          v-html="aboutHtml"
        />
        <p v-else-if="activeTab === 'about'" class="home-about-plain">
          {{ plugin.description || copy.noDesc }}
        </p>
        <template v-else-if="plugin.hasUi && activeTab === 'settings'">
          <PluginHostPane
            v-if="plugin.enabled && settingsMount"
            :mount-fn="settingsMount"
            :plugin-id="plugin.id"
            fit="content"
          />
          <p v-else-if="!plugin.enabled" class="home-settings-hint">{{ copy.enableForSettings }}</p>
          <p v-else class="home-settings-hint">{{ copy.noSettings }}</p>
        </template>
      </div>
    </template>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import DOMPurify from "dompurify";
import { marked } from "marked";
import { ArrowLeft, Download, RefreshCw, Trash2 } from "@lucide/vue";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import PluginHostPane from "@/components/plugins/PluginHostPane.vue";
import PluginSidebarIcon from "@/components/plugins/PluginSidebarIcon.vue";
import { usePluginActions } from "@/composables/plugins/usePluginActions";
import { usePluginsStore } from "@/stores/plugins";
import { useSettingStore } from "@/stores/setting";
import { pluginAssetUrl, pluginIconUrl } from "@/services/plugins/ipc";

const props = defineProps<{ pluginId: string }>();
const emit = defineEmits<{ back: [] }>();

const pluginsStore = usePluginsStore();
const settingStore = useSettingStore();
const {
  pending,
  ensurePending,
  iconSrc,
  reload,
  openWindow,
  closeWindow,
  remove,
  toggle,
  exportPack,
} = usePluginActions();

const plugin = computed(() => pluginsStore.plugins.find((p) => p.id === props.pluginId));

const copy = computed(() =>
  settingStore.language === "zh-CN"
    ? {
        back: "返回",
        missing: "找不到这个插件。",
        about: "详情",
        settings: "设置",
        noDesc: "无简介",
        perms: "声明的权限",
        grant: "启用时授权",
        enable: "启用",
        disable: "停用",
        reload: "重载",
        exportPack: "导出",
        window: "窗口",
        closeWindow: "关闭窗口",
        remove: "卸载",
        removeTitle: "卸载插件",
        removeDesc: `确定卸载「${plugin.value?.name ?? plugin.value?.id}」吗？插件文件夹会被删除，此操作无法撤销。`,
        cancel: "取消",
        apiUnsupported: "此插件的契约版本高于/不同于当前支持版本，行为可能异常",
        enableForSettings: "启用插件后才能在这里配置。",
        noSettings: "这个插件没有可配置项。",
      }
    : {
        back: "Back",
        missing: "Plugin not found.",
        about: "About",
        settings: "Settings",
        noDesc: "No description",
        perms: "Declared permissions",
        grant: "Grant on enable",
        enable: "Enable",
        disable: "Disable",
        reload: "Reload",
        exportPack: "Export",
        window: "Window",
        closeWindow: "Close window",
        remove: "Uninstall",
        removeTitle: "Uninstall plugin",
        removeDesc: `Uninstall “${plugin.value?.name ?? plugin.value?.id}”? The plugin folder will be deleted. This cannot be undone.`,
        cancel: "Cancel",
        apiUnsupported:
          "This plugin targets a contract version Anya doesn't fully support; behavior may be off.",
        enableForSettings: "Enable this plugin to configure it here.",
        noSettings: "This plugin has nothing to configure.",
      },
);

const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const activeTab = ref<"about" | "settings">("about");

watch(
  plugin,
  (value) => {
    if (value) ensurePending(value);
    if (value && !value.hasUi) activeTab.value = "about";
  },
  { immediate: true },
);

async function onToggle() {
  if (plugin.value) await toggle(plugin.value);
}

async function onRemove(id: string) {
  const confirmed = await confirmDialogRef.value?.ask({
    title: copy.value.removeTitle,
    description: copy.value.removeDesc,
    confirmLabel: copy.value.remove,
    cancelLabel: copy.value.cancel,
    tone: "danger",
  });
  if (!confirmed) return;
  await remove(id);
  emit("back");
}

const settingsMount = computed(() => pluginsStore.homeSettingsViews.get(props.pluginId));

/** Fetches the manifest's `about` file and renders it sanitized; falls back to plain `description`. */
const aboutHtml = ref("");

watch(
  plugin,
  async (value) => {
    aboutHtml.value = "";
    if (!value?.about) return;
    try {
      const res = await fetch(pluginAssetUrl(pluginIconUrl(value.id, value.about)));
      const text = await res.text();
      const isMarkdown = /\.md$/i.test(value.about);
      const raw = isMarkdown ? await marked.parse(text) : text;
      aboutHtml.value = DOMPurify.sanitize(raw);
    } catch {
      aboutHtml.value = "";
    }
  },
  { immediate: true },
);
</script>

<style scoped>
.plugin-home {
  display: flex;
  flex-direction: column;
  gap: 10px;
  flex: 1;
  min-height: 0;
}
.home-header {
  display: flex;
  align-items: center;
  gap: 10px;
}
.back-btn {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  flex: none;
  padding: 0;
  border: none;
  border-radius: 999px;
  background: transparent;
  color: var(--peek-muted, #8b939e);
  cursor: pointer;
}
.back-btn:hover {
  color: inherit;
  background: var(--peek-surface-muted, rgba(255, 255, 255, 0.08));
}
.home-icon {
  flex: none;
  width: 40px;
  height: 40px;
  border-radius: 12px;
  border: 1.5px solid var(--peek-border, rgba(255, 255, 255, 0.18));
  display: grid;
  place-items: center;
}
.home-title {
  flex: 1;
  min-width: 0;
}
.home-name {
  font-size: 15px;
  font-weight: 650;
}
.row-warn {
  margin-left: 6px;
  color: #fbbf24;
  font-weight: 700;
}
.home-meta {
  margin: 2px 0 0;
  font-size: 11.5px;
  color: var(--peek-muted, #8b939e);
}
.home-side {
  flex: none;
  display: flex;
  align-items: center;
  gap: 2px;
}
.home-side .row-toggle {
  margin-right: 6px;
}
.home-icon-btn {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  flex: none;
  padding: 0;
  border: none;
  border-radius: 999px;
  background: transparent;
  color: var(--peek-muted, #8b939e);
  cursor: pointer;
}
.home-icon-btn:hover {
  color: inherit;
  background: var(--peek-surface-muted, rgba(255, 255, 255, 0.08));
}
.home-icon-btn.is-danger:hover {
  color: #f87171;
  background: rgba(248, 113, 113, 0.12);
}
.home-missing {
  color: var(--peek-muted, #8b939e);
  font-size: 13px;
}
.row-toggle {
  position: relative;
  display: inline-flex;
  cursor: pointer;
}
.row-toggle input {
  position: absolute;
  opacity: 0;
  width: 1px;
  height: 1px;
}
.toggle-track {
  width: 34px;
  height: 20px;
  border-radius: 999px;
  background: var(--peek-border, rgba(255, 255, 255, 0.18));
  display: inline-flex;
  align-items: center;
  padding: 2px;
  transition: background-color 0.15s ease;
}
.toggle-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #fff;
  transform: translateX(0);
  transition: transform 0.15s ease;
}
.row-toggle input:checked + .toggle-track {
  background: var(--peek-accent, #ef4444);
}
.row-toggle input:checked + .toggle-track .toggle-thumb {
  transform: translateX(14px);
}
.home-grant {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
}
.grant-item {
  display: flex;
  gap: 6px;
  align-items: center;
}
.muted {
  color: var(--peek-muted, #8b939e);
  font-size: 12px;
  margin: 0;
}
.home-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.home-actions button {
  font: inherit;
  font-size: 12px;
  padding: 4px 8px;
  border-radius: 6px;
  border: 1px solid var(--peek-border, rgba(255, 255, 255, 0.12));
  background: transparent;
  color: inherit;
  cursor: pointer;
}
.home-tabs {
  display: flex;
  gap: 4px;
  border-bottom: 1px solid var(--peek-border, rgba(255, 255, 255, 0.08));
}
.home-tab {
  font: inherit;
  font-size: 13px;
  padding: 6px 10px;
  border: none;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: var(--peek-muted, #8b939e);
  cursor: pointer;
}
.home-tab.active {
  color: var(--peek-text);
  border-bottom-color: var(--peek-accent, #ef4444);
  font-weight: 600;
}
.home-body {
  flex: 1 1 auto;
  min-height: 12rem;
  display: flex;
  flex-direction: column;
  overflow: auto;
}
.home-body :deep(.plugin-host-pane) {
  width: 100%;
  flex: 1 1 auto;
  overflow: auto;
  min-height: 8rem;
}
.home-about-plain,
.home-settings-hint {
  color: var(--peek-muted, #8b939e);
  font-size: 13px;
}
.home-about {
  font-size: 13px;
  line-height: 1.65;
  color: var(--peek-text);
  max-width: 42rem;
}
.home-about :deep(h1) {
  font-size: 1.25rem;
  font-weight: 650;
  margin: 0 0 8px;
}
.home-about :deep(h2) {
  font-size: 13px;
  font-weight: 650;
  margin: 18px 0 8px;
}
.home-about :deep(p),
.home-about :deep(ul),
.home-about :deep(ol) {
  margin: 0 0 10px;
}
.home-about :deep(li + li) {
  margin-top: 4px;
}
.home-about :deep(code) {
  font-size: 12px;
  padding: 1px 5px;
  border-radius: 4px;
  background: var(--peek-surface-muted, rgba(255, 255, 255, 0.06));
}
.home-about :deep(table) {
  width: 100%;
  border-collapse: collapse;
  font-size: 12.5px;
  margin: 0 0 12px;
}
.home-about :deep(th),
.home-about :deep(td) {
  text-align: left;
  padding: 6px 8px;
  border-bottom: 1px solid var(--peek-border, rgba(255, 255, 255, 0.08));
  vertical-align: top;
}
.home-about :deep(th) {
  color: var(--peek-muted, #8b939e);
  font-weight: 600;
}
.home-about :deep(hr) {
  border: none;
  border-top: 1px solid var(--peek-border, rgba(255, 255, 255, 0.08));
  margin: 16px 0;
}
</style>
