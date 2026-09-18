<template>
  <div ref="root" class="diff-stack peek-scrollbar">
    <article
      v-for="file in files"
      :key="file.id"
      class="split-file"
      :class="{ active: file.id === activeId }"
      :data-change-id="file.id"
    >
      <header class="split-file-head">
        <img
          v-if="file.language.icon"
          class="split-file-icon"
          :src="file.language.icon"
          alt=""
          aria-hidden="true"
        />
        <span v-else class="split-file-icon fallback" aria-hidden="true">
          {{ file.language.badge }}
        </span>
        <strong class="split-file-path" :title="file.path">{{ file.displayPath }}</strong>
        <span class="split-file-stats">
          <span class="added">+{{ file.added }}</span>
          <span class="removed">-{{ file.removed }}</span>
        </span>
      </header>
      <CodeDiffEditor
        :old-text="file.oldText"
        :new-text="file.newText"
        :unified-diff="file.diff"
        :language="file.language.id"
        view-mode="split"
        :wrap-lines="wrapLines"
      />
    </article>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import CodeDiffEditor from "@/components/chat/CodeDiffEditor.vue";
import type { CodeChangeEntry } from "@/services/chat/codeChanges";
import type { CodeLanguageInfo } from "@/services/chat/codeLanguage";

type SplitDiffFile = CodeChangeEntry & {
  language: CodeLanguageInfo;
  displayPath: string;
};

const props = defineProps<{
  files: SplitDiffFile[];
  activeId: string;
  wrapLines: boolean;
}>();

const root = ref<HTMLElement | null>(null);
let hunkIndex = -1;

watch(
  () => props.files,
  () => {
    hunkIndex = -1;
  },
);

/** Jump to the next added/removed hunk across stacked files. */
function nextChange() {
  const rows = [...(root.value?.querySelectorAll(".split-row") ?? [])];
  const starts = rows.filter(
    (element, index) =>
      element.classList.contains("is-change") && !rows[index - 1]?.classList.contains("is-change"),
  );
  if (!starts.length) return "";
  hunkIndex = (hunkIndex + 1) % starts.length;
  const target = starts[hunkIndex];
  target?.scrollIntoView({ block: "center", inline: "nearest" });
  return target?.closest<HTMLElement>("[data-change-id]")?.dataset.changeId ?? "";
}

/** Scroll the stacked file block to the top of the pane. */
function scrollToFile(id: string) {
  root.value
    ?.querySelector<HTMLElement>(`[data-change-id="${CSS.escape(id)}"]`)
    ?.scrollIntoView({ block: "start" });
}

defineExpose({ nextChange, scrollToFile });
</script>

<style scoped>
.diff-stack {
  flex: 1;
  min-height: 0;
  overflow: auto;
  overscroll-behavior: contain;
}
.split-file {
  display: flex;
  flex-direction: column;
}
.split-file + .split-file {
  margin-top: 10px;
}
.split-file.active .split-file-head {
  background: color-mix(in srgb, var(--peek-text) 6%, var(--peek-surface));
}
.split-file-head {
  position: sticky;
  top: 0;
  z-index: 2;
  display: flex;
  align-items: center;
  gap: 6px;
  min-height: 28px;
  margin: 0 6px;
  padding: 2px 8px;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-text) 8%, transparent);
  background: color-mix(in srgb, var(--peek-surface) 92%, transparent);
}
.split-file-icon {
  flex: none;
  width: 15px;
  height: 15px;
  display: grid;
  place-items: center;
  object-fit: contain;
}
.split-file-icon.fallback {
  border-radius: 3px;
  background: color-mix(in srgb, var(--peek-muted) 11%, transparent);
  color: var(--peek-muted);
  font: 700 8px/1 var(--font-mono);
}
.split-file-path {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  font-size: 11px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.split-file-stats {
  flex: none;
  display: inline-flex;
  gap: 6px;
  font: 600 10px/1 var(--font-mono);
  font-variant-numeric: tabular-nums;
}
.added {
  color: #4ade80;
}
.removed {
  color: #fb7185;
}
.diff-stack :deep(.code-diff-editor) {
  flex: none;
  overflow: visible;
}
</style>
