<template>
  <section class="plugins-panel settings-page is-wide">
    <nav class="settings-tabs plugins-top-tabs" role="tablist" :aria-label="copy.tabsLabel">
      <button
        type="button"
        role="tab"
        class="settings-tab"
        :class="{ on: tab === 'installed' }"
        :aria-selected="tab === 'installed'"
        @click="tab = 'installed'"
      >
        {{ copy.installed }}
      </button>
      <button
        type="button"
        role="tab"
        class="settings-tab"
        :class="{ on: tab === 'skills' }"
        :aria-selected="tab === 'skills'"
        @click="tab = 'skills'"
      >
        <ScrollText class="plugins-tab-icon" :size="14" :stroke-width="1.75" />
        {{ copy.skills }}
      </button>
      <button
        type="button"
        role="tab"
        class="settings-tab"
        :class="{ on: tab === 'mcp' }"
        :aria-selected="tab === 'mcp'"
        @click="tab = 'mcp'"
      >
        <Cable class="plugins-tab-icon" :size="14" :stroke-width="1.75" />
        {{ copy.mcp }}
      </button>
    </nav>

    <div v-show="tab === 'installed'" class="plugins-installed">
      <PluginDiagnostics />
      <UserPluginsPanel />
    </div>
    <SkillsSettings v-if="tab === 'skills'" embedded />
    <McpSettings v-else-if="tab === 'mcp'" embedded />
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { Cable, ScrollText } from "@lucide/vue";
import SkillsSettings from "@/components/settings/SkillsSettings.vue";
import McpSettings from "@/components/settings/McpSettings.vue";
import UserPluginsPanel from "@/components/plugins/UserPluginsPanel.vue";
import PluginDiagnostics from "@/components/plugins/PluginDiagnostics.vue";
import { useSettingStore } from "@/stores/setting";

type PluginsTab = "installed" | "skills" | "mcp";

const tab = ref<PluginsTab>("installed");
const settingStore = useSettingStore();

const copy = computed(() =>
  settingStore.language === "zh-CN"
    ? {
        installed: "已安装",
        skills: "技能",
        mcp: "MCP",
        tabsLabel: "插件分类",
      }
    : {
        installed: "Installed",
        skills: "Skills",
        mcp: "MCP",
        tabsLabel: "Plugin sections",
      },
);
</script>

<style scoped>
.plugins-panel {
  display: flex;
  flex-direction: column;
  min-height: 0;
  gap: 0;
}

.plugins-top-tabs {
  align-self: flex-start;
  margin-bottom: 12px;
}

.plugins-top-tabs .settings-tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.plugins-tab-icon {
  flex: none;
  opacity: 0.82;
}

.plugins-installed {
  flex: 1;
  min-height: 0;
  width: 100%;
  display: flex;
  flex-direction: column;
}

.plugins-installed :deep(.user-plugins) {
  flex: 1;
  min-height: 0;
}

.plugins-panel > :deep(.plugins-installed),
.plugins-panel > :deep(.skills-settings),
.plugins-panel > :deep(.mcp-settings) {
  flex: 1;
  min-height: 0;
  width: 100%;
}

.plugins-panel > :deep(.skills-settings),
.plugins-panel > :deep(.mcp-settings) {
  margin-inline: 0;
  padding-inline: 0;
  padding-top: 0;
}

.plugins-panel > :deep(.skills-settings) .settings-tabs,
.plugins-panel > :deep(.mcp-settings) .settings-tabs {
  margin-top: 0;
}
</style>
