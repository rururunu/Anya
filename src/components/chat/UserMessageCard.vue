<script setup lang="ts">
import { computed, nextTick, onUnmounted, reactive, ref, watch } from "vue";
import { Bot, File, Folder, Puzzle, Undo2, Zap } from "@lucide/vue";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import ComposerEditable from "@/components/chat/ComposerEditable.vue";
import UserMessageFooter from "@/components/chat/UserMessageFooter.vue";
import UserMessageImage from "@/components/chat/UserMessageImage.vue";
import { codeLanguageForPath } from "@/services/chat/codeLanguage";
import { formatMentionPath, normalizeMentionPath } from "@/services/chat/composerSegments";
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
  canResend: boolean;
  busy: boolean;
}>();

const emit = defineEmits<{
  preview: [source: string];
  resend: [text: string];
  rewind: [];
}>();

const settingStore = useSettingStore();
const pluginsStore = usePluginsStore();
const editing = ref(false);
const draft = ref("");
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
const needsFold = computed(() => shouldFoldUserMessage(plainText.value));
const collapsed = computed(() => needsFold.value && !expanded.value && !editing.value);
const canEdit = computed(
  () =>
    props.canResend &&
    !props.busy &&
    !isSoftInjectContent(props.message.content) &&
    props.message.injected !== true,
);
const canSend = computed(() => editing.value && draft.value.trim().length > 0);

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
  composerRef.value?.setSelection(len, len);
}

function cancelEdit() {
  editing.value = false;
  draft.value = content.value.message;
}

function commitEdit() {
  if (!editing.value) return;
  emit("resend", draft.value);
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
    }"
    :title="canEdit && !editing ? tr(settingStore.language, 'editAndResend') : undefined"
    @click="startEdit"
    @focusout="scheduleBlurCheck"
  >
    <button
      v-if="canResend && !editing"
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
        @keydown="onComposerKeydown"
        @click.stop
      />
      <span v-else-if="content.message" class="user-message-text">
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
      v-if="needsFold && !editing"
      type="button"
      class="user-bubble-toggle"
      data-tauri-drag-region="false"
      @click.stop="expanded = !expanded"
    >
      {{ tr(settingStore.language, expanded ? "collapse" : "expand") }}
    </button>
    <UserMessageFooter
      v-if="editing"
      :session-id="sessionId"
      :can-send="canSend"
      :busy="busy"
      @attach="void attachFiles()"
      @send="commitEdit"
    />
  </div>
</template>

<style scoped>
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
