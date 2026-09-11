<template>
  <section v-if="hasDiagnostics" class="plugin-diagnostics">
    <div v-if="conflicts.length" class="diagnostics-block">
      <h4>{{ copy.conflictsTitle }}</h4>
      <ul>
        <li v-for="item in conflicts" :key="item.id">
          <code>{{ item.target }}</code>
          — {{ copy.ownedBy }}
          <b>{{ item.ownerPluginId }}</b>
          , {{ copy.rejected }}
          <b>{{ item.rejectedPluginId }}</b>
          <button
            type="button"
            class="diagnostics-takeover"
            @click="takeOver(item.ownerPluginId, item.rejectedPluginId)"
          >
            {{ copy.takeOver }}
          </button>
        </li>
      </ul>
    </div>
    <div v-if="unsupported.length" class="diagnostics-block">
      <h4>{{ copy.apiTitle }}</h4>
      <ul>
        <li v-for="item in unsupported" :key="item.id">
          <b>{{ item.name }}</b>
          — apiVersion
          <code>{{ item.apiVersion }}</code>
        </li>
      </ul>
    </div>
    <div v-if="recentErrors.length" class="diagnostics-block">
      <h4>{{ copy.errorsTitle }}</h4>
      <button type="button" class="diagnostics-clear" @click="pluginsStore.clearErrors()">
        {{ copy.clear }}
      </button>
      <ul>
        <li v-for="(err, idx) in recentErrors" :key="idx">
          <code>{{ err.pluginId }}</code>
          [{{ err.phase }}] {{ err.message }}
        </li>
      </ul>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { usePluginsStore } from "@/stores/plugins";
import { useSettingStore } from "@/stores/setting";
import { usePluginActions } from "@/composables/plugins/usePluginActions";

const pluginsStore = usePluginsStore();
const settingStore = useSettingStore();
const { takeOver } = usePluginActions();

const slotConflicts = computed(() => pluginsStore.slotConflicts);
const assetConflicts = computed(() => pluginsStore.assetConflicts);
const conflicts = computed(() => [
  ...slotConflicts.value.map((item, idx) => ({
    id: `slot:${item.anchorId}:${item.rejectedPluginId}:${idx}`,
    target: item.anchorId,
    ownerPluginId: item.ownerPluginId,
    rejectedPluginId: item.rejectedPluginId,
  })),
  ...assetConflicts.value.map((item, idx) => ({
    id: `asset:${item.key}:${item.rejectedPluginId}:${idx}`,
    target: item.key,
    ownerPluginId: item.ownerPluginId,
    rejectedPluginId: item.rejectedPluginId,
  })),
]);
const recentErrors = computed(() => pluginsStore.errors.slice(-20).reverse());
const unsupported = computed(() => pluginsStore.plugins.filter((p) => !p.apiSupported));
const hasDiagnostics = computed(
  () => conflicts.value.length > 0 || recentErrors.value.length > 0 || unsupported.value.length > 0,
);

const copy = computed(() =>
  settingStore.language === "zh-CN"
    ? {
        conflictsTitle: "锚点 / 资源冲突",
        ownedBy: "当前占用者",
        rejected: "被拒绝",
        takeOver: "停用占用者并重载被拒绝插件",
        apiTitle: "契约版本不受支持",
        errorsTitle: "最近插件错误",
        clear: "清空",
      }
    : {
        conflictsTitle: "Anchor / asset conflicts",
        ownedBy: "owned by",
        rejected: "rejected",
        takeOver: "Disable owner and reload rejected plugin",
        apiTitle: "Unsupported contract version",
        errorsTitle: "Recent plugin errors",
        clear: "Clear",
      },
);
</script>

<style scoped>
.plugin-diagnostics {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 12px;
  padding: 10px 12px;
  border-radius: 10px;
  background: var(--peek-surface-muted, rgba(255, 255, 255, 0.04));
  border: 1px solid var(--peek-border, rgba(255, 255, 255, 0.08));
  font-size: 12.5px;
}

.diagnostics-block h4 {
  margin: 0 0 4px;
  font-size: 12px;
  opacity: 0.75;
}

.diagnostics-block ul {
  margin: 0;
  padding-left: 16px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.diagnostics-takeover,
.diagnostics-clear {
  border: none;
  background: transparent;
  color: inherit;
  opacity: 0.8;
  cursor: pointer;
  font-size: 12px;
  text-decoration: underline;
}

.diagnostics-takeover {
  display: inline;
  margin-left: 6px;
}

.diagnostics-clear {
  float: right;
}
</style>
