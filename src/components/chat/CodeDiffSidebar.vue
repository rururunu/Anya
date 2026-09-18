<template>
  <aside
    class="diff-sidebar"
    :class="{ embedded }"
    :style="{ width: `${props.width}px` }"
    data-tauri-drag-region="false"
    data-no-drag
    :aria-label="tr(settingStore.language, 'codeChanges')"
  >
    <div class="diff-sidebar-body">
      <header class="diff-summary">
        <div class="diff-summary-title">
          <DiffScopePicker :model-value="diffScope" @update:model-value="setScope" />
          <strong v-if="visibleChanges.length">
            {{ tr(settingStore.language, "editedFiles", { count: visibleChanges.length }) }}
          </strong>
        </div>
        <span v-if="visibleChanges.length" class="diff-summary-stats">
          <span class="added">+{{ totals.added }}</span>
          <span class="removed">-{{ totals.removed }}</span>
        </span>
      </header>

      <div v-if="visibleChanges.length" class="diff-workspace">
        <section v-if="activeChange" class="diff-view">
          <header class="diff-view-header">
            <div v-if="viewMode !== 'split'" class="active-file">
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
              <strong class="active-file-path" :title="activeChange.path">
                {{ activeDisplayPath }}
              </strong>
              <span class="active-file-stats">
                <span class="added">+{{ activeChange.added }}</span>
                <span class="removed">-{{ activeChange.removed }}</span>
              </span>
            </div>
            <div class="diff-view-actions">
              <button
                type="button"
                class="icon-button"
                :title="settingStore.language === 'zh-CN' ? '下一处修改' : 'Next change'"
                :aria-label="settingStore.language === 'zh-CN' ? '下一处修改' : 'Next change'"
                @click="goNextChange"
              >
                ↓
              </button>
              <button
                v-if="visibleChanges.length > 1"
                type="button"
                class="icon-button"
                :title="settingStore.language === 'zh-CN' ? '下一个文件' : 'Next file'"
                :aria-label="settingStore.language === 'zh-CN' ? '下一个文件' : 'Next file'"
                @click="nextFile"
              >
                →
              </button>
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
          <SplitDiffStack
            v-if="viewMode === 'split'"
            ref="diffStackRef"
            :files="stackedFiles"
            :active-id="activeId"
            :wrap-lines="wrapLines"
          />
          <CodeDiffEditor
            v-else
            ref="diffEditorRef"
            :key="activeChange.id"
            :old-text="activeChange.oldText"
            :new-text="activeChange.newText"
            :unified-diff="activeChange.diff"
            :language="activeChange.language.id"
            view-mode="unified"
            :wrap-lines="wrapLines"
          />
        </section>
        <nav
          v-if="visibleChanges.length > 1"
          ref="changeFilesRef"
          class="change-tree peek-scrollbar"
          :aria-label="tr(settingStore.language, 'changedFiles')"
        >
          <div
            v-for="row in treeRows"
            :key="row.type === 'dir' ? `dir:${row.path}` : row.changeId"
            class="change-tree-row"
            :class="{
              dir: row.type === 'dir',
              file: row.type === 'file',
              active: row.type === 'file' && row.changeId === activeId,
            }"
            :style="{ paddingLeft: `${8 + row.depth * 12}px` }"
          >
            <button
              v-if="row.type === 'dir'"
              type="button"
              class="change-tree-dir"
              @click="toggleFolder(row.path)"
            >
              <ChevronRight
                class="change-tree-chevron"
                :class="{ open: row.expanded }"
                :size="12"
                :stroke-width="2.2"
                aria-hidden="true"
              />
              <span>{{ row.name }}</span>
            </button>
            <button
              v-else
              type="button"
              class="change-tree-file"
              :title="changeById(row.changeId)?.path"
              @click="activeId = row.changeId"
            >
              <img
                v-if="changeById(row.changeId)?.language.icon"
                class="language-file-icon"
                :src="changeById(row.changeId)?.language.icon"
                alt=""
                aria-hidden="true"
              />
              <span
                v-else
                class="language-file-icon language-file-icon-fallback"
                aria-hidden="true"
              >
                {{ changeById(row.changeId)?.language.badge }}
              </span>
              <span class="change-tree-name">{{ row.name }}</span>
              <component
                :is="changeKindIcon(row.changeId)"
                class="change-tree-mark"
                :data-kind="changeKind(row.changeId)"
                :size="12"
                :stroke-width="2.25"
                aria-hidden="true"
              />
            </button>
          </div>
        </nav>
      </div>
      <div v-else class="diff-empty">
        <FileDiff :size="28" :stroke-width="1.35" aria-hidden="true" />
        <p>{{ emptyTitle }}</p>
        <span>{{ emptyHint }}</span>
      </div>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import {
  Check,
  ChevronRight,
  Columns2,
  Copy,
  ExternalLink,
  FileDiff,
  FolderOpen,
  Rows3,
  SquareMinus,
  SquarePen,
  SquarePlus,
  TextWrap,
} from "@lucide/vue";
import CodeDiffEditor from "@/components/chat/CodeDiffEditor.vue";
import DiffScopePicker from "@/components/chat/DiffScopePicker.vue";
import SplitDiffStack from "@/components/chat/SplitDiffStack.vue";
import { useDiffScope } from "@/composables/chat/useDiffScope";
import { copyText } from "@/services/clipboard";
import { resolveChangeFilePath, type CodeChangeEntry } from "@/services/chat/codeChanges";
import { buildChangeFileTree, flattenChangeFileTree } from "@/services/chat/changeFileTree";
import { codeLanguageForPath, type CodeLanguageInfo } from "@/services/chat/codeLanguage";
import { openInDefaultApp, revealInExplorer } from "@/services/ipc";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage } from "@/types/chat";

type DiffViewMode = "unified" | "split";
type ChangeEntry = CodeChangeEntry & { language: CodeLanguageInfo };

const props = defineProps<{
  messages: ChatMessage[];
  width: number;
  embedded?: boolean;
  focusPath?: string;
  focusAt?: number;
}>();
const settingStore = useSettingStore();
const activeId = ref("");
const diffEditorRef = ref<InstanceType<typeof CodeDiffEditor> | null>(null);
const diffStackRef = ref<{
  nextChange: () => string;
  scrollToFile: (id: string) => void;
} | null>(null);
const closedIds = ref<Set<string>>(new Set());
const changeFilesRef = ref<HTMLElement | null>(null);
const collapsedFolders = ref<Set<string>>(new Set());
const copiedId = ref("");
const storedViewMode = localStorage.getItem("anya.diffViewMode");
const preferredViewMode = ref<DiffViewMode | null>(
  storedViewMode === "unified" || storedViewMode === "split" ? storedViewMode : null,
);
const viewMode = computed<DiffViewMode>(
  () => preferredViewMode.value ?? (props.width < 820 ? "unified" : "split"),
);
const wrapLines = ref(localStorage.getItem("anya.diffWrapLines") !== "false");
let copyResetTimer: ReturnType<typeof setTimeout> | null = null;
let skipFileScroll = false;

const { scope: diffScope, setScope, changes: scopedChanges } = useDiffScope(() => props.messages);

const allChanges = computed<ChangeEntry[]>(() =>
  scopedChanges.value.map((change) => ({ ...change, language: codeLanguageForPath(change.path) })),
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

const changeLookup = computed(() =>
  Object.fromEntries(visibleChanges.value.map((change) => [change.id, change])),
);

const treeRows = computed(() =>
  flattenChangeFileTree(
    buildChangeFileTree(
      visibleChanges.value.map((change) => ({ id: change.id, path: change.path })),
    ),
    collapsedFolders.value,
  ),
);

const activeDisplayPath = computed(() => fileDisplayPath(activeChange.value?.id ?? ""));

const stackedFiles = computed(() =>
  visibleChanges.value.map((change) => ({
    ...change,
    displayPath: fileDisplayPath(change.id),
  })),
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
    changeFilesRef.value
      ?.querySelector<HTMLElement>(".change-tree-row.active")
      ?.scrollIntoView({ block: "nearest" });
    if (viewMode.value === "split" && !skipFileScroll) {
      diffStackRef.value?.scrollToFile(activeId.value);
    }
    skipFileScroll = false;
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

function changeById(id: string) {
  return changeLookup.value[id];
}

function changeKind(id: string) {
  const change = changeById(id);
  if (!change) return "modified";
  if (change.added > 0 && change.removed === 0) return "added";
  if (change.added === 0 && change.removed > 0) return "deleted";
  return "modified";
}

function changeKindIcon(id: string) {
  const kind = changeKind(id);
  if (kind === "added") return SquarePlus;
  if (kind === "deleted") return SquareMinus;
  return SquarePen;
}

function toggleFolder(path: string) {
  const next = new Set(collapsedFolders.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  collapsedFolders.value = next;
}

function fileDisplayPath(id: string) {
  const row = treeRows.value.find((item) => item.type === "file" && item.changeId === id);
  return row?.path ?? changeById(id)?.path ?? "";
}

const emptyTitle = computed(() =>
  tr(
    settingStore.language,
    diffScope.value === "uncommitted" ? "noUncommittedChanges" : "noCodeChanges",
  ),
);
const emptyHint = computed(() =>
  tr(
    settingStore.language,
    diffScope.value === "uncommitted" ? "uncommittedChangesHint" : "changesAppearHere",
  ),
);

function setViewMode(mode: DiffViewMode) {
  preferredViewMode.value = mode;
  localStorage.setItem("anya.diffViewMode", mode);
}
function nextFile() {
  const index = visibleChanges.value.findIndex((change) => change.id === activeChange.value?.id);
  activeId.value = visibleChanges.value[(index + 1) % visibleChanges.value.length]?.id ?? "";
}
function goNextChange() {
  if (viewMode.value !== "split") {
    diffEditorRef.value?.nextChange();
    return;
  }
  const id = diffStackRef.value?.nextChange() ?? "";
  if (id && id !== activeId.value) {
    skipFileScroll = true;
    activeId.value = id;
  }
}
function toggleLineWrap() {
  wrapLines.value = !wrapLines.value;
  localStorage.setItem("anya.diffWrapLines", String(wrapLines.value));
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
.diff-sidebar-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.diff-workspace {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
}
.diff-view {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
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
.change-tree {
  flex: none;
  box-sizing: border-box;
  width: 196px;
  min-width: 148px;
  min-height: 0;
  overflow: auto;
  padding: 4px 4px 8px;
  border-left: 1px solid color-mix(in srgb, var(--peek-text) 8%, transparent);
}
.change-tree-row {
  min-height: 24px;
  display: flex;
  align-items: center;
}
.change-tree-dir,
.change-tree-file {
  width: 100%;
  min-width: 0;
  height: 24px;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 0 6px 0 2px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--peek-muted);
  text-align: left;
  cursor: pointer;
}
.change-tree-dir {
  font-size: 11px;
  font-weight: 550;
}
.change-tree-file {
  color: var(--peek-text);
}
.change-tree-dir:hover,
.change-tree-file:hover,
.change-tree-row.active .change-tree-file {
  background: color-mix(in srgb, var(--peek-text) 6%, transparent);
}
.change-tree-chevron {
  flex: none;
  color: var(--peek-faint);
  transition: transform 120ms ease;
}
.change-tree-chevron.open {
  transform: rotate(90deg);
}
.change-tree-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
}
.change-tree-mark {
  flex: none;
}
.change-tree-mark[data-kind="added"] {
  color: #3fb950;
}
.change-tree-mark[data-kind="modified"] {
  color: #d29922;
}
.change-tree-mark[data-kind="deleted"] {
  color: #f85149;
}
.language-file-icon {
  flex: none;
  width: 14px;
  height: 14px;
  display: inline-grid;
  place-items: center;
  border-radius: 3px;
  object-fit: contain;
}
.language-file-icon-fallback {
  background: color-mix(in srgb, var(--peek-muted) 11%, transparent);
  color: var(--peek-muted);
  font: 700 8px/1 var(--font-mono);
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
.active-file-path {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  color: var(--peek-text);
  font-size: 11px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
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
  margin-left: auto;
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
@container workspace-sidebar (max-width: 620px) {
  .diff-workspace {
    flex-direction: column;
  }
  .change-tree {
    width: 100%;
    max-height: 148px;
    order: -1;
    border-left: 0;
    border-bottom: 1px solid color-mix(in srgb, var(--peek-text) 8%, transparent);
  }
}
</style>
