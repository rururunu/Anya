<template>
  <section class="file-diff-card" :class="[kind, status, { collapsed: !expanded }]">
    <button
      type="button"
      class="file-diff-header"
      :aria-expanded="expanded"
      @click="expanded = !expanded"
    >
      <ChevronRight
        class="file-diff-chevron"
        :class="{ open: expanded }"
        :size="12"
        aria-hidden="true"
      />
      <img v-if="fileIcon" class="file-diff-icon-img" :src="fileIcon" alt="" />
      <span v-else class="file-diff-icon" aria-hidden="true">
        <component :is="fallbackIcon" :size="13" />
      </span>
      <span class="file-diff-name" :title="path">{{ name }}</span>
      <span class="change-stats">
        <span v-if="added" class="added">+{{ added }}</span>
        <span v-if="removed" class="removed">-{{ removed }}</span>
      </span>
      <span v-if="status === 'running'" class="file-diff-status">{{ runningLabel }}</span>
      <span v-else-if="status === 'error'" class="file-diff-status error">{{ failedLabel }}</span>
    </button>
    <div v-if="expanded" class="file-diff-body peek-scrollbar">
      <div
        v-for="(line, index) in highlightedLines"
        :key="index"
        class="diff-line"
        :class="line.kind"
      >
        <span class="line-no old">{{ line.oldNo ?? "" }}</span>
        <span class="line-no new">{{ line.newNo ?? "" }}</span>
        <span class="diff-marker">{{ marker(line.kind) }}</span>
        <span class="diff-text" v-html="line.html" />
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch, type Component } from "vue";
import DOMPurify from "dompurify";
import hljs from "highlight.js/lib/common";
import { ChevronRight, FilePenLine, FilePlus2, FileX2 } from "@lucide/vue";
import { codeLanguageForPath } from "@/services/chat/codeLanguage";
import type { DiffHunk, DiffLineKind } from "@/services/chat/toolDiff";
import { fileBasename } from "@/services/chat/toolDiff";
import { textIncludesQuery } from "@/services/chat/conversationFind";
import { useExpandForFind } from "@/composables/chat/useConversationFind";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";

const props = withDefaults(
  defineProps<{
    path: string;
    hunk: DiffHunk;
    kind?: string;
    status?: "running" | "done" | "error" | string;
    startCollapsed?: boolean;
  }>(),
  {
    startCollapsed: false,
  },
);

const settingStore = useSettingStore();
const expanded = ref(!props.startCollapsed || props.status === "running");
const runningLabel = computed(() => tr(settingStore.language, "running"));
const failedLabel = computed(() => tr(settingStore.language, "failed"));

const name = computed(() => fileBasename(props.path));
const language = computed(() => codeLanguageForPath(props.path));
const fileIcon = computed(() => language.value.icon);
const added = computed(() => props.hunk.added);
const removed = computed(() => props.hunk.removed);
const kind = computed(() => props.kind ?? "edit");
const status = computed(() => props.status ?? "done");

watch(
  () => props.status,
  (next, prev) => {
    if (next === "running") expanded.value = true;
    else if (props.startCollapsed && prev === "running" && next !== "running") {
      expanded.value = false;
    }
  },
);

useExpandForFind(
  (query) =>
    textIncludesQuery(props.path, query) ||
    props.hunk.lines.some((line) => textIncludesQuery(line.text, query)),
  () => {
    expanded.value = true;
  },
);

const fallbackIcon = computed((): Component => {
  if (kind.value === "create") return FilePlus2;
  if (kind.value === "delete") return FileX2;
  return FilePenLine;
});

const highlightedLines = computed(() => {
  const languageId = language.value.id;
  return props.hunk.lines.map((line) => ({
    ...line,
    html: highlightLine(line.text, languageId),
  }));
});

function marker(kind: DiffLineKind) {
  if (kind === "addition") return "+";
  if (kind === "deletion") return "-";
  return " ";
}

function highlightLine(text: string, languageId: string): string {
  if (!text) return "";
  try {
    const highlighted =
      languageId && hljs.getLanguage(languageId)
        ? hljs.highlight(text, { language: languageId, ignoreIllegals: true }).value
        : hljs.highlightAuto(text).value;
    return DOMPurify.sanitize(highlighted);
  } catch {
    return escapeHtml(text);
  }
}

function escapeHtml(text: string) {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}
</script>

<style scoped>
.file-diff-card {
  width: 100%;
  box-sizing: border-box;
  margin: 4px 0 8px;
  border: 1px solid color-mix(in srgb, var(--peek-border) 88%, transparent);
  border-radius: 10px;
  background: color-mix(in srgb, var(--peek-input-bg) 92%, #0b0d10);
  overflow: hidden;
}
.file-diff-card.running {
  border-color: color-mix(in srgb, var(--peek-accent) 35%, var(--peek-border));
}
.file-diff-card.error {
  border-color: color-mix(in srgb, var(--destructive) 40%, var(--peek-border));
}
.file-diff-header {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  min-height: 34px;
  margin: 0;
  padding: 7px 12px;
  border: 0;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-border) 70%, transparent);
  background: color-mix(in srgb, var(--peek-panel) 55%, transparent);
  color: color-mix(in srgb, var(--peek-text) 88%, var(--peek-muted));
  font: inherit;
  font-size: 12px;
  line-height: 1.35;
  text-align: left;
  cursor: pointer;
}
.file-diff-card.collapsed .file-diff-header {
  border-bottom: 0;
}
.file-diff-chevron {
  flex: none;
  color: var(--peek-faint);
  transition: transform 140ms ease;
}
.file-diff-chevron.open {
  transform: rotate(90deg);
}
.file-diff-icon-img {
  flex: none;
  width: 14px;
  height: 14px;
  object-fit: contain;
}
.file-diff-icon {
  flex: none;
  display: inline-flex;
  color: var(--peek-muted);
}
.file-diff-card.create .file-diff-icon {
  color: #22c55e;
}
.file-diff-card.edit .file-diff-icon {
  color: #eab308;
}
.file-diff-card.delete .file-diff-icon {
  color: var(--destructive);
}
.file-diff-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 550;
}
.change-stats {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  flex: none;
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 650;
}
.change-stats .added {
  color: #22c55e;
}
.change-stats .removed {
  color: var(--destructive);
}
.file-diff-status {
  flex: none;
  font-size: 10px;
  color: var(--peek-muted);
}
.file-diff-status.error {
  color: var(--destructive);
}
.file-diff-body {
  max-height: var(--agent-card-max-height, 240px);
  overflow: auto;
  font-family: var(--font-mono);
  font-size: 11px;
  line-height: 1.55;
}
.diff-line {
  display: grid;
  grid-template-columns: 36px 36px 16px minmax(0, 1fr);
  min-width: 100%;
  white-space: pre;
}
.line-no {
  padding: 0 6px;
  text-align: right;
  color: color-mix(in srgb, var(--peek-muted) 72%, transparent);
  user-select: none;
}
.diff-marker {
  text-align: center;
  user-select: none;
  color: var(--peek-muted);
}
.diff-text {
  padding-right: 12px;
  overflow: hidden;
}
.diff-line.deletion {
  background: color-mix(in srgb, var(--destructive) 16%, transparent);
}
.diff-line.deletion .diff-marker {
  color: color-mix(in srgb, #fecaca 88%, var(--peek-text));
}
.diff-line.addition {
  background: color-mix(in srgb, #22c55e 16%, transparent);
}
.diff-line.addition .diff-marker {
  color: color-mix(in srgb, #bbf7d0 88%, var(--peek-text));
}
:global([data-theme="light"]) .diff-line.deletion .diff-marker,
:global([data-theme="cream"]) .diff-line.deletion .diff-marker {
  color: #991b1b;
}
:global([data-theme="light"]) .diff-line.addition .diff-marker,
:global([data-theme="cream"]) .diff-line.addition .diff-marker {
  color: #166534;
}

.file-diff-body :deep(.hljs-comment),
.file-diff-body :deep(.hljs-quote) {
  color: #7f8c98;
  font-style: italic;
}
.file-diff-body :deep(.hljs-keyword),
.file-diff-body :deep(.hljs-selector-tag),
.file-diff-body :deep(.hljs-type),
.file-diff-body :deep(.hljs-literal) {
  color: #c792ea;
}
.file-diff-body :deep(.hljs-string),
.file-diff-body :deep(.hljs-regexp),
.file-diff-body :deep(.hljs-attribute) {
  color: #addb67;
}
.file-diff-body :deep(.hljs-number),
.file-diff-body :deep(.hljs-symbol) {
  color: #f78c6c;
}
.file-diff-body :deep(.hljs-title),
.file-diff-body :deep(.hljs-section),
.file-diff-body :deep(.hljs-built_in) {
  color: #82aaff;
}
.file-diff-body :deep(.hljs-variable),
.file-diff-body :deep(.hljs-params) {
  color: #f07178;
}
</style>
