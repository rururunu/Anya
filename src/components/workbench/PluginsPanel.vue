<template>
  <section class="plugins-panel settings-page is-wide">
    <nav class="settings-tabs plugins-top-tabs" role="tablist" :aria-label="copy.tabsLabel">
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

    <SkillsSettings v-if="tab === 'skills'" embedded />
    <McpSettings v-else embedded />
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { Cable, ScrollText } from "@lucide/vue";
import SkillsSettings from "@/components/settings/SkillsSettings.vue";
import McpSettings from "@/components/settings/McpSettings.vue";
import { useSettingStore } from "@/stores/setting";

type PluginsTab = "skills" | "mcp";

const tab = ref<PluginsTab>("skills");
const settingStore = useSettingStore();

const copy = computed(() =>
  settingStore.language === "zh-CN"
    ? {
        skills: "\u6280\u80fd",
        mcp: "MCP",
        tabsLabel: "\u63d2\u4ef6\u5206\u7c7b",
      }
    : {
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

.plugins-panel > :deep(.skills-settings),
.plugins-panel > :deep(.mcp-settings) {
  flex: 1;
  min-height: 0;
  width: 100%;
  margin-inline: 0;
  padding-inline: 0;
  padding-top: 0;
}

.plugins-panel > :deep(.skills-settings) .settings-tabs,
.plugins-panel > :deep(.mcp-settings) .settings-tabs {
  margin-top: 0;
}
</style>
