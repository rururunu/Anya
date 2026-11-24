<script setup lang="ts">
import { computed } from "vue";
import { Settings } from "@lucide/vue";
import UserAvatar from "@/components/chat/UserAvatar.vue";
import { useLocalProfile } from "@/services/settings/localProfile";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
const emit = defineEmits<{ open: [] }>();
const props = defineProps<{ settingsLabel: string }>();
const profile = useLocalProfile();
const settingStore = useSettingStore();
const nickname = computed(
  () => profile.value.displayName.trim() || tr(settingStore.language, "shell.profile.localUser"),
);
</script>

<template>
  <div class="sidebar-profile-row">
    <button
      class="sidebar-profile-button"
      type="button"
      :aria-label="`${nickname} · ${props.settingsLabel}`"
      @click="emit('open')"
    >
      <UserAvatar class="sidebar-profile-avatar" />
      <span class="sidebar-profile-name">{{ nickname }}</span>
    </button>
    <slot name="actions" />
    <button
      class="sidebar-settings-button"
      type="button"
      :title="props.settingsLabel"
      :aria-label="props.settingsLabel"
      @click="emit('open')"
    >
      <Settings :size="17" :stroke-width="1.75" aria-hidden="true" />
    </button>
  </div>
</template>

<style scoped>
.sidebar-profile-button {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: 1;
  min-width: 0;
  padding: 6px;
  border-radius: 10px;
  color: var(--peek-text);
  text-align: left;
  cursor: pointer;
}
.sidebar-profile-row {
  display: flex;
  flex: none;
  align-items: center;
  gap: 4px;
  margin-top: 12px;
  padding-inline: 4px;
}
.sidebar-settings-button {
  display: inline-grid;
  flex: none;
  place-items: center;
  width: var(--peek-control-icon);
  height: var(--peek-control-icon);
  border-radius: var(--peek-radius-sm);
  color: var(--peek-muted);
  cursor: pointer;
}
.sidebar-profile-button:hover,
.sidebar-settings-button:hover {
  background: var(--peek-row-hover);
}
.sidebar-profile-button:focus-visible,
.sidebar-settings-button:focus-visible {
  outline: 2px solid var(--peek-accent);
  outline-offset: 2px;
}
.sidebar-profile-avatar {
  flex: none;
  width: 30px;
  height: 30px;
}
.sidebar-profile-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  font-weight: 500;
}
</style>
