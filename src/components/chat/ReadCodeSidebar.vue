<template>
  <section class="read-code-sidebar">
    <header>
      <FileCode2 :size="15" />
      <span :title="preview?.path">{{ preview?.path }}</span>
      <span class="read-range">{{ preview?.startLine }}–{{ preview?.endLine }}</span>
      <button
        type="button"
        :aria-label="
          wrapLines
            ? language === 'zh-CN'
              ? '关闭自动换行'
              : 'Disable line wrap'
            : language === 'zh-CN'
              ? '开启自动换行'
              : 'Enable line wrap'
        "
        :title="
          wrapLines
            ? language === 'zh-CN'
              ? '关闭自动换行'
              : 'Disable line wrap'
            : language === 'zh-CN'
              ? '开启自动换行'
              : 'Enable line wrap'
        "
        :aria-pressed="wrapLines"
        @click="wrapLines = !wrapLines"
      >
        <TextWrap :size="15" />
      </button>
    </header>
    <p class="read-code-caption">
      {{ tr(language, current ? "readCodeCurrent" : "readCodeSnapshot") }}
      <span v-if="notice">· {{ notice }}</span>
      <button v-if="availableCurrent && !current" type="button" @click="showCurrent">
        {{ language === "zh-CN" ? "查看当前文件上下文" : "View current file context" }}
      </button>
    </p>
    <ReadCodeEditor
      v-if="preview"
      class="sidebar-code"
      :content="content"
      :path="preview.path"
      :first-line="current ? 1 : preview.startLine"
      :start-line="preview.startLine"
      :end-line="preview.endLine"
      :wrap-lines="wrapLines"
    />
  </section>
</template>
<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { FileCode2, TextWrap } from "@lucide/vue";
import ReadCodeEditor from "./ReadCodeEditor.vue";
import { useCodeReadPreviewStore } from "@/stores/codeReadPreview";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
const store = useCodeReadPreviewStore();
const preview = computed(() => store.selection);
const language = computed(() => useSettingStore().language);
const content = ref("");
const current = ref(false);
const availableCurrent = ref("");
const notice = ref("");
const wrapLines = ref(localStorage.getItem("anya.readCodeWrap") === "true");
watch(wrapLines, (value) => localStorage.setItem("anya.readCodeWrap", String(value)));
let revision = 0;
function showCurrent() {
  content.value = availableCurrent.value;
  current.value = true;
  notice.value = tr(language.value, "readCodeChanged");
}
watch(
  preview,
  async (selection) => {
    const request = ++revision;
    content.value = selection?.content ?? "";
    current.value = false;
    availableCurrent.value = "";
    notice.value = "";
    if (!selection) return;
    try {
      const text = await invoke<string>("preview_read_code", { activityId: selection.activityId });
      if (request !== revision) return;
      availableCurrent.value = text;
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
header button {
  flex: none;
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}
header button:hover {
  background: color-mix(in srgb, var(--peek-text) 8%, transparent);
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
.read-code-caption button {
  margin-left: 8px;
  border: 0;
  background: transparent;
  color: var(--peek-accent);
  cursor: pointer;
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
