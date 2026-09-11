<template>
  <section class="user-plugins">
    <KeepAlive :max="1">
      <PluginHomeView
        v-if="pluginsStore.selectedPluginId"
        :key="pluginsStore.selectedPluginId"
        :plugin-id="pluginsStore.selectedPluginId"
        @back="pluginsStore.setSelectedPluginId(null)"
      />
    </KeepAlive>
    <div v-if="!pluginsStore.selectedPluginId" class="user-plugins-list">
      <p v-if="pluginsStore.safeMode" class="user-plugins-safe">
        {{ copy.safeMode }}
        <button type="button" class="settings-tab" @click="exitSafeMode">
          {{ copy.exitSafe }}
        </button>
      </p>
      <p class="user-plugins-warn">{{ copy.warning }}</p>
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
      <p v-if="!plugins.length" class="user-plugins-empty">{{ copy.empty }}</p>
      <div class="user-plugins-grid">
        <article
          v-for="plugin in plugins"
          :key="plugin.id"
          class="user-plugin-card"
          @click="pluginsStore.setSelectedPluginId(plugin.id)"
        >
          <div class="card-icon">
            <PluginSidebarIcon :name="iconSrc(plugin)" :size="18" />
          </div>
          <div class="card-main">
            <div class="card-name">
              <strong>{{ plugin.name }}</strong>
              <span v-if="!plugin.apiSupported" class="row-warn" :title="actionCopy.apiUnsupported">
                !
              </span>
            </div>
            <div class="card-meta">
              <span class="row-type">{{ typeLabel(plugin) }}</span>
              <span class="row-sep">|</span>
              <span class="row-desc">{{ plugin.description || actionCopy.noDesc }}</span>
            </div>
          </div>
          <label
            class="row-toggle"
            :title="plugin.enabled ? actionCopy.disable : actionCopy.enable"
            @click.stop
          >
            <input type="checkbox" :checked="plugin.enabled" @change="toggle(plugin)" />
            <span class="toggle-track"><span class="toggle-thumb" /></span>
          </label>
        </article>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted } from "vue";
import { AppWindow, FolderOpen, FolderPlus, RefreshCw, Upload } from "@lucide/vue";
import { Button } from "@/components/ui/button";
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

const pluginsStore = usePluginsStore();
const settingStore = useSettingStore();
const plugins = computed(() => pluginsStore.plugins);
const {
  copy: actionCopy,
  ensurePending,
  iconSrc,
  typeLabel,
  toggle,
  importZip,
  importFolder,
} = usePluginActions();

const copy = computed(() =>
  settingStore.language === "zh-CN"
    ? {
        warning:
          "启用会把已打包的 activate.js 加载进工作台同一页，并可读取会话和设置（含 API Key）。令牌只约束 Deno/终端。不要启用不信任的本地插件。Enable 时会把 UI 打成一份 ESM。",
        refresh: "刷新",
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
.user-plugins {
  display: flex;
  flex-direction: column;
  gap: 12px;
  flex: 1;
  min-height: 0;
}
.user-plugins-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
  flex: 1;
  min-height: 0;
}
.user-plugins-warn {
  margin: 0;
  font-size: 12px;
  line-height: 1.45;
  color: var(--peek-muted, #8b939e);
}
.user-plugins-safe {
  margin: 0;
  font-size: 12px;
  line-height: 1.45;
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
  color: #fbbf24;
}
.user-plugins-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}
.toolbar-icons {
  display: flex;
  align-items: center;
  gap: 2px;
}
.user-plugins-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 10px;
}
.user-plugin-card {
  border: 1px solid var(--peek-border, rgba(255, 255, 255, 0.08));
  border-radius: 12px;
  padding: 10px;
  display: flex;
  align-items: flex-start;
  gap: 8px;
  min-width: 0;
  cursor: pointer;
}
.user-plugin-card:hover {
  background: var(--peek-row-hover);
}
.card-icon {
  flex: none;
  width: 34px;
  height: 34px;
  border-radius: 10px;
  border: 1.5px solid var(--peek-border, rgba(255, 255, 255, 0.18));
  display: grid;
  place-items: center;
  opacity: 0.85;
}
.card-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding-top: 1px;
}
.card-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
}
.card-name strong {
  font-weight: 600;
}
.row-warn {
  margin-left: 6px;
  color: #fbbf24;
  font-weight: 700;
}
.card-meta {
  font-size: 11.5px;
  color: var(--peek-muted, #8b939e);
  display: flex;
  gap: 5px;
  min-width: 0;
}
.row-type {
  flex: none;
  font-weight: 600;
  opacity: 0.85;
}
.row-sep {
  flex: none;
  opacity: 0.5;
}
.row-desc {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}
.row-toggle {
  position: relative;
  display: inline-flex;
  flex: none;
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
</style>
