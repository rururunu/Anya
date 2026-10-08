<template>
  <section class="read-code-sidebar">
    <header>
      <FileCode2 :size="15" />
      <span :title="preview?.path">{{ preview?.path }}</span>
      <span class="read-range">{{ preview?.startLine }}–{{ preview?.endLine }}</span>
    </header>
    <p class="read-code-caption">
      {{ tr(language, current ? "readCodeCurrent" : "readCodeSnapshot") }}
      <span v-if="notice">· {{ notice }}</span>
    </p>
    <ReadCodeEditor
      v-if="preview"
      class="sidebar-code"
      :content="content"
      :path="preview.path"
      :first-line="current ? 1 : preview.startLine"
      :start-line="preview.startLine"
      :end-line="preview.endLine"
    />
  </section>
</template>
<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { FileCode2 } from "@lucide/vue";
import ReadCodeEditor from "./ReadCodeEditor.vue";
import { useCodeReadPreviewStore } from "@/stores/codeReadPreview";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
const store = useCodeReadPreviewStore();
const preview = computed(() => store.selection);
const language = computed(() => useSettingStore().language);
const content = ref("");
const current = ref(false);
const notice = ref("");
let revision = 0;
watch(
  preview,
  async (selection) => {
    const request = ++revision;
    content.value = selection?.content ?? "";
    current.value = false;
    notice.value = "";
    if (!selection) return;
    try {
      const text = await invoke<string>("preview_read_code", { activityId: selection.activityId });
      if (request !== revision) return;
      const lines = text.split(/\r?\n/).slice(selection.startLine - 1, selection.endLine);
      const captured = selection.content.split("\n");
      if (
        lines.length !== captured.length ||
        !lines.every(
          (line, index) =>
            line === captured[index] ||
            (captured[index].includes("…[") && line.startsWith(captured[index].split("…[")[0])),
        )
      ) {
        notice.value = tr(language.value, "readCodeChanged");
        return;
      }
      content.value = text;
      current.value = true;
    } catch {
      if (request === revision) notice.value = tr(language.value, "readCodeFallback");
    }
  },
  { immediate: true },
);
</script>
<style scoped>
.read-code-sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  color: var(--peek-text);
  background: var(--peek-surface);
}
header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 14px;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-text) 10%, transparent);
  font-size: 12px;
}
header > span:first-of-type {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.read-range {
  font-size: 11px;
  color: var(--peek-muted);
  white-space: nowrap;
}
.read-code-caption {
  margin: 0;
  padding: 8px 14px;
  font-size: 11px;
  color: var(--peek-muted);
}
.sidebar-code {
  flex: 1;
}
</style>
