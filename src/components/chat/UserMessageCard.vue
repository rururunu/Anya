<script setup lang="ts">
import { computed, nextTick, onUnmounted, reactive, ref, watch } from "vue";
import {
  Bot,
  Check,
  Clock3,
  CornerDownLeft,
  File,
  Folder,
  Puzzle,
  Trash2,
  Undo2,
  X,
  Zap,
} from "@lucide/vue";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import ComposerEditable from "@/components/chat/ComposerEditable.vue";
import FileMentionPicker from "@/components/chat/input/FileMentionPicker.vue";
import HashMentionPicker from "@/components/chat/input/HashMentionPicker.vue";
import CommandSuggestions from "@/components/chat/input/CommandSuggestions.vue";
import { listSkills } from "@/commands/skills";
import { listWorkspaceFiles } from "@/commands/workspace";
import { slashCommands } from "@/commands/slash";
import UserMessageFooter from "@/components/chat/UserMessageFooter.vue";
import UserMessageImage from "@/components/chat/UserMessageImage.vue";
import { codeLanguageForPath } from "@/services/chat/codeLanguage";
import {
  formatMentionPath,
  formatResourceMention,
  normalizeMentionPath,
} from "@/services/chat/composerSegments";
import {
  activeFilePathMention,
  activeHashMention,
  filterHashMentionItems,
  isHashableAgentPlugin,
  type HashMentionItem,
} from "@/services/chat/hashMentions";
import { splitInlineTokenParts } from "@/services/chat/inlineTokenMarks";
import "@/services/chat/inlineTokenMarks.css";
import {
  mcpMentionIconUrl,
  mcpMentionLabel,
  pluginMentionIconUrl,
  pluginMentionLabel,
  prettyHashInstallId,
  skillMentionIconUrl,
  skillMentionLabel,
} from "@/services/chat/hashMentionDisplay";
import { lookupInstallIcon } from "@/services/iconCache";
import { parseSelectionAttachment } from "@/services/chat/selectionAttachment";
import { isSoftInjectContent, stripSoftInjectMarker } from "@/services/chat/softInject";
import { shouldFoldUserMessage, USER_MESSAGE_FOLD_VISIBLE_LINES } from "@/services/chat/longText";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";
import { usePluginsStore } from "@/stores/plugins";
import type { ChatMessage } from "@/types/chat";

const props = defineProps<{
  message: ChatMessage;
  sessionId: string;
  workspaceRoot?: string;
  canResend: boolean;
  busy: boolean;
  inlineEdit?: boolean;
  queued?: boolean;
}>();

const emit = defineEmits<{
  preview: [source: string];
  resend: [text: string];
  rewind: [];
  remove: [];
  guide: [];
}>();

const settingStore = useSettingStore();
const pluginsStore = usePluginsStore();
const editing = ref(false);
const draft = ref("");
const editCaret = ref(0);
const editSelectedIndex = ref(0);
const editSuggestionsRef = ref<HTMLElement | null>(null);
const editFiles = ref<string[]>([]);
const editCatalog = ref<HashMentionItem[]>([]);
const editCatalogLoading = ref(false);
const editFilesLoading = ref(false);
const dismissedEditSuggestion = ref<string | null>(null);
const expanded = ref(false);
const ignoreBlur = ref(false);
const composerRoot = ref<HTMLElement | null>(null);
const composerRef = ref<InstanceType<typeof ComposerEditable> | null>(null);
const brokenHashIcons = reactive<Record<string, boolean>>({});
const resolvedHashIcons = reactive<Record<string, string>>({});

const content = computed(() =>
  parseSelectionAttachment(stripSoftInjectMarker(props.message.content)),
);
const plainText = computed(() =>
  [content.value.message, content.value.selection].filter(Boolean).join("\n"),
);
const needsFold = computed(() => !props.inlineEdit && shouldFoldUserMessage(plainText.value));
const collapsed = computed(() => needsFold.value && !expanded.value && !editing.value);
const canEdit = computed(
  () =>
    props.canResend &&
    !props.busy &&
    !isSoftInjectContent(props.message.content) &&
    props.message.injected !== true,
);
const canSend = computed(() => editing.value && draft.value.trim().length > 0);
const activeEditCommand = computed(() => {
  if (!editing.value) return null;
  const before = draft.value.slice(0, editCaret.value);
  const match = /(^|\s)(\/[\w-]*)$/.exec(before);
  if (!match || (draft.value[editCaret.value] && !/\s/.test(draft.value[editCaret.value])))
    return null;
  return { start: before.length - match[2].length, query: match[2] };
});
const activeEditFile = computed(() =>
  editing.value && props.workspaceRoot ? activeFilePathMention(draft.value, editCaret.value) : null,
);
const activeEditHash = computed(() =>
  editing.value && !activeEditFile.value && !activeEditCommand.value
    ? activeHashMention(draft.value, editCaret.value)
    : null,
);
const editFileSuggestions = computed(() => {
  const query = activeEditFile.value?.query.toLowerCase() ?? "";
  return editFiles.value.filter((path) => path.toLowerCase().includes(query)).slice(0, 12);
});
const editHashSuggestions = computed(() =>
  activeEditHash.value ? filterHashMentionItems(editCatalog.value, activeEditHash.value.query) : [],
);
const editCommandSuggestions = computed(() =>
  activeEditCommand.value
    ? slashCommands
        .filter((item) => item.command.startsWith(activeEditCommand.value!.query.toLowerCase()))
        .map((item) => ({ ...item, description: tr(settingStore.language, item.descriptionKey) }))
    : [],
);
const editSuggestionKind = computed<"file" | "hash" | "command" | null>(() => {
  if (activeEditFile.value) return "file";
  if (activeEditHash.value) return "hash";
  if (activeEditCommand.value) return "command";
  return null;
});
const editSuggestionKey = computed(() =>
  editSuggestionKind.value ? `${editSuggestionKind.value}:${editCaret.value}:${draft.value}` : null,
);
const showEditSuggestions = computed(
  () =>
    editSuggestionKind.value !== null && dismissedEditSuggestion.value !== editSuggestionKey.value,
);
const editSuggestionCount = computed(() => {
  if (editSuggestionKind.value === "file") return editFileSuggestions.value.length;
  if (editSuggestionKind.value === "hash") return editHashSuggestions.value.length;
  return editCommandSuggestions.value.length;
});

type InlinePart =
  | { kind: "text"; text: string }
  | { kind: "mention"; path: string; name: string; isDir: boolean }
  | { kind: "skill"; id: string }
  | { kind: "mcp"; id: string }
  | { kind: "plugin"; id: string };

function inlineParts(text: string): InlinePart[] {
  return splitInlineTokenParts(text).map((part) => {
    if (part.kind === "mention") {
      return { kind: "mention", path: part.path, name: part.name, isDir: part.isDir };
    }
    if (part.kind === "skill" || part.kind === "mcp" || part.kind === "plugin") {
      return { kind: part.kind, id: part.id };
    }
    return { kind: "text", text: part.text };
  });
}

function fileIconForPath(path: string) {
  return codeLanguageForPath(normalizeMentionPath(path)).icon;
}

function hashKey(kind: "skill" | "mcp" | "plugin", id: string) {
  return `${kind}:${id}`;
}

function hashLabel(kind: "skill" | "mcp" | "plugin", id: string) {
  if (kind === "mcp") return mcpMentionLabel(id, settingStore.mcpServers ?? []);
  if (kind === "plugin") return pluginMentionLabel(id, pluginsStore.plugins);
  return skillMentionLabel(id);
}

function hashTitle(kind: "skill" | "mcp" | "plugin", id: string) {
  if (kind === "mcp") {
    const server = (settingStore.mcpServers ?? []).find((item) => item.id === id);
    return server?.qualifiedName?.trim() || prettyHashInstallId(id);
  }
  if (kind === "plugin") {
    return (
      pluginsStore.plugins.find((item) => item.id === id)?.name?.trim() || prettyHashInstallId(id)
    );
  }
  return prettyHashInstallId(id);
}

function hashIcon(kind: "skill" | "mcp" | "plugin", id: string) {
  const key = hashKey(kind, id);
  if (brokenHashIcons[key]) return null;
  if (resolvedHashIcons[key]) return resolvedHashIcons[key];
  if (kind === "plugin") {
    return pluginMentionIconUrl(id, pluginsStore.plugins);
  }
  const sync =
    kind === "mcp" ? mcpMentionIconUrl(id, settingStore.mcpServers ?? []) : skillMentionIconUrl(id);
  if (sync) {
    resolvedHashIcons[key] = sync;
    return sync;
  }
  void lookupInstallIcon(kind, id).then((local) => {
    if (local && !brokenHashIcons[key]) resolvedHashIcons[key] = local;
  });
  return null;
}

async function startEdit() {
  if (!canEdit.value || editing.value) return;
  editing.value = true;
  expanded.value = true;
  draft.value = content.value.message;
  await nextTick();
  composerRef.value?.focus();
  const len = draft.value.length;
  editCaret.value = len;
  composerRef.value?.setSelection(len, len);
  void loadEditResources();
}

async function loadEditResources() {
  editCatalogLoading.value = true;
  await pluginsStore.refresh().catch((error) => console.error("list_plugins failed:", error));
  const skills = await listSkills().catch((error) => {
    console.error("list_skills failed:", error);
    return [];
  });
  const enabledBuiltins = new Set(settingStore.enabledBuiltinSkills ?? []);
  editCatalog.value = [
    ...skills
      .filter((skill) => skill.source !== "builtin" || enabledBuiltins.has(skill.name))
      .map((skill) => ({
        kind: "skill" as const,
        id: skill.name,
        title: skill.title || skill.name,
        description: skill.description || undefined,
        iconUrl: skill.iconUrl ?? null,
        vendor: skill.qualifiedName?.trim() || undefined,
      })),
    ...pluginsStore.plugins.filter(isHashableAgentPlugin).map((plugin) => ({
      kind: "plugin" as const,
      id: plugin.id,
      title: plugin.name || plugin.id,
      description: plugin.description || undefined,
      iconUrl: plugin.icon ? pluginMentionIconUrl(plugin.id, pluginsStore.plugins) : null,
    })),
    ...(settingStore.mcpServers ?? [])
      .filter((server) => server.enabled !== false)
      .map((server) => ({
        kind: "mcp" as const,
        id: server.id,
        title: server.title || server.id,
        description: server.description || server.command || undefined,
        iconUrl: server.iconUrl ?? null,
        vendor: server.qualifiedName?.trim() || undefined,
      })),
  ];
  editCatalogLoading.value = false;
  if (props.workspaceRoot) {
    editFilesLoading.value = true;
    editFiles.value = await listWorkspaceFiles().catch((error) => {
      console.error("list_workspace_files failed:", error);
      return [];
    });
    editFilesLoading.value = false;
  }
}

function insertEditSuggestion(token: string, start: number, end: number) {
  const before = draft.value.slice(0, start);
  const after = draft.value.slice(end);
  const inserted = `${token}${after && /^\s/.test(after) ? "" : " "}`;
  const caret = before.length + inserted.length;
  dismissedEditSuggestion.value = null;
  editSelectedIndex.value = 0;
  composerRef.value?.setText(`${before}${inserted}${after}`, caret);
  composerRef.value?.focus({ preventScroll: true });
}

function selectEditSuggestion(index = editSelectedIndex.value) {
  if (editSuggestionKind.value === "file" && activeEditFile.value) {
    const path = editFileSuggestions.value[index];
    if (path)
      insertEditSuggestion(
        formatMentionPath(path),
        activeEditFile.value.start,
        activeEditFile.value.end,
      );
  } else if (editSuggestionKind.value === "hash" && activeEditHash.value) {
    const item = editHashSuggestions.value[index];
    if (item)
      insertEditSuggestion(
        formatResourceMention(item.kind, item.id),
        activeEditHash.value.start,
        activeEditHash.value.end,
      );
  } else if (editSuggestionKind.value === "command" && activeEditCommand.value) {
    const item = editCommandSuggestions.value[index];
    if (item) insertEditSuggestion(item.command, activeEditCommand.value.start, editCaret.value);
  }
}

function cancelEdit() {
  editing.value = false;
  draft.value = content.value.message;
  dismissedEditSuggestion.value = null;
}

function commitEdit() {
  if (!editing.value || !canSend.value || props.busy) return;
  emit("resend", draft.value);
  if (props.queued) editing.value = false;
}

function isInsideEditor(target: EventTarget | null) {
  if (!(target instanceof Node)) return false;
  if (composerRoot.value?.contains(target)) return true;
  return target instanceof Element && Boolean(target.closest(".user-footer-picker"));
}

let blurTimer = 0;
function scheduleBlurCheck() {
  window.clearTimeout(blurTimer);
  blurTimer = window.setTimeout(() => {
    if (!editing.value || ignoreBlur.value) return;
    if (isInsideEditor(document.activeElement)) return;
    if (document.querySelector(".user-footer-picker")) return;
    cancelEdit();
  }, 120);
}

onUnmounted(() => window.clearTimeout(blurTimer));

async function attachFiles() {
  if (!editing.value || props.busy) return;
  ignoreBlur.value = true;
  try {
    const selected = await openFileDialog({
      multiple: true,
      directory: false,
      title: tr(settingStore.language, "chatInput.attachPickFiles"),
    });
    const paths = (Array.isArray(selected) ? selected : selected ? [selected] : [])
      .map((entry) => String(entry ?? "").trim())
      .filter(Boolean);
    if (paths.length === 0) return;
    const tokens = paths.map((path) => formatMentionPath(path)).join(" ");
    draft.value = draft.value.trim() ? `${draft.value.trim()} ${tokens}` : tokens;
  } finally {
    ignoreBlur.value = false;
    await nextTick();
    composerRef.value?.focus();
  }
}

function onComposerKeydown(event: KeyboardEvent) {
  if (event.isComposing || event.keyCode === 229) return;
  if (showEditSuggestions.value) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const count = editSuggestionCount.value;
      if (count)
        editSelectedIndex.value =
          (editSelectedIndex.value + (event.key === "ArrowDown" ? 1 : -1) + count) % count;
      return;
    }
    if ((event.key === "Enter" || event.key === "Tab") && editSuggestionCount.value) {
      event.preventDefault();
      selectEditSuggestion();
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      dismissedEditSuggestion.value = editSuggestionKey.value;
      return;
    }
  }
  if (event.key === "Escape") {
    event.preventDefault();
    cancelEdit();
    return;
  }
  if (event.key === "Enter" && !event.shiftKey) {
    event.preventDefault();
    commitEdit();
  }
}

watch(editSuggestionKey, () => {
  editSelectedIndex.value = 0;
});

watch(editSelectedIndex, async () => {
  await nextTick();
  const list = editSuggestionsRef.value?.querySelector<HTMLElement>(".command-list");
  const active = list?.querySelector<HTMLElement>(".command-item.active");
  if (!list || !active) return;
  const top = active.offsetTop;
  const bottom = top + active.offsetHeight;
  if (top < list.scrollTop) list.scrollTop = top;
  else if (bottom > list.scrollTop + list.clientHeight) list.scrollTop = bottom - list.clientHeight;
});

watch(
  () => props.message.id,
  () => {
    editing.value = false;
    expanded.value = false;
    draft.value = "";
  },
);
</script>

<template>
  <div
    ref="composerRoot"
    class="user-composer"
    :class="{
      'is-editing': editing,
      'is-editable': canEdit && !editing,
      'is-collapsed': collapsed,
      'inline-edit': inlineEdit,
      'is-queued': queued,
    }"
    :title="
      inlineEdit
        ? undefined
        : canEdit && !editing
          ? tr(settingStore.language, queued ? 'queuedEdit' : 'editAndResend')
          : undefined
    "
    @click="startEdit"
    @focusout="scheduleBlurCheck"
  >
    <button
      v-if="canResend && !editing && !queued"
      type="button"
      class="user-rewind-btn"
      data-tauri-drag-region="false"
      :aria-label="tr(settingStore.language, 'rewind')"
      :title="tr(settingStore.language, 'rewind')"
      :disabled="busy"
      @click.stop="emit('rewind')"
    >
      <Undo2 :size="14" :stroke-width="2" aria-hidden="true" />
    </button>
    <div v-if="content.images?.length" class="user-images" data-tauri-drag-region="false">
      <UserMessageImage
        v-for="(img, idx) in content.images"
        :key="idx"
        :source="img"
        @preview="emit('preview', $event)"
      />
    </div>
    <div
      v-if="content.attachedFiles?.length"
      class="user-attached-files"
      data-tauri-drag-region="false"
    >
      <div
        v-for="(file, idx) in content.attachedFiles"
        :key="`${file.path}-${idx}`"
        class="user-file-chip"
        :class="{ skipped: Boolean(file.skipped) }"
        :title="file.skipped ? `${file.path} (${file.skipped})` : file.path"
      >
        <img
          v-if="fileIconForPath(file.path)"
          class="user-file-icon-img"
          :src="fileIconForPath(file.path) || ''"
          alt=""
        />
        <File v-else :size="12" :stroke-width="1.75" aria-hidden="true" />
        <span class="user-file-name">{{ file.name }}</span>
      </div>
    </div>
    <div
      v-if="content.message || content.selection || editing"
      class="user-composer-body"
      data-tauri-drag-region="false"
      @click="startEdit"
    >
      <ComposerEditable
        v-if="editing"
        ref="composerRef"
        v-model="draft"
        class="user-composer-field"
        multiline
        :mcp-servers="settingStore.mcpServers ?? []"
        :plugins="pluginsStore.plugins"
        :aria-expanded="showEditSuggestions"
        :file-catalog="editFiles"
        @caret-change="editCaret = $event"
        @keydown="onComposerKeydown"
        @click.stop
      />
      <div
        v-if="editing && showEditSuggestions"
        ref="editSuggestionsRef"
        class="user-edit-suggestions"
        @click.stop
      >
        <FileMentionPicker
          v-if="editSuggestionKind === 'file'"
          :loading="editFilesLoading"
          :suggestions="editFileSuggestions"
          :selected-index="editSelectedIndex"
          :loading-text="tr(settingStore.language, 'loadingFiles')"
          :empty-text="tr(settingStore.language, 'noMatchingFiles')"
          :ariaLabel="tr(settingStore.language, 'workspace')"
          @hover="editSelectedIndex = $event"
          @select="(path) => selectEditSuggestion(editFileSuggestions.indexOf(path))"
        />
        <HashMentionPicker
          v-else-if="editSuggestionKind === 'hash'"
          :loading="editCatalogLoading"
          :items="editHashSuggestions"
          :selected-index="editSelectedIndex"
          :loading-text="tr(settingStore.language, 'loadingHashMentions')"
          :empty-text="tr(settingStore.language, 'noMatchingHashMentions')"
          :ariaLabel="tr(settingStore.language, 'hashMentions')"
          :skill-label="tr(settingStore.language, 'hashSkill')"
          :mcp-label="tr(settingStore.language, 'hashMcp')"
          :plugin-label="tr(settingStore.language, 'hashPlugin')"
          @hover="editSelectedIndex = $event"
          @select="(item) => selectEditSuggestion(editHashSuggestions.indexOf(item))"
        />
        <CommandSuggestions
          v-else-if="editSuggestionKind === 'command'"
          :commands="editCommandSuggestions"
          :selected-index="editSelectedIndex"
          :appearance="inlineEdit ? 'overlay' : 'workbench'"
          :ariaLabel="tr(settingStore.language, 'commandSuggestions')"
          @hover="editSelectedIndex = $event"
          @select="
            (command) =>
              selectEditSuggestion(
                editCommandSuggestions.findIndex((item) => item.command === command),
              )
          "
        />
      </div>
      <span v-if="!editing && content.message" class="user-message-text">
        <template
          v-for="(part, partIdx) in inlineParts(content.message)"
          :key="`${message.id}-part-${partIdx}`"
        >
          <span
            v-if="part.kind === 'mention'"
            class="inline-token inline-token-mark inline-token-file"
            :class="{ 'is-dir': part.isDir }"
            :title="normalizeMentionPath(part.path)"
          >
            <Folder
              v-if="part.isDir"
              :size="12"
              class="inline-token-logo-fallback"
              aria-hidden="true"
            />
            <img
              v-else-if="fileIconForPath(part.path)"
              class="inline-token-logo"
              :src="fileIconForPath(part.path) || ''"
              alt=""
            />
            <File v-else :size="12" class="inline-token-logo-fallback" aria-hidden="true" />
            <span class="inline-token-label">@{{ part.name }}</span>
          </span>
          <span
            v-else-if="part.kind === 'skill'"
            class="inline-token inline-token-mark inline-token-skill"
            :title="hashTitle('skill', part.id)"
          >
            <img
              v-if="hashIcon('skill', part.id)"
              class="inline-token-logo"
              :src="hashIcon('skill', part.id) || ''"
              alt=""
              referrerpolicy="no-referrer"
              @error="brokenHashIcons[hashKey('skill', part.id)] = true"
            />
            <Zap v-else :size="12" class="inline-token-logo-fallback" aria-hidden="true" />
            <span class="inline-token-label">{{ hashLabel("skill", part.id) }}</span>
          </span>
          <span
            v-else-if="part.kind === 'mcp'"
            class="inline-token inline-token-mark inline-token-mcp"
            :title="hashTitle('mcp', part.id)"
          >
            <img
              v-if="hashIcon('mcp', part.id)"
              class="inline-token-logo"
              :src="hashIcon('mcp', part.id) || ''"
              alt=""
              referrerpolicy="no-referrer"
              @error="brokenHashIcons[hashKey('mcp', part.id)] = true"
            />
            <Bot v-else :size="12" class="inline-token-logo-fallback" aria-hidden="true" />
            <span class="inline-token-label">{{ hashLabel("mcp", part.id) }}</span>
          </span>
          <span
            v-else-if="part.kind === 'plugin'"
            class="inline-token inline-token-mark inline-token-plugin"
            :title="hashTitle('plugin', part.id)"
          >
            <img
              v-if="hashIcon('plugin', part.id)"
              class="inline-token-logo"
              :src="hashIcon('plugin', part.id) || ''"
              alt=""
              referrerpolicy="no-referrer"
              @error="brokenHashIcons[hashKey('plugin', part.id)] = true"
            />
            <Puzzle v-else :size="12" class="inline-token-logo-fallback" aria-hidden="true" />
            <span class="inline-token-label">{{ hashLabel("plugin", part.id) }}</span>
          </span>
          <template v-else>{{ part.text }}</template>
        </template>
      </span>
      <span v-if="content.selection && !editing" class="user-selection-quote">
        {{ content.selection }}
      </span>
    </div>
    <button
      v-if="needsFold && !editing && !queued"
      type="button"
      class="user-bubble-toggle"
      data-tauri-drag-region="false"
      @click.stop="expanded = !expanded"
    >
      {{ tr(settingStore.language, expanded ? "collapse" : "expand") }}
    </button>
    <UserMessageFooter
      v-if="editing && !inlineEdit"
      :session-id="sessionId"
      :can-send="canSend"
      :busy="busy"
      @attach="void attachFiles()"
      @send="commitEdit"
    />
    <div v-if="editing && inlineEdit" class="inline-edit-actions" @mousedown.prevent @click.stop>
      <button
        type="button"
        :disabled="!canSend || busy"
        :title="tr(settingStore.language, 'editAndResend')"
        :aria-label="tr(settingStore.language, 'editAndResend')"
        @click="commitEdit"
      >
        <Check :size="14" />
      </button>
      <button
        type="button"
        :title="tr(settingStore.language, 'rewindCancel')"
        :aria-label="tr(settingStore.language, 'rewindCancel')"
        @click="cancelEdit"
      >
        <X :size="14" />
      </button>
    </div>
    <div v-if="queued && !editing" class="queued-message-actions" data-tauri-drag-region="false">
      <span
        class="queued-state"
        :title="tr(settingStore.language, 'queuedWaiting')"
        :aria-label="tr(settingStore.language, 'queuedWaiting')"
      >
        <Clock3 :size="13" aria-hidden="true" />
      </span>
      <button
        type="button"
        :disabled="busy"
        :title="tr(settingStore.language, 'queuedGuide')"
        :aria-label="tr(settingStore.language, 'queuedGuide')"
        @click.stop="emit('guide')"
      >
        <CornerDownLeft :size="14" aria-hidden="true" />
      </button>
      <button
        type="button"
        :disabled="busy"
        :title="tr(settingStore.language, 'queuedDelete')"
        :aria-label="tr(settingStore.language, 'queuedDelete')"
        @click.stop="emit('remove')"
      >
        <Trash2 :size="14" aria-hidden="true" />
      </button>
    </div>
    <time
      v-if="inlineEdit && !editing && message.timestamp > 0"
      class="message-timestamp"
      :datetime="new Date(message.timestamp).toISOString()"
    >
      {{ new Date(message.timestamp).toLocaleString(settingStore.language) }}
    </time>
  </div>
</template>

<style scoped>
.user-composer.is-queued:not(.is-editing) {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 12px;
}
.message-timestamp {
  display: block;
  grid-column: 1 / -1;
  margin-top: -4px;
  color: color-mix(in srgb, var(--peek-muted) 72%, transparent);
  font-size: 11px;
  font-weight: 400;
  line-height: 16px;
  opacity: 0;
  pointer-events: none;
}
.user-composer:hover .message-timestamp,
.user-composer:focus-within .message-timestamp {
  opacity: 1;
}
.is-queued:not(.is-editing) .user-composer-body {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.is-queued:not(.is-editing) .user-message-text {
  white-space: nowrap;
}
.queued-message-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  flex: none;
  margin: 0;
  font-size: 11px;
  color: var(--peek-muted);
}
.queued-state {
  display: inline-flex;
  align-items: center;
  padding: 2px;
  color: var(--peek-muted);
}
.queued-message-actions button {
  border: none;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
  padding: 2px 4px;
  border-radius: 4px;
}
.queued-message-actions button:hover:not(:disabled) {
  background: var(--peek-list-active);
  color: var(--peek-text);
}
.user-composer.inline-edit.is-editing {
  padding-right: 60px;
  border: none;
  background: transparent;
  box-shadow: none;
}
.inline-edit :deep(.composer-editable) {
  padding: 0;
  border: none;
  outline: none;
  background: transparent;
  font: inherit;
  line-height: inherit;
}
.inline-edit-actions {
  position: absolute;
  top: 6px;
  right: 0;
  display: flex;
  gap: 2px;
}
.inline-edit-actions button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: none;
  border-radius: 5px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}
.inline-edit-actions button:hover:not(:disabled) {
  background: var(--peek-list-active);
  color: var(--peek-text);
}
.inline-edit-actions button:disabled {
  opacity: 0.4;
  cursor: default;
}
.user-composer {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 8px;
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
  padding: 8px 36px 8px 12px;
  border: 1px solid
    var(--peek-composer-border, color-mix(in srgb, var(--peek-text) 16%, transparent));
  border-radius: var(--peek-radius-composer, 16px);
  background: var(--peek-composer-fill, var(--peek-user-bubble-bg));
  color: var(--peek-user-bubble-text, var(--peek-text));
  font-size: var(--peek-font-md, 14px);
  font-weight: 450;
  line-height: 1.5;
  letter-spacing: -0.01em;
  box-shadow: var(--peek-composer-shadow, none);
  transition: border-color var(--motion-fast, 110ms) var(--motion-ease-out, ease);
}
.user-composer.is-editable {
  cursor: text;
}
.user-composer.is-editing {
  padding-right: 12px;
  border-color: var(
    --peek-composer-border-focus,
    var(--peek-composer-border, color-mix(in srgb, var(--peek-text) 16%, transparent))
  );
  box-shadow: var(--peek-composer-shadow-focus, none);
}
.user-images {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-start;
  gap: 8px;
  max-width: 100%;
}
.user-attached-files {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  max-width: 100%;
}
.user-file-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: min(220px, 100%);
  min-height: 28px;
  padding: 6px 10px;
  border: 1px solid var(--peek-border, color-mix(in srgb, var(--peek-text) 12%, transparent));
  border-radius: 10px;
  background: color-mix(in srgb, var(--peek-surface, var(--peek-list-bg)) 70%, transparent);
  color: inherit;
  font-size: 12px;
  font-weight: 500;
  line-height: 1.3;
}
.user-file-icon-img {
  flex: none;
  width: 13px;
  height: 13px;
  object-fit: contain;
}
.user-file-chip.skipped {
  opacity: 0.55;
}
.user-file-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.user-composer-body {
  min-width: 0;
  white-space: pre-wrap;
  word-break: break-word;
  overflow-wrap: anywhere;
}
.user-composer.is-collapsed .user-composer-body {
  max-height: calc(1.5em * v-bind(USER_MESSAGE_FOLD_VISIBLE_LINES));
  overflow: hidden;
  -webkit-mask-image: linear-gradient(to bottom, #000 62%, transparent 100%);
  mask-image: linear-gradient(to bottom, #000 62%, transparent 100%);
}
.user-composer-field {
  width: 100%;
  color: inherit;
}
.user-edit-suggestions {
  width: min(420px, 100%);
  margin-top: 6px;
  border: 1px solid var(--peek-border);
  border-radius: 10px;
  background: var(--peek-list-bg);
  overflow: hidden;
}
.user-edit-suggestions :deep(.command-list) {
  width: 100%;
  max-height: 180px;
  margin: 0;
  border: 0;
}
.user-composer :deep(.composer-editable) {
  color: inherit;
}
.user-message-text {
  display: inline;
  min-width: 0;
}
.user-bubble-toggle {
  align-self: flex-start;
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: color-mix(in srgb, currentColor 62%, transparent);
  font: inherit;
  font-size: 12px;
  font-weight: 500;
  line-height: 1.3;
  cursor: pointer;
}
.user-bubble-toggle:hover {
  color: inherit;
}
.user-selection-quote {
  display: block;
  margin-top: 6px;
  color: color-mix(in srgb, currentColor 70%, var(--peek-muted));
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}
.user-rewind-btn {
  position: absolute;
  top: 4px;
  right: 4px;
  z-index: 1;
  width: 26px;
  height: 26px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  margin: 0;
  padding: 0;
  border: 0;
  border-radius: 50%;
  background: transparent;
  color: color-mix(in srgb, var(--peek-muted, currentColor) 82%, transparent);
  cursor: pointer;
  opacity: 0.72;
  transition:
    opacity 120ms ease,
    color 120ms ease,
    background-color 120ms ease;
}
.user-rewind-btn:hover:not(:disabled) {
  opacity: 1;
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 8%, transparent);
}
.user-rewind-btn:disabled {
  cursor: default;
  opacity: 0.35;
}
</style>
