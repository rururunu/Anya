<template>
  <aside
    class="diff-sidebar"
    :class="{ embedded }"
    :style="{ width: `${props.width}px` }"
    data-tauri-drag-region="false"
    data-no-drag
    :aria-label="tr(settingStore.language, 'codeChanges')"
  >
    <div v-if="visibleChanges.length" class="diff-sidebar-body">
      <header class="diff-summary">
        <div class="diff-summary-title">
          <FileDiff :size="14" :stroke-width="1.7" aria-hidden="true" />
          <strong>
            {{ tr(settingStore.language, "editedFiles", { count: visibleChanges.length }) }}
          </strong>
        </div>
        <span class="diff-summary-stats">
          <span class="added">+{{ totals.added }}</span>
          <span class="removed">-{{ totals.removed }}</span>
        </span>
      </header>

      <nav
        ref="changeFilesRef"
        class="change-files peek-card-tabs"
        :aria-label="tr(settingStore.language, 'changedFiles')"
        @wheel="scrollFileTabs"
      >
        <div
          v-for="change in visibleChanges"
          :key="change.id"
          class="change-file-shell peek-card-tab"
          :class="{ active: change.id === activeId }"
        >
          <button
            type="button"
            class="change-file"
            :title="change.path"
            @click="activeId = change.id"
          >
            <img
              v-if="change.language.icon"
              class="language-file-icon"
              :src="change.language.icon"
              :alt="change.language.label"
              :title="change.language.label"
            />
            <span
              v-else
              class="language-file-icon language-file-icon-fallback"
              :data-family="change.language.family"
              :title="change.language.label"
              aria-hidden="true"
            >
              {{ change.language.badge }}
            </span>
            <span class="change-file-name">{{ fileName(change.path) }}</span>
            <span class="change-file-stats">
              <span class="added">+{{ change.added }}</span>
              <span class="removed">-{{ change.removed }}</span>
            </span>
          </button>
          <button
            type="button"
            class="change-file-close"
            :aria-label="closeTabLabel"
            :title="closeTabLabel"
            @click.stop="closeChange(change.id)"
          >
            <X :size="11" />
          </button>
        </div>
      </nav>

      <section v-if="activeChange" class="diff-view">
        <header class="diff-view-header">
          <div class="active-file">
            <img
              v-if="activeChange.language.icon"
              class="active-file-icon"
              :src="activeChange.language.icon"
              alt=""
              aria-hidden="true"
            />
            <span v-else class="active-file-icon language-file-icon-fallback" aria-hidden="true">
              {{ activeChange.language.badge }}
            </span>
            <div class="active-file-label">
              <strong :title="activeChange.path">{{ fileName(activeChange.path) }}</strong>
              <span class="diff-path" :title="activeChange.path">
                {{ parentPath(activeChange.path) }}
              </span>
            </div>
            <span class="active-file-stats">
              <span class="added">+{{ activeChange.added }}</span>
              <span class="removed">-{{ activeChange.removed }}</span>
            </span>
          </div>
          <div class="diff-view-actions">
            <button
              type="button"
              class="icon-button"
              :aria-label="tr(settingStore.language, 'diff.openFile')"
              :title="tr(settingStore.language, 'diff.openFile')"
              @click.stop="openActiveFile"
            >
              <ExternalLink :size="13" aria-hidden="true" />
            </button>
            <button
              type="button"
              class="icon-button"
              :aria-label="tr(settingStore.language, 'diff.showInFolder')"
              :title="tr(settingStore.language, 'diff.showInFolder')"
              @click.stop="revealActiveFile"
            >
              <FolderOpen :size="13" aria-hidden="true" />
            </button>
            <div
              class="view-mode-switch"
              role="group"
              :aria-label="tr(settingStore.language, 'diffViewMode')"
            >
              <button
                type="button"
                :class="{ active: viewMode === 'unified' }"
                :aria-label="tr(settingStore.language, 'diffUnified')"
                :title="tr(settingStore.language, 'diffUnified')"
                @click.stop="setViewMode('unified')"
              >
                <Rows3 :size="14" aria-hidden="true" />
              </button>
              <button
                type="button"
                :class="{ active: viewMode === 'split' }"
                :aria-label="tr(settingStore.language, 'diffSplit')"
                :title="tr(settingStore.language, 'diffSplit')"
                @click.stop="setViewMode('split')"
              >
                <Columns2 :size="14" aria-hidden="true" />
              </button>
            </div>
            <button
              type="button"
              class="icon-button"
              :class="{ active: wrapLines }"
              :aria-pressed="wrapLines"
              :aria-label="
                tr(settingStore.language, wrapLines ? 'disableDiffWrap' : 'enableDiffWrap')
              "
              :title="tr(settingStore.language, wrapLines ? 'disableDiffWrap' : 'enableDiffWrap')"
              @click.stop="toggleLineWrap"
            >
              <TextWrap :size="14" aria-hidden="true" />
            </button>
            <button
              type="button"
              class="icon-button"
              :class="{ copied: copiedId === activeChange.id }"
              :aria-label="
                tr(settingStore.language, copiedId === activeChange.id ? 'copied' : 'copyDiff')
              "
              :title="
                tr(settingStore.language, copiedId === activeChange.id ? 'copied' : 'copyDiff')
              "
              @click.stop="copyActiveDiff"
            >
              <Check v-if="copiedId === activeChange.id" :size="14" aria-hidden="true" />
              <Copy v-else :size="14" aria-hidden="true" />
            </button>
          </div>
        </header>
        <CodeDiffEditor
          :key="`${activeChange.id}:${viewMode}`"
          :old-text="activeChange.oldText"
          :new-text="activeChange.newText"
          :unified-diff="activeChange.diff"
          :language="activeChange.language.id"
          :view-mode="viewMode"
          :wrap-lines="wrapLines"
        />
      </section>
    </div>
    <div v-else class="diff-empty">
      <FileDiff :size="28" :stroke-width="1.35" aria-hidden="true" />
      <p>{{ tr(settingStore.language, "noCodeChanges") }}</p>
      <span>{{ tr(settingStore.language, "changesAppearHere") }}</span>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import {
  Check,
  Columns2,
  Copy,
  ExternalLink,
  FileDiff,
  FolderOpen,
  Rows3,
  TextWrap,
  X,
} from "@lucide/vue";
import CodeDiffEditor from "@/components/chat/CodeDiffEditor.vue";
import { copyText } from "@/services/clipboard";
import { extractCodeChanges, resolveChangeFilePath } from "@/services/chat/codeChanges";
import { codeLanguageForPath, type CodeLanguageInfo } from "@/services/chat/codeLanguage";
import { fileBasename, fileParentDir } from "@/services/chat/toolDiff";
import { openInDefaultApp, revealInExplorer } from "@/services/ipc";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage } from "@/types/chat";

type DiffViewMode = "unified" | "split";
type ChangeEntry = ReturnType<typeof extractCodeChanges>[number] & { language: CodeLanguageInfo };

const props = defineProps<{
  messages: ChatMessage[];
  width: number;
  embedded?: boolean;
  focusPath?: string;
  focusAt?: number;
}>();
const settingStore = useSettingStore();
const activeId = ref("");
const closedIds = ref<Set<string>>(new Set());
const changeFilesRef = ref<HTMLElement | null>(null);
const copiedId = ref("");
const storedViewMode = localStorage.getItem("anya.diffViewMode");
const viewMode = ref<DiffViewMode>(storedViewMode === "unified" ? "unified" : "split");
const wrapLines = ref(localStorage.getItem("anya.diffWrapLines") !== "false");
let copyResetTimer: ReturnType<typeof setTimeout> | null = null;

const closeTabLabel = computed(() => tr(settingStore.language, "diff.closeTab"));

const allChanges = computed<ChangeEntry[]>(() =>
  extractCodeChanges(props.messages.filter((message) => message.status === "done"))
    .map((change) => ({ ...change, language: codeLanguageForPath(change.path) }))
    .reverse(),
);

const visibleChanges = computed(() =>
  allChanges.value.filter((change) => !closedIds.value.has(change.id)),
);

const activeChange = computed(
  () =>
    visibleChanges.value.find((change) => change.id === activeId.value) ?? visibleChanges.value[0],
);
const totals = computed(() =>
  visibleChanges.value.reduce(
    (result, change) => ({
      added: result.added + change.added,
      removed: result.removed + change.removed,
    }),
    { added: 0, removed: 0 },
  ),
);

watch(
  allChanges,
  (next) => {
    const liveIds = new Set(next.map((change) => change.id));
    // Drop dismissals for files that are no longer in the change set.
    const pruned = new Set([...closedIds.value].filter((id) => liveIds.has(id)));
    if (pruned.size !== closedIds.value.size) {
      closedIds.value = pruned;
    }
  },
  { immediate: true },
);

watch(
  visibleChanges,
  (next) => {
    if (!next.some((change) => change.id === activeId.value)) {
      activeId.value = next[0]?.id ?? "";
    }
  },
  { immediate: true },
);

watch(
  () => [props.focusPath, props.focusAt] as const,
  ([path]) => {
    if (!path) return;
    focusChange(path);
  },
  { immediate: true },
);

watch(activeId, () => {
  void nextTick(() => {
    const tabs = changeFilesRef.value;
    const active = tabs?.querySelector<HTMLElement>(".change-file-shell.active");
    active?.scrollIntoView({ inline: "nearest", block: "nearest" });
  });
});

function normalizeChangePath(path: string) {
  return path.replace(/\\/g, "/").replace(/\/+$/, "");
}

function focusChange(path: string) {
  const needle = normalizeChangePath(path);
  const match =
    allChanges.value.find((change) => normalizeChangePath(change.path) === needle) ??
    allChanges.value.find((change) => {
      const current = normalizeChangePath(change.path);
      return current.endsWith(`/${needle}`) || needle.endsWith(`/${current}`);
    });
  if (!match) return;
  if (closedIds.value.has(match.id)) {
    const next = new Set(closedIds.value);
    next.delete(match.id);
    closedIds.value = next;
  }
  activeId.value = match.id;
}

function closeChange(id: string) {
  const next = new Set(closedIds.value);
  next.add(id);
  closedIds.value = next;
  if (activeId.value === id) {
    activeId.value = visibleChanges.value[0]?.id ?? "";
  }
}

function scrollFileTabs(event: WheelEvent) {
  const tabs = changeFilesRef.value;
  if (!tabs || tabs.scrollWidth <= tabs.clientWidth) return;
  event.preventDefault();
  tabs.scrollLeft += Math.abs(event.deltaY) >= Math.abs(event.deltaX) ? event.deltaY : event.deltaX;
}
function setViewMode(mode: DiffViewMode) {
  viewMode.value = mode;
  localStorage.setItem("anya.diffViewMode", mode);
}
function toggleLineWrap() {
  wrapLines.value = !wrapLines.value;
  localStorage.setItem("anya.diffWrapLines", String(wrapLines.value));
}
function fileName(path: string) {
  return fileBasename(path);
}
function parentPath(path: string) {
  return fileParentDir(path);
}
async function openActiveFile() {
  const change = activeChange.value;
  if (!change) return;
  const path = resolveChangeFilePath(change, props.messages);
  if (!path) return;
  try {
    await openInDefaultApp(path);
  } catch {
    try {
      await revealInExplorer(path);
    } catch {
      /* ignore */
    }
  }
}
async function revealActiveFile() {
  const change = activeChange.value;
  if (!change) return;
  const path = resolveChangeFilePath(change, props.messages);
  if (!path) return;
  try {
    await revealInExplorer(path);
  } catch {
    try {
      await openInDefaultApp(path);
    } catch {
      /* ignore */
    }
  }
}
async function copyActiveDiff() {
  const change = activeChange.value;
  if (!change) return;
  await copyText(change.diff);
  copiedId.value = change.id;
  if (copyResetTimer) clearTimeout(copyResetTimer);
  copyResetTimer = setTimeout(() => {
    copiedId.value = "";
    copyResetTimer = null;
  }, 1600);
}
</script>

<style scoped>
.diff-sidebar {
  flex: none;
  box-sizing: border-box;
  width: 520px;
  min-width: 320px;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding-top: 34px;
  background: transparent;
  color: var(--peek-text);
}
.diff-sidebar.embedded {
  flex: 1;
  width: 100% !important;
  min-width: 0;
  padding-top: 0;
}
.diff-sidebar-body,
.diff-view {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.diff-view {
  background: transparent;
}
.diff-view :deep(.code-diff-editor) {
  flex: 1;
  min-height: 0;
}
.diff-summary {
  flex: none;
  min-height: 28px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin: 4px 6px 2px;
  padding: 0 8px;
  border-radius: 5px;
  background: color-mix(in srgb, var(--peek-text) 3%, transparent);
}
.diff-summary-title {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--peek-text);
}
.diff-summary-title svg {
  flex: none;
  color: var(--peek-muted);
}
.diff-summary-title strong {
  overflow: hidden;
  font-size: 11px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.diff-summary-stats,
.active-file-stats {
  flex: none;
  display: inline-flex;
  gap: 6px;
  font: 600 10px/1 var(--font-mono);
  font-variant-numeric: tabular-nums;
}
.change-files {
  flex: none;
  min-height: 32px;
  margin: 0 6px 2px;
  padding: 2px;
  gap: 2px;
  border-radius: 6px;
  background: color-mix(in srgb, var(--peek-text) 2%, transparent);
}
.change-file-shell {
  position: relative;
  flex: 0 0 auto;
  min-width: 0;
  max-width: 280px;
  height: 28px;
  gap: 0;
  padding: 0;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--peek-code-muted, var(--peek-muted));
}
.change-file-shell:hover {
  background: color-mix(in srgb, var(--peek-text) 4%, transparent);
  color: var(--peek-text);
}
.change-file-shell.active {
  background: color-mix(in srgb, var(--peek-text) 7%, transparent);
  color: var(--peek-text);
  box-shadow: 0 3px 10px color-mix(in srgb, #000 10%, transparent);
}
.change-file {
  flex: 1;
  min-width: 0;
  height: 100%;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 0 2px 0 8px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}
.change-file-name {
  flex: 0 1 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
  font-weight: 550;
}
.language-file-icon {
  flex: none;
  width: 15px;
  height: 15px;
  display: inline-grid;
  place-items: center;
  border-radius: 3px;
  object-fit: contain;
}
.language-file-icon-fallback {
  background: color-mix(in srgb, var(--peek-muted) 11%, transparent);
  color: var(--peek-muted);
  font: 700 9px/1 var(--font-mono);
}
.change-file-stats {
  flex: none;
  display: flex;
  gap: 4px;
  padding-left: 6px;
  font: 600 9px/1 var(--font-mono);
  font-variant-numeric: tabular-nums;
}
.change-file-close {
  position: relative;
  z-index: 1;
  flex: none;
  width: 20px;
  height: 20px;
  display: inline-grid;
  place-items: center;
  margin-right: 4px;
  padding: 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--peek-faint);
  cursor: pointer;
  opacity: 0;
}
.change-file-shell:hover .change-file-close,
.change-file-shell.active .change-file-close {
  opacity: 1;
}
.change-file-close:hover {
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 8%, transparent);
}
.added {
  color: #4ade80;
}
.removed {
  color: #fb7185;
}
.diff-view-header {
  flex: none;
  min-height: 30px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin: 0 6px 4px;
  padding: 2px 5px 2px 7px;
  border-radius: 5px;
  background: color-mix(in srgb, var(--peek-text) 2.5%, transparent);
}
.active-file {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  overflow: hidden;
}
.active-file-icon {
  flex: none;
  width: 15px;
  height: 15px;
  display: grid;
  place-items: center;
  object-fit: contain;
}
.active-file-label {
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 6px;
}
.active-file-label strong {
  overflow: hidden;
  color: var(--peek-text);
  font-size: 11px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.diff-path {
  display: none;
}
.active-file-stats {
  flex: none;
  margin-left: 4px;
}
.diff-view-actions {
  flex: none;
  display: flex;
  align-items: center;
  gap: 3px;
  flex-shrink: 0;
}
.view-mode-switch {
  display: inline-flex;
  height: 22px;
  padding: 1px;
  border: 1px solid color-mix(in srgb, var(--peek-text) 9%, var(--peek-border));
  border-radius: 4px;
  background: color-mix(in srgb, var(--peek-text) 3%, transparent);
}
.view-mode-switch button,
.icon-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--peek-icon, var(--peek-muted));
  cursor: pointer;
}
.view-mode-switch button {
  width: 22px;
  height: 18px;
  border-radius: 3px;
}
.view-mode-switch button.active,
.icon-button.active {
  background: color-mix(in srgb, var(--peek-accent) 16%, var(--peek-surface));
  color: var(--peek-accent);
}
.icon-button {
  flex: none;
  width: 24px;
  height: 24px;
  border: 0;
  border-radius: 4px;
}
.icon-button:hover {
  background: color-mix(in srgb, var(--peek-text) 6%, transparent);
  color: var(--peek-text);
}
.icon-button.copied {
  color: #4ade80;
}
.diff-empty {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 7px;
  padding: 28px;
  color: var(--peek-muted);
  text-align: center;
}
.diff-empty p {
  margin: 5px 0 0;
  color: var(--peek-text);
  font-size: 12px;
}
.diff-empty span {
  max-width: 240px;
  font-size: 10px;
  line-height: 1.5;
}
</style>
