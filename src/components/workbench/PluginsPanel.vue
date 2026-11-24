<template>
  <TabsRoot default-value="plugins" class="plugins-panel" activation-mode="manual">
    <TabsList
      class="plugins-tabs"
      :aria-label="tr(settingStore.language, 'shell.plugins.sections')"
    >
      <TabsTrigger v-for="tab in tabs" :key="tab.id" :value="tab.id" class="plugins-tab">
        <component :is="tab.icon" :size="16" :stroke-width="1.75" aria-hidden="true" />
        {{ tab.label }}
      </TabsTrigger>
    </TabsList>
    <TabsContent value="plugins" class="plugins-content peek-scrollbar">
      <UserPluginsPanel />
    </TabsContent>
    <TabsContent value="skills" class="plugins-content peek-scrollbar">
      <SkillsSettings embedded />
    </TabsContent>
    <TabsContent value="mcp" class="plugins-content peek-scrollbar">
      <McpSettings embedded />
    </TabsContent>
  </TabsRoot>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Cable, Puzzle, ScrollText } from "@lucide/vue";
import { TabsContent, TabsList, TabsRoot, TabsTrigger } from "reka-ui";
import SkillsSettings from "@/components/settings/SkillsSettings.vue";
import McpSettings from "@/components/settings/McpSettings.vue";
import UserPluginsPanel from "@/components/plugins/UserPluginsPanel.vue";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";

const props = defineProps<{ pluginsLabel: string }>();
const settingStore = useSettingStore();
const tabs = computed(() => [
  { id: "plugins", icon: Puzzle, label: props.pluginsLabel },
  { id: "skills", icon: ScrollText, label: tr(settingStore.language, "shell.plugins.skills") },
  { id: "mcp", icon: Cable, label: "MCP" },
]);
</script>

<style scoped>
.plugins-panel {
  --plugins-gutter: clamp(16px, 3.4vw, 32px);
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  background: var(--peek-list-bg);
}

.plugins-tabs {
  display: grid;
  flex: none;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  width: min(100%, 440px);
  max-width: calc(100% - 2 * var(--plugins-gutter));
  gap: 4px;
  margin: 14px auto;
  padding: 4px;
  border-radius: 12px;
  background: color-mix(in srgb, var(--peek-text) 5%, var(--peek-list-bg));
}

.plugins-tab {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  min-height: 34px;
  padding: 4px 6px;
  border-radius: 9px;
  color: var(--peek-muted);
  font-size: 13px;
  font-weight: 550;
  line-height: 1.25;
  cursor: pointer;
}

.plugins-tab svg {
  flex: none;
}

.plugins-tab:hover {
  color: var(--peek-text);
}

.plugins-tab[data-state="active"] {
  background: var(--peek-list-bg);
  box-shadow: 0 1px 3px rgb(0 0 0 / 12%);
  color: var(--peek-text);
  font-weight: 650;
}

.plugins-content {
  flex: 1;
  min-height: 0;
  padding: 16px var(--plugins-gutter) 28px;
  overflow: auto;
  scrollbar-gutter: stable both-edges;
  overscroll-behavior: contain;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 65%, transparent);
}

.plugins-content > :deep(.settings-page) {
  width: 100%;
  min-height: 0;
  height: auto;
  margin: 0;
  padding: 0;
  overflow: visible;
}

.plugins-content :deep(.settings-empty) {
  margin: 0;
  min-height: 120px;
}
</style>
