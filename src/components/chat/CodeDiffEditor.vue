<template>
  <div class="code-diff-editor" :class="`is-${viewMode}`">
    <div v-if="loading" class="code-diff-loading" />
    <div v-else-if="error" class="code-diff-error">{{ error }}</div>
    <SplitDiffView
      v-else-if="document && viewMode === 'split'"
      ref="splitViewRef"
      :rows="document.rows"
      :wrap-lines="wrapLines"
    />
    <div v-else-if="document" ref="unifiedHost" class="editor-pane unified-editor" />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { Compartment, EditorState, RangeSetBuilder, type Extension } from "@codemirror/state";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { Decoration, EditorView, keymap, lineNumbers } from "@codemirror/view";
import { defaultKeymap } from "@codemirror/commands";
import { tags } from "@lezer/highlight";
import SplitDiffView from "@/components/chat/SplitDiffView.vue";
import {
  buildCodeDiff,
  inlineRangesForRow,
  type CodeDiffDocument,
  type CodeDiffInlineRange,
  type CodeDiffLine,
  type CodeDiffLineKind,
} from "@/services/chat/codeDiff";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";

type DiffViewMode = "split" | "unified";

const props = defineProps<{
  oldText?: string | null;
  newText?: string | null;
  unifiedDiff: string;
  language: string;
  viewMode: DiffViewMode;
  wrapLines: boolean;
}>();

const unifiedHost = ref<HTMLElement | null>(null);
const splitViewRef = ref<{ nextChange: () => void } | null>(null);
const document = ref<CodeDiffDocument | null>(null);
const loading = ref(false);
const error = ref("");
const settingStore = useSettingStore();
const wrapCompartment = new Compartment();
let unifiedView: EditorView | null = null;
let requestVersion = 0;
let editorVersion = 0;
let resizeObserver: ResizeObserver | null = null;

const requestKey = computed(() =>
  JSON.stringify({
    oldText: props.oldText,
    newText: props.newText,
    unifiedDiff: props.unifiedDiff,
  }),
);

watch(requestKey, loadDocument, { immediate: true });
watch(
  () => [loading.value, document.value, props.viewMode, props.language] as const,
  async ([isLoading, currentDocument]) => {
    if (isLoading || !currentDocument) {
      destroyEditors();
      return;
    }
    await nextTick();
    await rebuildEditors();
  },
  { flush: "post" },
);
watch(() => props.wrapLines, updateWrap);

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  resizeObserver = null;
  destroyEditors();
});

async function loadDocument() {
  const version = ++requestVersion;
  loading.value = true;
  error.value = "";
  try {
    const next = await buildCodeDiff({
      oldText: props.oldText,
      newText: props.newText,
      unifiedDiff: props.unifiedDiff,
    });
    if (version !== requestVersion) return;
    document.value = next;
  } catch (cause) {
    if (version === requestVersion) {
      error.value = cause instanceof Error ? cause.message : String(cause);
      document.value = null;
      destroyEditors();
    }
  } finally {
    if (version === requestVersion) loading.value = false;
  }
}

async function rebuildEditors() {
  const version = ++editorVersion;
  destroyEditors();
  if (!document.value || props.viewMode === "split") return;
  const language = await languageExtension(props.language);
  if (version !== editorVersion || !document.value || !unifiedHost.value) return;
  unifiedView = createEditor(unifiedHost.value, unifiedLines(), language);
  observeEditorResize([unifiedView]);
}

function observeEditorResize(views: EditorView[]) {
  resizeObserver?.disconnect();
  const hosts = views
    .map((view) => view.dom.parentElement)
    .filter((host): host is HTMLElement => host instanceof HTMLElement);
  if (!hosts.length) return;
  resizeObserver = new ResizeObserver(() => {
    for (const view of views) view.requestMeasure();
  });
  for (const host of hosts) resizeObserver.observe(host);
}

function destroyEditors() {
  unifiedView?.destroy();
  unifiedView = null;
}

function updateWrap(wrapLines: boolean) {
  if (!unifiedView) return;
  unifiedView.dispatch({
    effects: wrapCompartment.reconfigure(wrapLines ? EditorView.lineWrapping : []),
  });
}

function createEditor(parent: HTMLElement, lines: DisplayLine[], language: Extension) {
  const display = lines.map((line) =>
    line.kind === "skip"
      ? {
          ...line,
          text: tr(settingStore.language, "diffUnmodifiedLines", { count: line.skipped ?? 0 }),
        }
      : line,
  );
  const lineNumbersByDisplayLine = display.map((line) =>
    line.kind === "skip" ? "⋯" : String(line.lineNumber ?? ""),
  );
  const state = EditorState.create({
    doc: display.map((line) => line.text).join("\n"),
    extensions: [
      keymap.of(defaultKeymap),
      EditorState.readOnly.of(true),
      EditorView.editable.of(false),
      lineNumbers({ formatNumber: (line) => lineNumbersByDisplayLine[line - 1] ?? "" }),
      wrapCompartment.of(props.wrapLines ? EditorView.lineWrapping : []),
      language,
      syntaxHighlighting(diffHighlightStyle, { fallback: true }),
      diffTheme,
      lineClasses(display),
      inlineMarks(display),
    ],
  });
  const view = new EditorView({ state, parent });
  view.scrollDOM.classList.add("peek-scrollbar");
  queueMicrotask(() => view.requestMeasure());
  return view;
}

type DisplayLine = {
  text: string;
  lineNumber?: number;
  kind?: CodeDiffLineKind;
  skipped?: number;
  inline?: CodeDiffInlineRange;
};

function unifiedLines(): DisplayLine[] {
  return document.value!.rows.flatMap((row) => {
    const ranges = inlineRangesForRow(row);
    if (row.left?.kind === "deletion") {
      const deleted = { ...displayLine(row.left), inline: ranges.left };
      if (!row.right) return [deleted];
      return [deleted, { ...displayLine(row.right), inline: ranges.right }];
    }
    if (row.right?.kind === "addition")
      return [{ ...displayLine(row.right), inline: ranges.right }];
    return [displayLine(row.left ?? row.right)];
  });
}

function displayLine(line: CodeDiffLine | null | undefined): DisplayLine {
  return line
    ? { text: line.text, lineNumber: line.lineNumber, kind: line.kind, skipped: line.skipped }
    : { text: "" };
}

function lineClasses(lines: DisplayLine[]): Extension {
  const builder = new RangeSetBuilder<Decoration>();
  let position = 0;
  for (const line of lines) {
    if (line.kind)
      builder.add(
        position,
        position,
        Decoration.line({ attributes: { class: `diff-${line.kind}` } }),
      );
    position += line.text.length + 1;
  }
  return EditorView.decorations.of(builder.finish());
}

function inlineMarks(lines: DisplayLine[]): Extension {
  const builder = new RangeSetBuilder<Decoration>();
  let position = 0;
  for (const line of lines) {
    if (line.inline && line.inline.to > line.inline.from) {
      builder.add(
        position + line.inline.from,
        position + line.inline.to,
        Decoration.mark({ class: "diff-inline" }),
      );
    }
    position += line.text.length + 1;
  }
  return EditorView.decorations.of(builder.finish());
}

let changeIndex = -1;
function nextChange() {
  if (props.viewMode === "split") {
    splitViewRef.value?.nextChange();
    return;
  }
  if (!document.value || !unifiedView) return;
  const lines = unifiedLines();
  const changed = (index: number) => ["addition", "deletion"].includes(lines[index]?.kind ?? "");
  const starts = lines.flatMap((_, index) =>
    changed(index) && !changed(index - 1) ? [index + 1] : [],
  );
  if (!starts.length) return;
  changeIndex = (changeIndex + 1) % starts.length;
  unifiedView.dispatch({
    effects: EditorView.scrollIntoView(unifiedView.state.doc.line(starts[changeIndex]).from, {
      y: "center",
    }),
  });
}
watch(requestKey, () => {
  changeIndex = -1;
});
defineExpose({ nextChange });

/** Language packs are loaded on demand so Diff does not pull every grammar into one chunk. */
async function languageExtension(language: string): Promise<Extension> {
  switch (language) {
    case "javascript": {
      const { javascript } = await import("@codemirror/lang-javascript");
      return javascript({ jsx: true });
    }
    case "typescript": {
      const { javascript } = await import("@codemirror/lang-javascript");
      return javascript({ jsx: true, typescript: true });
    }
    case "json": {
      const { json } = await import("@codemirror/lang-json");
      return json();
    }
    case "xml": {
      const { xml } = await import("@codemirror/lang-xml");
      return xml();
    }
    case "css":
    case "scss": {
      const { css } = await import("@codemirror/lang-css");
      return css();
    }
    case "rust": {
      const { rust } = await import("@codemirror/lang-rust");
      return rust();
    }
    case "python": {
      const { python } = await import("@codemirror/lang-python");
      return python();
    }
    case "yaml": {
      const { yaml } = await import("@codemirror/lang-yaml");
      return yaml();
    }
    case "markdown": {
      const { markdown } = await import("@codemirror/lang-markdown");
      return markdown();
    }
    case "sql": {
      const { sql } = await import("@codemirror/lang-sql");
      return sql();
    }
    case "go": {
      const { go } = await import("@codemirror/lang-go");
      return go();
    }
    case "java": {
      const { java } = await import("@codemirror/lang-java");
      return java();
    }
    case "cpp":
    case "c": {
      const { cpp } = await import("@codemirror/lang-cpp");
      return cpp();
    }
    case "php": {
      const { php } = await import("@codemirror/lang-php");
      return php();
    }
    case "html": {
      const { html } = await import("@codemirror/lang-html");
      return html();
    }
    default:
      return [];
  }
}

const diffHighlightStyle = HighlightStyle.define([
  { tag: tags.comment, color: "var(--peek-syntax-comment)", fontStyle: "italic" },
  { tag: tags.lineComment, color: "var(--peek-syntax-comment)", fontStyle: "italic" },
  { tag: tags.blockComment, color: "var(--peek-syntax-comment)", fontStyle: "italic" },
  {
    tag: [tags.keyword, tags.controlKeyword, tags.moduleKeyword, tags.definitionKeyword],
    color: "var(--peek-syntax-keyword)",
  },
  {
    tag: [tags.operator, tags.operatorKeyword, tags.compareOperator, tags.logicOperator],
    color: "var(--peek-syntax-operator)",
  },
  {
    tag: [tags.string, tags.special(tags.string), tags.character, tags.regexp],
    color: "var(--peek-syntax-string)",
  },
  { tag: [tags.number, tags.bool, tags.null, tags.atom], color: "var(--peek-syntax-number)" },
  {
    tag: [tags.function(tags.variableName), tags.function(tags.propertyName), tags.labelName],
    color: "var(--peek-syntax-function)",
  },
  {
    tag: [tags.typeName, tags.className, tags.namespace, tags.self],
    color: "var(--peek-syntax-type)",
  },
  { tag: [tags.propertyName, tags.attributeName], color: "var(--peek-syntax-property)" },
  {
    tag: [tags.variableName, tags.local(tags.variableName), tags.definition(tags.variableName)],
    color: "var(--peek-syntax-variable)",
  },
  {
    tag: [
      tags.punctuation,
      tags.separator,
      tags.bracket,
      tags.angleBracket,
      tags.paren,
      tags.squareBracket,
    ],
    color: "var(--peek-syntax-punctuation)",
  },
  { tag: tags.meta, color: "var(--peek-syntax-comment)" },
  { tag: tags.invalid, color: "var(--peek-danger)" },
]);

const diffTheme = EditorView.theme({
  "&": {
    height: "100%",
    maxHeight: "100%",
    backgroundColor: "transparent",
    color: "var(--peek-code-fg, var(--peek-text))",
  },
  ".cm-editor": {
    height: "100%",
    maxHeight: "100%",
  },
  ".cm-scroller": {
    overflow: "auto",
    overscrollBehavior: "contain",
    minHeight: 0,
    height: "100%",
    maxHeight: "100%",
    fontFamily: "var(--font-mono)",
    fontSize: "12px",
    lineHeight: "1.65",
  },
  ".cm-content": {
    padding: "0 0 18px",
    minHeight: "100%",
    caretColor: "transparent",
    color: "var(--peek-code-fg, var(--peek-text))",
  },
  ".cm-line": { padding: "0 12px", minHeight: "20px" },
  ".cm-gutters": {
    border: "0",
    backgroundColor: "color-mix(in srgb, var(--peek-text) 1.8%, transparent)",
    color: "var(--peek-code-muted, var(--peek-faint))",
  },
  ".cm-gutterElement": { padding: "0 8px 0 4px", minWidth: "36px" },
  ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "transparent" },
  ".cm-selectionBackground, ::selection": {
    backgroundColor: "var(--peek-code-selection, var(--peek-list-active)) !important",
  },
  ".cm-line.diff-addition": {
    backgroundColor: "color-mix(in srgb, #2ea043 16%, transparent)",
    boxShadow: "inset 3px 0 0 #2ea043",
  },
  ".cm-line.diff-deletion": {
    backgroundColor: "color-mix(in srgb, #f85149 15%, transparent)",
    boxShadow: "inset 3px 0 0 #f85149",
  },
  ".cm-line.diff-skip": {
    backgroundColor: "color-mix(in srgb, var(--peek-text) 4%, transparent)",
    color: "var(--peek-muted)",
    fontStyle: "italic",
    boxShadow: "inset 0 1px 0 color-mix(in srgb, var(--peek-text) 8%, transparent)",
  },
  ".diff-inline": {
    borderRadius: "2px",
  },
  ".cm-line.diff-addition .diff-inline": {
    backgroundColor: "color-mix(in srgb, #2ea043 38%, transparent)",
  },
  ".cm-line.diff-deletion .diff-inline": {
    backgroundColor: "color-mix(in srgb, #f85149 36%, transparent)",
  },
});
</script>

<style scoped>
.code-diff-editor {
  flex: 1;
  min-height: 0;
  position: relative;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: transparent;
}
.code-diff-editor.is-split {
  flex: none;
  min-height: 48px;
  overflow: visible;
}
.editor-pane {
  min-width: 0;
  min-height: 0;
  height: 100%;
  overflow: hidden;
  border-radius: 5px;
  background: color-mix(in srgb, var(--peek-text) 1.2%, transparent);
}
.editor-pane.unified-editor {
  flex: 1;
  min-height: 0;
  margin: 0 6px 6px;
}
.code-diff-loading {
  position: absolute;
  inset: 0;
  background: transparent;
}
.code-diff-error {
  padding: 16px;
  color: var(--peek-muted);
  font: 11px/1.5 var(--font-mono);
  white-space: pre-wrap;
}
</style>
