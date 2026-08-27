<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { codeLanguageForPath } from "@/services/chat/codeLanguage";
import {
  formatResourceMention,
  isDirMention,
  normalizeMentionPath,
} from "@/services/chat/composerSegments";
import {
  getComposerSelectionOffsets,
  renderComposerEditable,
  serializeComposerEditable,
  setComposerSelectionOffsets,
  type ComposerTokenMeta,
  type ResolveComposerTokenMeta,
} from "@/services/chat/composerEditableDom";
import type { InlineTokenPart } from "@/services/chat/inlineTokenMarks";
import {
  mcpMentionIconUrl,
  mcpMentionLabel,
  skillMentionIconUrl,
  skillMentionLabel,
} from "@/services/chat/hashMentionDisplay";
import type { McpServerConfig } from "@/types/setting";
import "@/services/chat/composerEditable.css";

const props = withDefaults(
  defineProps<{
    modelValue: string;
    placeholder?: string;
    readonly?: boolean;
    multiline?: boolean;
    empty?: boolean;
    ariaExpanded?: boolean;
    mcpServers?: readonly Pick<McpServerConfig, "id" | "title" | "qualifiedName" | "iconUrl">[];
    skills?: readonly {
      name: string;
      title?: string;
      qualifiedName?: string | null;
      iconUrl?: string | null;
    }[];
    /** Workspace-relative paths used to disambiguate file labels. */
    fileCatalog?: readonly string[];
  }>(),
  {
    placeholder: "",
    readonly: false,
    multiline: false,
    empty: false,
    ariaExpanded: false,
    mcpServers: () => [],
    skills: () => [],
    fileCatalog: () => [],
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: string];
  caretChange: [caret: number];
  input: [];
  keydown: [event: KeyboardEvent];
  paste: [event: ClipboardEvent];
  focus: [event: FocusEvent];
  blur: [event: FocusEvent];
}>();

const rootRef = ref<HTMLDivElement | null>(null);
const isComposing = ref(false);
const isEmpty = computed(() => !(props.modelValue ?? "").length);
/** Last text emitted from DOM — avoids prop-watch re-serializing on every key. */
let lastEmittedText = props.modelValue ?? "";

const resolveMeta: ResolveComposerTokenMeta = (part) => resolveTokenMeta(part);

function resolveTokenMeta(part: Exclude<InlineTokenPart, { kind: "text" }>): ComposerTokenMeta {
  if (part.kind === "mention") {
    const path = normalizeMentionPath(part.path);
    const iconUrl = part.isDir ? null : (codeLanguageForPath(path).icon ?? null);
    return {
      kind: "mention",
      token: part.raw,
      label: `@${part.name}`,
      title: path,
      iconUrl,
      fallback: part.isDir || isDirMention(part.path) ? "folder" : "file",
      className: part.isDir ? "ce-token-file is-dir" : "ce-token-file",
    };
  }
  if (part.kind === "skill") {
    return {
      kind: "skill",
      token: part.raw || formatResourceMention("skill", part.id),
      label: skillMentionLabel(part.id, props.skills),
      title: part.id,
      iconUrl: skillMentionIconUrl(part.id, props.skills),
      fallback: "zap",
      className: "ce-token-skill",
    };
  }
  return {
    kind: "mcp",
    token: part.raw || formatResourceMention("mcp", part.id),
    label: mcpMentionLabel(part.id, props.mcpServers),
    title: part.id,
    iconUrl: mcpMentionIconUrl(part.id, props.mcpServers),
    fallback: "bot",
    className: "ce-token-mcp",
  };
}

function syncFromProp(text: string, caret?: { start: number; end: number }) {
  const root = rootRef.value;
  if (!root) return;
  renderComposerEditable(root, text, resolveMeta);
  if (caret) setComposerSelectionOffsets(root, caret.start, caret.end);
}

function emitTextFromDom() {
  const root = rootRef.value;
  if (!root || isComposing.value) return;
  // Serialize only — do not re-parse typed `@query` into marks (picker inserts marks).
  const text = serializeComposerEditable(root);
  lastEmittedText = text;
  if (text !== props.modelValue) {
    emit("update:modelValue", text);
  }
  const { start } = getComposerSelectionOffsets(root);
  emit("caretChange", start);
  emit("input");
}

function onCompositionStart() {
  isComposing.value = true;
}

function onCompositionEnd() {
  isComposing.value = false;
  void nextTick(() => {
    emitTextFromDom();
  });
}

function onInput() {
  if (isComposing.value) return;
  emitTextFromDom();
}

function onKeyDown(event: KeyboardEvent) {
  emit("keydown", event);
}

function onPaste(event: ClipboardEvent) {
  // Always paste as plain text, then let normalize turn wire tokens into marks.
  event.preventDefault();
  const plain = event.clipboardData?.getData("text/plain") ?? "";
  if (!plain) {
    emit("paste", event);
    return;
  }
  const normalized = plain.replace(/\r\n|\r/g, "\n");
  const root = rootRef.value;
  if (!root) return;
  const { start, end } = getComposerSelectionOffsets(root);
  const current = serializeComposerEditable(root);
  const next = `${current.slice(0, start)}${normalized}${current.slice(end)}`;
  const caret = start + normalized.length;
  syncFromProp(next, { start: caret, end: caret });
  lastEmittedText = next;
  emit("update:modelValue", next);
  emit("caretChange", caret);
  emit("input");
  emit("paste", event);
}

function onSelectOrCaret() {
  const root = rootRef.value;
  if (!root) return;
  emit("caretChange", getComposerSelectionOffsets(root).start);
}

function onClick(event: MouseEvent) {
  const root = rootRef.value;
  if (!root) return;
  // Clicking empty padding should still place caret at end.
  if (event.target === root && isEmpty.value) {
    setComposerSelectionOffsets(root, 0);
  }
  onSelectOrCaret();
}

function focus(options?: FocusOptions) {
  rootRef.value?.focus(options);
}

function blur() {
  rootRef.value?.blur();
}

function getSelection() {
  const root = rootRef.value;
  if (!root) {
    const len = props.modelValue.length;
    return { start: len, end: len };
  }
  return getComposerSelectionOffsets(root);
}

function setSelection(start: number, end: number = start) {
  const root = rootRef.value;
  if (!root) return;
  root.focus({ preventScroll: true });
  setComposerSelectionOffsets(root, start, end);
  emit("caretChange", start);
}

function setText(text: string, caret?: number) {
  const pos = caret ?? text.length;
  syncFromProp(text, { start: pos, end: pos });
  lastEmittedText = text;
  if (text !== props.modelValue) emit("update:modelValue", text);
  emit("caretChange", pos);
  emit("input");
}

function replaceRange(start: number, end: number, insert: string, caret?: number) {
  const current = props.modelValue;
  const next = `${current.slice(0, start)}${insert}${current.slice(end)}`;
  const pos = caret ?? start + insert.length;
  setText(next, pos);
}

function insertAtCaret(text: string) {
  const { start, end } = getSelection();
  replaceRange(start, end, text);
}

function resize() {
  // Height auto-grow is owned by ChatInputBar.resizeComposerInput.
}

watch(
  () => props.modelValue,
  (value) => {
    if (isComposing.value) return;
    const root = rootRef.value;
    if (!root) return;
    // Typing path: we just emitted this exact string — don't walk the DOM again.
    if (value === lastEmittedText) return;
    if (serializeComposerEditable(root) === value) {
      lastEmittedText = value;
      return;
    }
    const caret = getComposerSelectionOffsets(root);
    syncFromProp(value, caret);
    lastEmittedText = value;
    emit("input");
  },
);

watch(
  () => [props.mcpServers?.length ?? 0, props.skills?.length ?? 0, props.fileCatalog?.length ?? 0],
  () => {
    if (isComposing.value) return;
    const root = rootRef.value;
    if (!root) return;
    const caret = getComposerSelectionOffsets(root);
    syncFromProp(props.modelValue, caret);
    lastEmittedText = props.modelValue ?? "";
  },
);

onMounted(() => {
  lastEmittedText = props.modelValue ?? "";
  syncFromProp(props.modelValue);
});

onBeforeUnmount(() => {
  // no-op
});

defineExpose({
  focus,
  blur,
  getSelection,
  setSelection,
  setText,
  replaceRange,
  insertAtCaret,
  resize,
  get el() {
    return rootRef.value;
  },
  /** Compatibility helpers used by ChatInputBar during migration. */
  get selectionStart() {
    return getSelection().start;
  },
  get selectionEnd() {
    return getSelection().end;
  },
  setSelectionRange(start: number, end: number) {
    setSelection(start, end);
  },
});
</script>

<template>
  <div
    ref="rootRef"
    class="composer-editable chat-input composer-textarea peek-scrollbar"
    :class="{
      'is-multiline': multiline,
      'is-empty': empty || isEmpty,
      'is-composing': isComposing,
    }"
    data-tauri-drag-region="false"
    role="textbox"
    aria-multiline="true"
    aria-autocomplete="list"
    :aria-expanded="ariaExpanded"
    :aria-placeholder="placeholder"
    :data-placeholder="placeholder"
    :spellcheck="false"
    :contenteditable="readonly ? 'false' : 'true'"
    @input="onInput"
    @keydown="onKeyDown"
    @keyup="onSelectOrCaret"
    @click="onClick"
    @mouseup="onSelectOrCaret"
    @compositionstart="onCompositionStart"
    @compositionend="onCompositionEnd"
    @paste="onPaste"
    @focus="emit('focus', $event)"
    @blur="emit('blur', $event)"
  />
</template>
