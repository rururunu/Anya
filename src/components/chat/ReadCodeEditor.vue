<template><div ref="host" class="read-code-editor" /></template>
<script setup lang="ts">
import { nextTick, onMounted, onBeforeUnmount, ref, watch } from "vue";
import { EditorState, RangeSetBuilder } from "@codemirror/state";
import { Decoration, EditorView, lineNumbers } from "@codemirror/view";
import { defaultHighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { codeLanguage } from "@/services/chat/codeLanguage";
const props = withDefaults(
  defineProps<{
    content: string;
    path: string;
    firstLine?: number;
    startLine?: number;
    endLine?: number;
  }>(),
  { firstLine: 1 },
);
const host = ref<HTMLElement | null>(null);
let view: EditorView | null = null;
let revision = 0;
async function rebuild() {
  const current = ++revision;
  const language = await codeLanguage(props.path);
  await nextTick();
  if (current !== revision || !host.value) return;
  view?.destroy();
  const document = EditorState.create({ doc: props.content });
  const selected = new RangeSetBuilder<Decoration>();
  const from = Math.max(1, (props.startLine ?? props.firstLine) - props.firstLine + 1);
  const to = Math.min(
    document.doc.lines,
    (props.endLine ?? props.firstLine - 1) - props.firstLine + 1,
  );
  for (let line = from; line <= to; line++)
    selected.add(
      document.doc.line(line).from,
      document.doc.line(line).from,
      Decoration.line({ class: "agent-read-line" }),
    );
  view = new EditorView({
    parent: host.value,
    state: EditorState.create({
      doc: props.content,
      extensions: [
        EditorState.readOnly.of(true),
        EditorView.editable.of(false),
        lineNumbers({ formatNumber: (line) => String(line + props.firstLine - 1) }),
        language,
        syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
        EditorView.decorations.of(selected.finish()),
        EditorView.theme({
          "&": {
            height: "100%",
            color: "var(--peek-text)",
            backgroundColor: "transparent",
            fontSize: "12px",
          },
          ".cm-scroller": {
            overflow: "auto",
            fontFamily: "ui-monospace, Consolas, monospace",
            lineHeight: "1.7",
          },
          ".cm-gutters": {
            backgroundColor: "transparent",
            color: "var(--peek-muted)",
            border: "none",
          },
          ".cm-content": { padding: "8px 0" },
          ".cm-line": { padding: "0 12px" },
          ".agent-read-line": {
            backgroundColor: "color-mix(in srgb, var(--peek-accent) 10%, transparent)",
            boxShadow: "inset 2px 0 var(--peek-accent)",
          },
          "&.cm-focused": { outline: "none" },
        }),
      ],
    }),
  });
  view.scrollDOM.classList.add("peek-scrollbar");
  if (props.startLine && from <= document.doc.lines)
    view.dispatch({
      effects: EditorView.scrollIntoView(document.doc.line(from).from, { y: "start" }),
    });
}
onMounted(() => void rebuild());
watch(
  () => [props.content, props.path, props.firstLine, props.startLine, props.endLine],
  () => void rebuild(),
);
onBeforeUnmount(() => {
  revision++;
  view?.destroy();
});
</script>
<style scoped>
.read-code-editor {
  height: 100%;
  min-height: 0;
  overflow: hidden;
}
</style>
