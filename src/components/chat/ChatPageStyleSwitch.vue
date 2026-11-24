<script setup lang="ts">
/** 在工作、聊天、流三种对话页之间切换。 */
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import type { ChatPageStyle } from "@/types/setting";

const settingStore = useSettingStore();

const options: Array<{
  id: ChatPageStyle;
  label: "chatPageWork" | "chatPageChat" | "chatPageStream";
}> = [
  { id: "work", label: "chatPageWork" },
  { id: "chat", label: "chatPageChat" },
  { id: "stream", label: "chatPageStream" },
];

function select(style: ChatPageStyle) {
  if (settingStore.chatPageStyle === style) return;
  void settingStore.update({ chatPageStyle: style });
}
</script>

<template>
  <div
    class="page-style-switch"
    role="tablist"
    :aria-label="tr(settingStore.language, 'chatPageStyleLabel')"
  >
    <button
      v-for="option in options"
      :key="option.id"
      type="button"
      role="tab"
      class="page-style-option"
      :class="{ active: settingStore.chatPageStyle === option.id }"
      :aria-selected="settingStore.chatPageStyle === option.id"
      @click="select(option.id)"
    >
      {{ tr(settingStore.language, option.label) }}
    </button>
  </div>
</template>

<style scoped>
.page-style-switch {
  display: flex;
  flex: none;
  align-items: center;
  gap: 1px;
  padding: 2px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--peek-text) 6%, transparent);
}

.page-style-option {
  appearance: none;
  height: 22px;
  margin: 0;
  padding: 0 8px;
  border: 0;
  border-radius: 999px;
  background: transparent;
  color: color-mix(in srgb, var(--peek-text) 62%, transparent);
  font-size: 11.5px;
  font-weight: 520;
  line-height: 22px;
  cursor: pointer;
}

.page-style-option.active {
  background: var(--peek-surface, #fff);
  color: var(--peek-text);
  box-shadow: 0 1px 2px rgb(0 0 0 / 8%);
}

.page-style-option:hover:not(.active) {
  color: var(--peek-text);
}
</style>
