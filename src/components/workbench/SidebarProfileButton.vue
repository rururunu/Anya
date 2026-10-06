<script setup lang="ts">
import { computed } from "vue";
import { Settings } from "@lucide/vue";
import UserAvatar from "@/components/chat/UserAvatar.vue";
import { useLocalProfile } from "@/services/settings/localProfile";
import { useSettingStore } from "@/stores/setting";
const emit = defineEmits<{ open: [] }>();
const props = defineProps<{ settingsLabel: string }>();
const profile = useLocalProfile();
const settingStore = useSettingStore();
const nickname = computed(
  () =>
    profile.value.displayName.trim() ||
    (settingStore.language === "zh-CN" ? "本地用户" : "Local user"),
);
</script>

<template>
  <button
    class="sidebar-profile-button"
    type="button"
    :aria-label="`${nickname} · ${props.settingsLabel}`"
    @click="emit('open')"
  >
    <UserAvatar class="sidebar-profile-avatar" />
    <span class="sidebar-profile-name">{{ nickname }}</span>
    <Settings :size="17" :stroke-width="1.75" aria-hidden="true" />
  </button>
</template>

<style scoped>
.sidebar-profile-button {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  flex: none;
  margin-top: 12px;
  padding: 9px 10px;
  border-radius: 10px;
  color: var(--peek-text);
  text-align: left;
  cursor: pointer;
}
.sidebar-profile-button:hover {
  background: var(--peek-row-hover);
}
.sidebar-profile-button:focus-visible {
  outline: 2px solid var(--peek-accent);
  outline-offset: 2px;
}
.sidebar-profile-avatar {
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
.sidebar-profile-button > svg {
  flex: none;
  color: var(--peek-muted);
}
</style>
