<template>
  <section class="user-plugins">
    <AppConfirmDialog ref="confirmDialogRef" />
    <KeepAlive :max="1">
      <PluginHomeView
        v-if="pluginsStore.selectedPluginId"
        :key="pluginsStore.selectedPluginId"
        :plugin-id="pluginsStore.selectedPluginId"
        @back="pluginsStore.setSelectedPluginId(null)"
      />
    </KeepAlive>
    <div v-if="!pluginsStore.selectedPluginId" class="user-plugins-list">
      <ExtensionToolbar>
        <h3 class="extension-section-label">
          {{ tr(settingStore.language, "shell.plugins.installed") }}
        </h3>
        <template #actions>
          <div class="user-plugins-toolbar">
            <div class="toolbar-icons">
              <Button
                variant="ghost"
                size="icon"
                class="size-8 shrink-0 text-muted-foreground"
                :title="copy.refresh"
                :aria-label="copy.refresh"
                @click="refresh"
              >
                <RefreshCw class="size-3.5" />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                class="size-8 shrink-0 text-muted-foreground"
                :title="copy.openDir"
                :aria-label="copy.openDir"
                @click="openDir"
              >
                <FolderOpen class="size-3.5" />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                class="size-8 shrink-0 text-muted-foreground"
                :title="copy.closeAll"
                :aria-label="copy.closeAll"
                @click="closeAllWindows"
              >
                <AppWindow class="size-3.5" />
              </Button>
            </div>
            <Button variant="ghost" size="sm" class="h-8 gap-1.5" @click="importFolder">
              <FolderPlus class="size-3.5" />
              {{ actionCopy.importFolder }}
            </Button>
            <Button size="sm" class="h-8 gap-1.5" @click="importZip">
              <Upload class="size-3.5" />
              {{ actionCopy.importZip }}
            </Button>
          </div>
        </template>
      </ExtensionToolbar>
      <p v-if="pluginsStore.safeMode" class="user-plugins-safe">
        {{ copy.safeMode }}
        <button type="button" class="settings-tab" @click="exitSafeMode">
          {{ copy.exitSafe }}
        </button>
      </p>
      <p class="user-plugins-warn">{{ copy.warning }}</p>
      <PluginDiagnostics />
      <p v-if="!plugins.length" class="settings-empty">{{ copy.empty }}</p>
      <div v-else class="user-plugins-list-items">
        <CatalogItemCard
          v-for="plugin in plugins"
          :key="plugin.id"
          :title="plugin.name"
          :meta="typeLabel(plugin)"
          :description="plugin.description || actionCopy.noDesc"
          :pills="plugin.apiSupported ? [] : [actionCopy.apiUnsupported]"
          :expand-label="copy.expand"
          :collapse-label="copy.collapse"
        >
          <template #icon>
            <PluginSidebarIcon :name="iconSrc(plugin)" :size="40" />
          </template>
          <template #action>
            <SettingsToggle
              :model-value="plugin.enabled"
              :aria-label="plugin.enabled ? actionCopy.disable : actionCopy.enable"
              :title="plugin.enabled ? actionCopy.disable : actionCopy.enable"
              @click.prevent="onToggle(plugin)"
            />
            <CatalogRoundAction
              :icon="ChevronRight"
              :label="copy.details + ' · ' + plugin.name"
              @click="pluginsStore.setSelectedPluginId(plugin.id)"
            />
          </template>
        </CatalogItemCard>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { AppWindow, ChevronRight, FolderOpen, FolderPlus, RefreshCw, Upload } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import CatalogItemCard from "@/components/settings/CatalogItemCard.vue";
import CatalogRoundAction from "@/components/settings/CatalogRoundAction.vue";
import ExtensionToolbar from "@/components/settings/ExtensionToolbar.vue";
import SettingsToggle from "@/components/settings/SettingsToggle.vue";
import PluginDiagnostics from "@/components/plugins/PluginDiagnostics.vue";
import { tr } from "@/services/i18n";
import {
  clearPluginSafeMode,
  closeAllUserPluginWindows,
  openPluginsDir,
} from "@/services/plugins/ipc";
import { syncEnabledPluginUi } from "@/composables/plugins/sdk";
import { usePluginsStore } from "@/stores/plugins";
import { useSettingStore } from "@/stores/setting";
import PluginSidebarIcon from "@/components/plugins/PluginSidebarIcon.vue";
import PluginHomeView from "@/components/plugins/PluginHomeView.vue";
import { usePluginActions } from "@/composables/plugins/usePluginActions";
import type { UserPluginSummary } from "@/services/plugins/ipc";

const pluginsStore = usePluginsStore();
const settingStore = useSettingStore();
const plugins = computed(() => pluginsStore.plugins);
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const {
  copy: actionCopy,
  ensurePending,
  iconSrc,
  typeLabel,
  toggle,
  importZip,
  importFolder,
} = usePluginActions();

async function onToggle(plugin: UserPluginSummary) {
  await toggle(plugin, (opts) => confirmDialogRef.value?.ask(opts) ?? Promise.resolve(false));
}

const copy = computed(() =>
  settingStore.language === "zh-CN"
    ? {
        warning:
          "启用会把已打包的 activate.js 加载进工作台同一页，并可读取会话和设置（含 API Key）。令牌只约束 Deno/终端。不要启用不信任的本地插件。Enable 时会把 UI 打成一份 ESM。",
        refresh: "刷新",
        details: "查看详情",
        expand: "展开",
        collapse: "收起",
        openDir: "打开插件目录",
        closeAll: "关闭全部插件窗口",
        empty:
          "还没有用户插件。可用 manage_plugin 创建，或点「导入」安装 zip/文件夹。不要改 Anya 源码。",
        safeMode: "插件安全模式：已跳过全部插件。",
        exitSafe: "退出安全模式",
      }
    : {
        warning:
          "Enabling loads the bundled activate.js into the same page as Anya. It can read chats and settings (including API keys). OS tokens only constrain Deno/PTY. Do not enable untrusted local plugins. Enable bundles UI to one ESM file.",
        refresh: "Refresh",
        details: "View details",
        expand: "More",
        collapse: "Less",
        openDir: "Open folder",
        closeAll: "Close plugin windows",
        empty:
          "No user plugins yet. Create one with manage_plugin, or Import a zip/folder. Never edit Anya source.",
        safeMode: "Plugin safe mode: all plugins skipped.",
        exitSafe: "Exit safe mode",
      },
);

onMounted(async () => {
  await pluginsStore.refresh();
  for (const plugin of pluginsStore.plugins) ensurePending(plugin);
});

async function refresh() {
  await pluginsStore.refresh();
}

async function openDir() {
  await openPluginsDir();
}

async function closeAllWindows() {
  await closeAllUserPluginWindows();
}

async function exitSafeMode() {
  await clearPluginSafeMode();
  await pluginsStore.refresh();
  await syncEnabledPluginUi();
}
</script>

<style scoped>
.user-plugins,
.user-plugins-list {
  min-width: 0;
}
.extension-section-label {
  margin: 0;
  font-size: 14px;
  font-weight: 650;
}
.user-plugins-warn,
.user-plugins-safe {
  margin: 0 0 12px;
  padding: 10px 12px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--peek-text) 4%, transparent);
  color: var(--peek-muted);
  font-size: 12px;
  line-height: 1.5;
}
.user-plugins-safe,
.user-plugins-toolbar,
.toolbar-icons {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}
.user-plugins-list-items {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
</style>
