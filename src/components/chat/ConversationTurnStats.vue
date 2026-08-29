<template>
  <p v-if="visible" class="conversation-turn-stats" :title="fullLine">
    <template v-for="(group, index) in groups" :key="`${index}-${group}`">
      <span v-if="index > 0" class="conversation-turn-stats-sep" aria-hidden="true">|</span>
      <span>{{ group }}</span>
    </template>
  </p>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, toRef } from "vue";
import type { ChatMessage } from "@/types/chat";
import { useConversationTurnStats } from "@/composables/chat/useConversationTurnStats";
import { useSettingStore } from "@/stores/setting";

const props = defineProps<{
  sessionId: string;
  messages: ChatMessage[];
}>();

const settingStore = useSettingStore();
const clock = ref(Date.now());
let timer: number | undefined;

onMounted(() => {
  timer = window.setInterval(() => {
    clock.value = Date.now();
  }, 1000);
});

onUnmounted(() => {
  if (timer) window.clearInterval(timer);
});

const { groups, fullLine, visible } = useConversationTurnStats({
  sessionId: toRef(props, "sessionId"),
  messages: computed(() => props.messages),
  clock,
  language: computed(() => settingStore.language),
});
</script>

<style scoped>
.conversation-turn-stats {
  width: 100%;
  margin: 0;
  padding: 0 4px 6px;
  overflow: hidden;
  color: var(--peek-faint);
  font-size: 10px;
  font-variant-numeric: tabular-nums;
  line-height: 1.4;
  text-overflow: ellipsis;
  white-space: nowrap;
  user-select: none;
}

.conversation-turn-stats-sep {
  margin: 0 6px;
  color: color-mix(in srgb, var(--peek-faint) 70%, transparent);
}
</style>
