<template>
  <section v-if="changes.length" class="changes-summary">
    <header class="changes-summary-header">
      <span class="changes-summary-icon" aria-hidden="true">
        <FileDiff :size="16" :stroke-width="1.8" />
      </span>
      <div class="changes-summary-title">
        <strong>{{ tr(settingStore.language, "editedFiles", { count: changes.length }) }}</strong>
        <span class="changes-summary-total">
          <span class="added">+{{ totals.added }}</span>
          <span class="removed">-{{ totals.removed }}</span>
        </span>
      </div>
      <div class="changes-summary-actions">
        <button v-if="canUndo" type="button" :disabled="busy" @click="$emit('undo')">
          {{ tr(settingStore.language, "undoChanges") }}
          <Undo2 :size="14" :stroke-width="1.8" aria-hidden="true" />
        </button>
        <button type="button" class="review-button" @click="$emit('review')">
          {{ tr(settingStore.language, "reviewChanges") }}
        </button>
      </div>
    </header>

    <div class="changes-summary-list">
      <button
        v-for="change in visibleChanges"
        :key="change.id"
        type="button"
        class="changes-summary-file"
        :title="change.path"
        @click="$emit('reviewFile', change.path)"
      >
        <span class="file-path">
          <span class="file-name">{{ fileName(change.path) }}</span>
          <span v-if="fileDir(change.path)" class="file-dir">{{ fileDir(change.path) }}</span>
        </span>
        <span class="file-stats">
          <span class="added">+{{ change.added }}</span>
          <span class="removed">-{{ change.removed }}</span>
        </span>
      </button>

      <button
        v-if="hiddenCount > 0"
        type="button"
        class="changes-summary-toggle"
        @click="expanded = !expanded"
      >
        <ChevronDown
          :size="14"
          :stroke-width="1.8"
          class="toggle-chevron"
          :class="{ open: expanded }"
          aria-hidden="true"
        />
        <span>
          {{
            expanded
              ? tr(settingStore.language, "collapse")
              : tr(settingStore.language, "moreEditedFiles", { count: hiddenCount })
          }}
        </span>
      </button>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ChevronDown, FileDiff, Undo2 } from "@lucide/vue";
import { extractCodeChanges } from "@/services/chat/codeChanges";
import { fileBasename, fileParentDir } from "@/services/chat/toolDiff";
import { textIncludesQuery } from "@/services/chat/conversationFind";
import { useExpandForFind } from "@/composables/chat/useConversationFind";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage } from "@/types/chat";

const COLLAPSED_LIMIT = 2;

const props = withDefaults(
  defineProps<{
    message: ChatMessage;
    canUndo?: boolean;
    busy?: boolean;
  }>(),
  {
    canUndo: false,
    busy: false,
  },
);

defineEmits<{
  undo: [];
  review: [];
  reviewFile: [path: string];
}>();

const settingStore = useSettingStore();
const expanded = ref(false);
const changes = computed(() => extractCodeChanges([props.message]));
const totals = computed(() =>
  changes.value.reduce(
    (total, change) => ({
      added: total.added + change.added,
      removed: total.removed + change.removed,
    }),
    { added: 0, removed: 0 },
  ),
);
const hiddenCount = computed(() => Math.max(0, changes.value.length - COLLAPSED_LIMIT));
const visibleChanges = computed(() =>
  expanded.value || hiddenCount.value === 0
    ? changes.value
    : changes.value.slice(0, COLLAPSED_LIMIT),
);

watch(
  () => props.message.id,
  () => {
    expanded.value = false;
  },
);

useExpandForFind(
  (query) =>
    changes.value.some(
      (change) => textIncludesQuery(change.path, query) || textIncludesQuery(change.diff, query),
    ),
  () => {
    expanded.value = true;
  },
);

function fileName(path: string) {
  return fileBasename(path);
}
function fileDir(path: string) {
  return fileParentDir(path);
}
</script>

<style scoped>
.changes-summary {
  width: 100%;
  box-sizing: border-box;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--peek-border) 88%, transparent);
  border-radius: 10px;
  background: color-mix(in srgb, var(--peek-input-bg) 92%, #0b0d10);
}

.changes-summary-header {
  min-height: 44px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-border) 70%, transparent);
  background: color-mix(in srgb, var(--peek-panel) 55%, transparent);
}

.changes-summary-icon {
  flex: none;
  width: 26px;
  height: 26px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid color-mix(in srgb, var(--peek-accent) 28%, var(--peek-border));
  border-radius: 8px;
  color: var(--peek-accent);
  background: color-mix(in srgb, var(--peek-accent) 8%, transparent);
}

.changes-summary-title {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.changes-summary-title strong {
  overflow: hidden;
  color: var(--peek-text);
  font-size: 12px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.changes-summary-total,
.file-stats {
  display: inline-flex;
  gap: 5px;
  font: 10px/1.2 var(--font-mono);
  font-variant-numeric: tabular-nums;
}

.added {
  color: #4ade80;
}
.removed {
  color: #fb7185;
}

.changes-summary-actions {
  flex: none;
  display: flex;
  align-items: center;
  gap: 4px;
}

.changes-summary-actions button {
  height: 28px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  padding: 0 9px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--peek-muted);
  font-size: 11px;
  white-space: nowrap;
  cursor: pointer;
}

.changes-summary-actions button:hover:not(:disabled) {
  color: var(--peek-text);
  background: var(--peek-hover-bg);
}

.changes-summary-actions button:disabled {
  opacity: 0.45;
  cursor: default;
}

.changes-summary-actions .review-button {
  border-color: color-mix(in srgb, var(--peek-border) 88%, transparent);
  color: var(--peek-text);
}

.changes-summary-list {
  display: flex;
  flex-direction: column;
}

.changes-summary-file {
  width: 100%;
  min-width: 0;
  min-height: 32px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 12px 6px 16px;
  border: 0;
  background: transparent;
  color: var(--peek-muted);
  text-align: left;
  cursor: pointer;
}

.changes-summary-file:hover {
  background: color-mix(in srgb, var(--peek-text) 4%, transparent);
  color: var(--peek-text);
}

.file-path {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: baseline;
  gap: 8px;
  overflow: hidden;
  font-size: 11px;
}
.file-name {
  flex: none;
  max-width: 58%;
  overflow: hidden;
  color: var(--peek-text);
  text-overflow: ellipsis;
  white-space: nowrap;
}
.file-dir {
  min-width: 0;
  overflow: hidden;
  color: var(--peek-muted);
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 10px;
}

.file-stats {
  flex: none;
}

.changes-summary-toggle {
  width: 100%;
  min-height: 30px;
  display: inline-flex;
  align-items: center;
  justify-content: flex-start;
  gap: 6px;
  padding: 4px 12px 8px 16px;
  border: 0;
  border-top: 1px solid color-mix(in srgb, var(--peek-border) 55%, transparent);
  background: transparent;
  color: var(--peek-muted);
  font-size: 11px;
  cursor: pointer;
}

.changes-summary-toggle:hover {
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 3%, transparent);
}

.toggle-chevron {
  flex: none;
  transition: transform 0.16s ease;
}

.toggle-chevron.open {
  transform: rotate(180deg);
}
</style>
