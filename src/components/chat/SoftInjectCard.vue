<template>
  <div class="inject-card" :class="{ 'is-collapsed': collapsed }">
    <span class="inject-label">{{ tr(settingStore.language, "softInjected") }}</span>
    <div class="inject-body">{{ text }}</div>
    <button
      v-if="needsFold"
      type="button"
      class="inject-toggle"
      data-tauri-drag-region="false"
      @click.stop="expanded = !expanded"
    >
      {{ tr(settingStore.language, expanded ? "collapse" : "expand") }}
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { parseSelectionAttachment } from "@/services/chat/selectionAttachment";
import { stripSoftInjectMarker } from "@/services/chat/softInject";
import { shouldFoldSoftInject, SOFT_INJECT_FOLD_VISIBLE_LINES } from "@/services/chat/longText";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage } from "@/types/chat";

const props = defineProps<{
  message?: ChatMessage;
  content?: string;
}>();

const settingStore = useSettingStore();
const expanded = ref(false);

const text = computed(() => {
  const raw = props.content ?? props.message?.content ?? "";
  const parsed = parseSelectionAttachment(stripSoftInjectMarker(raw));
  return [parsed.message, parsed.selection].filter(Boolean).join("\n");
});
const needsFold = computed(() => shouldFoldSoftInject(text.value));
const collapsed = computed(() => needsFold.value && !expanded.value);
</script>

<style scoped>
.inject-card {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 4px;
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
  padding: 8px 12px;
  border: 1px solid
    var(--peek-composer-border, color-mix(in srgb, var(--peek-text) 16%, transparent));
  border-radius: var(--peek-radius-composer, 16px);
  background: var(--peek-composer-fill, var(--peek-user-bubble-bg));
  color: var(--peek-user-bubble-text, var(--peek-text));
}
.inject-label {
  color: var(--peek-muted);
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.02em;
  line-height: 1.2;
}
.inject-body {
  min-width: 0;
  font-size: 13px;
  font-weight: 500;
  line-height: 1.5;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  word-break: break-word;
}
.inject-card.is-collapsed .inject-body {
  max-height: calc(1.5em * v-bind(SOFT_INJECT_FOLD_VISIBLE_LINES));
  overflow: hidden;
  -webkit-mask-image: linear-gradient(to bottom, #000 58%, transparent 100%);
  mask-image: linear-gradient(to bottom, #000 58%, transparent 100%);
}
.inject-toggle {
  align-self: flex-start;
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--peek-muted);
  font-size: 11px;
  font-weight: 550;
  cursor: pointer;
}
.inject-toggle:hover {
  color: var(--peek-text);
}
</style>
