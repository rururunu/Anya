<template>
  <section class="file-read-card">
    <header>
      <button
        class="read-expand"
        type="button"
        :aria-expanded="expanded"
        :aria-label="tr(language, 'executionDetails')"
        @click="expanded = !expanded"
      >
        <ChevronRight :size="12" :class="{ expanded }" />
      </button>
      <FileCode2 :size="14" aria-hidden="true" />
      <button
        class="read-file-link"
        type="button"
        :title="preview.path"
        @click="store.open(preview)"
      >
        {{ filename }}
      </button>
      <span class="read-lines">{{ preview.startLine }}–{{ preview.endLine }}</span>
      <button
        class="read-open"
        type="button"
        :title="tr(language, 'readCodeOpen')"
        :aria-label="tr(language, 'readCodeOpen')"
        @click="store.open(preview)"
      >
        <PanelRightOpen :size="13" />
      </button>
    </header>
    <div
      v-if="expanded"
      class="read-card-body"
      :style="{ height: `${Math.min(240, (preview.endLine - preview.startLine + 1) * 21 + 18)}px` }"
    >
      <ReadCodeEditor
        :content="preview.content"
        :path="preview.path"
        :first-line="preview.startLine"
      />
    </div>
  </section>
</template>
<script setup lang="ts">
import { computed, ref } from "vue";
import { ChevronRight, FileCode2, PanelRightOpen } from "@lucide/vue";
import ReadCodeEditor from "./ReadCodeEditor.vue";
import type { ReadCodePreview } from "@/services/chat/readCodePreview";
import { useCodeReadPreviewStore } from "@/stores/codeReadPreview";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
const props = defineProps<{ preview: ReadCodePreview }>();
const store = useCodeReadPreviewStore();
const language = computed(() => useSettingStore().language);
const filename = computed(() => props.preview.path.split(/[\\/]/).pop());
const expanded = ref(false);
</script>
<style scoped>
.file-read-card {
  border: 1px solid color-mix(in srgb, var(--peek-text) 12%, transparent);
  border-radius: 9px;
  overflow: hidden;
  background: var(--peek-surface);
  margin: 4px 0;
}
header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  color: var(--peek-muted);
}
button {
  display: inline-flex;
  align-items: center;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  cursor: pointer;
}
.read-file-link {
  min-width: 0;
  color: var(--peek-text);
  font: inherit;
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.read-file-link:hover {
  text-decoration: underline;
}
.read-expand {
  padding: 3px;
}
.read-expand .expanded {
  transform: rotate(90deg);
}
.read-lines {
  font-size: 11px;
  white-space: nowrap;
}
.read-open {
  margin-left: auto;
  padding: 3px;
}
.read-card-body {
  border-top: 1px solid color-mix(in srgb, var(--peek-text) 8%, transparent);
  background: color-mix(in srgb, var(--peek-text) 2%, var(--peek-surface));
}
</style>
