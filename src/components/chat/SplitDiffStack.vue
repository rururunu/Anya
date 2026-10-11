<template>
  <div ref="root" class="diff-stack peek-scrollbar">
    <article
      v-for="file in files"
      :key="file.id"
      :ref="(el) => registerArticle(file.id, el)"
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
        v-if="mountedIds.has(file.id)"
        :old-text="file.oldText"
        :new-text="file.newText"
        :unified-diff="file.diff"
        :language="file.language.id"
        :wrap-lines="wrapLines"
        :view-mode="viewMode"
      />
      <div v-else class="split-file-pending" :style="{ height: `${placeholderHeight(file)}px` }" />
    </article>
  </div>
</template>

<script setup lang="ts">
import {
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
  type ComponentPublicInstance,
} from "vue";
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
  viewMode: "split" | "unified";
  active: boolean;
}>();

/** Rough split-row height; only used while a body is still a placeholder. */
const ROW_HEIGHT = 18;
const MAX_ESTIMATE_ROWS = 2000;
/** Mount bodies this far ahead so the placeholder swap happens off-screen. */
const PREMOUNT_MARGIN = "800px 0px";

const root = ref<HTMLElement | null>(null);
const mountedIds = ref<Set<string>>(new Set());
const articles = new Map<string, HTMLElement>();
const measuredHeights = new Map<string, number>();
let observer: IntersectionObserver | null = null;
let cursorFile = 0;
let cursorHunk = -1;

/** Keep only nearby diff bodies mounted while retaining file headers and heights. */
function registerArticle(id: string, element: Element | ComponentPublicInstance | null) {
  const host = element instanceof HTMLElement ? element : null;
  const previous = articles.get(id);
  if (previous && previous !== host) {
    observer?.unobserve(previous);
    articles.delete(id);
  }
  if (!host) return;
  articles.set(id, host);
  observer?.observe(host);
}

function mount(id: string) {
  if (!props.active) return;
  if (mountedIds.value.has(id)) return;
  mountedIds.value = new Set(mountedIds.value).add(id);
}

function placeholderHeight(file: SplitDiffFile): number {
  return measuredHeights.get(file.id) ?? estimateHeight(file);
}

function unmount(id: string) {
  if (!mountedIds.value.has(id)) return;
  const body = articles.get(id)?.querySelector<HTMLElement>(".code-diff-editor");
  if (body && body.offsetHeight > 0) measuredHeights.set(id, body.offsetHeight);
  const next = new Set(mountedIds.value);
  next.delete(id);
  mountedIds.value = next;
}

function countLines(text: string): number {
  if (!text) return 0;
  let count = 1;
  for (let index = 0; index < text.length; index += 1) {
    if (text.charCodeAt(index) === 10) count += 1;
  }
  return count;
}

function estimateHeight(file: SplitDiffFile): number {
  const lines =
    file.oldText != null && file.newText != null
      ? Math.max(countLines(file.oldText), countLines(file.newText))
      : (file.added + file.removed) * 2 + 4;
  return Math.max(72, Math.min(lines, MAX_ESTIMATE_ROWS) * ROW_HEIGHT);
}

function changeStarts(fileId: string): HTMLElement[] {
  const article = root.value?.querySelector<HTMLElement>(
    `[data-change-id="${CSS.escape(fileId)}"]`,
  );
  const selector = props.viewMode === "split" ? ".split-row" : ".cm-line";
  const rows = [...(article?.querySelectorAll<HTMLElement>(selector) ?? [])];
  const changed = (element: HTMLElement) =>
    props.viewMode === "split"
      ? element.classList.contains("is-change")
      : element.classList.contains("diff-addition") || element.classList.contains("diff-deletion");
  return rows.filter(
    (element, index) => changed(element) && !(rows[index - 1] && changed(rows[index - 1]!)),
  );
}

/**
 * Jump to the next added/removed hunk across stacked files. A file is mounted on
 * the way in, so hunks in a not-yet-rendered block are still reachable.
 */
function nextChange(): string {
  const total = props.files.length;
  if (!total) return "";

  for (let step = 0; step < total; step += 1) {
    const index = (cursorFile + step) % total;
    const file = props.files[index];
    if (!file) continue;

    if (!mountedIds.value.has(file.id)) {
      mount(file.id);
      cursorFile = index;
      cursorHunk = 0;
      void nextTick(() => {
        const starts = changeStarts(file.id);
        if (!starts.length) return;
        starts[0]?.scrollIntoView({ block: "center", inline: "nearest" });
        if (starts.length === 1) {
          cursorFile = (index + 1) % total;
          cursorHunk = -1;
        }
      });
      return file.id;
    }

    const starts = changeStarts(file.id);
    if (!starts.length) continue;

    cursorFile = index;
    cursorHunk = (cursorHunk + 1) % starts.length;
    starts[cursorHunk]?.scrollIntoView({ block: "center", inline: "nearest" });
    if (cursorHunk === starts.length - 1) {
      cursorFile = (index + 1) % total;
      cursorHunk = -1;
    }
    return file.id;
  }

  return "";
}

/** Scroll the stacked file block to the top of the pane. */
function scrollToFile(id: string) {
  root.value
    ?.querySelector<HTMLElement>(`[data-change-id="${CSS.escape(id)}"]`)
    ?.scrollIntoView({ block: "start" });
}

defineExpose({ nextChange, scrollToFile });

watch(
  () => props.files,
  () => {
    cursorFile = 0;
    cursorHunk = -1;
    const live = new Set(props.files.map((file) => file.id));
    for (const id of measuredHeights.keys()) if (!live.has(id)) measuredHeights.delete(id);
  },
);
watch(
  () => [props.viewMode, props.wrapLines],
  () => measuredHeights.clear(),
);

watch(
  () => props.active,
  (active) => {
    if (!active) {
      for (const id of [...mountedIds.value]) unmount(id);
    } else {
      if (!observer && typeof IntersectionObserver === "undefined") {
        mountedIds.value = new Set(props.files.map((file) => file.id));
      }
      for (const article of articles.values()) observer?.observe(article);
    }
  },
);

onMounted(() => {
  if (typeof IntersectionObserver === "undefined") {
    // Without an observer, render everything rather than nothing.
    if (props.active) mountedIds.value = new Set(props.files.map((file) => file.id));
    return;
  }
  observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        const id = (entry.target as HTMLElement).dataset.changeId;
        if (!id) continue;
        if (entry.isIntersecting) mount(id);
        else unmount(id);
      }
    },
    { root: root.value, rootMargin: PREMOUNT_MARGIN },
  );
  for (const element of articles.values()) observer.observe(element);
});

onBeforeUnmount(() => {
  observer?.disconnect();
  observer = null;
  articles.clear();
});
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
.diff-stack :deep(.code-diff-editor.is-unified) {
  height: auto;
}
.diff-stack :deep(.code-diff-editor.is-unified .editor-pane) {
  height: auto;
  overflow: visible;
}
.diff-stack :deep(.code-diff-editor.is-unified .cm-scroller) {
  overflow: visible;
}
.split-file-pending {
  margin: 0 6px;
  border-radius: 6px;
  background: color-mix(in srgb, var(--peek-text) 4%, transparent);
}
</style>
