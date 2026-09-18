<template>
  <div ref="root" class="split-diff" :class="{ wrap: wrapLines }">
    <template v-for="(row, index) in rows" :key="index">
      <div v-if="isSkip(row)" class="split-skip">
        {{ skipLabel(row) }}
      </div>
      <div v-else class="split-row" :class="{ 'is-change': isChange(row) }">
        <div class="split-cell" :class="cellKind(row, 'left')">
          <span class="split-gutter">{{ lineNo(row.left) }}</span>
          <pre
            class="split-code"
          ><span v-for="(part, partIndex) in parts(row, 'left')" :key="partIndex" :class="{ mark: part.mark }">{{ part.text }}</span></pre>
        </div>
        <div class="split-cell" :class="cellKind(row, 'right')">
          <span class="split-gutter">{{ lineNo(row.right) }}</span>
          <pre
            class="split-code"
          ><span v-for="(part, partIndex) in parts(row, 'right')" :key="partIndex" :class="{ mark: part.mark }">{{ part.text }}</span></pre>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import { storeToRefs } from "pinia";
import { inlineRangesForRow, type CodeDiffRow } from "@/services/chat/codeDiff";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";

const props = defineProps<{
  rows: CodeDiffRow[];
  wrapLines: boolean;
}>();

const root = ref<HTMLElement | null>(null);
const { language } = storeToRefs(useSettingStore());
let changeIndex = -1;

function isSkip(row: CodeDiffRow) {
  return row.left?.kind === "skip" || row.right?.kind === "skip";
}

function isChange(row: CodeDiffRow) {
  return row.left?.kind === "deletion" || row.right?.kind === "addition";
}

function skipLabel(row: CodeDiffRow) {
  return tr(language.value, "diffUnmodifiedLines", {
    count: row.left?.skipped ?? row.right?.skipped ?? 0,
  });
}

function cellKind(row: CodeDiffRow, side: "left" | "right") {
  const line = row[side];
  if (!line) return "empty";
  if (line.kind === "context" || line.kind === "addition" || line.kind === "deletion") {
    return line.kind;
  }
  return "empty";
}

function lineNo(line: CodeDiffRow["left"]) {
  if (!line?.lineNumber || line.kind === "skip") return "";
  return String(line.lineNumber);
}

function parts(row: CodeDiffRow, side: "left" | "right") {
  const line = row[side];
  if (!line) return [{ text: " ", mark: false }];
  const range = inlineRangesForRow(row)[side];
  const text = line.text.length ? line.text : " ";
  if (!range) return [{ text, mark: false }];
  const segments = [
    { text: text.slice(0, range.from), mark: false },
    { text: text.slice(range.from, range.to), mark: true },
    { text: text.slice(range.to), mark: false },
  ].filter((part) => part.text.length);
  return segments.length ? segments : [{ text: " ", mark: false }];
}

/** Scroll the next added/removed hunk into view. */
function nextChange() {
  const rows = [...(root.value?.querySelectorAll(".split-row") ?? [])];
  const starts = rows.filter(
    (element, index) =>
      element.classList.contains("is-change") && !rows[index - 1]?.classList.contains("is-change"),
  );
  if (!starts.length) return;
  changeIndex = (changeIndex + 1) % starts.length;
  starts[changeIndex]?.scrollIntoView({ block: "center", inline: "nearest" });
}

watch(
  () => props.rows,
  () => {
    changeIndex = -1;
  },
);

defineExpose({ nextChange });
</script>

<style scoped>
.split-diff {
  min-width: 0;
  display: flex;
  flex-direction: column;
  color: var(--peek-code-fg, var(--peek-text));
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 20px;
  tab-size: 4;
}
.split-skip {
  padding: 3px 12px;
  border-top: 1px solid color-mix(in srgb, var(--peek-text) 8%, transparent);
  border-bottom: 1px solid color-mix(in srgb, var(--peek-text) 8%, transparent);
  background: color-mix(in srgb, var(--peek-text) 5.5%, transparent);
  color: var(--peek-muted);
  font-size: 11px;
}
.split-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  min-height: 20px;
}
.split-cell {
  display: grid;
  grid-template-columns: 44px minmax(0, 1fr);
  min-width: 0;
}
.split-cell:last-child {
  border-left: 1px solid color-mix(in srgb, var(--peek-text) 10%, transparent);
}
.split-gutter {
  box-sizing: border-box;
  min-width: 44px;
  padding: 0 8px 0 4px;
  border-right: 1px solid color-mix(in srgb, var(--peek-text) 6%, transparent);
  color: var(--peek-code-muted, var(--peek-faint));
  font-variant-numeric: tabular-nums;
  text-align: right;
  user-select: none;
}
.split-code {
  margin: 0;
  min-width: 0;
  padding: 0 12px;
  overflow-x: auto;
  font: inherit;
  white-space: pre;
}
.split-diff.wrap .split-code {
  overflow-x: hidden;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.split-cell.addition {
  background: color-mix(in srgb, #2ea043 16%, transparent);
}
.split-cell.deletion {
  background: color-mix(in srgb, #f85149 15%, transparent);
}
.split-cell.addition .split-code {
  box-shadow: inset 3px 0 0 #2ea043;
}
.split-cell.deletion .split-code {
  box-shadow: inset 3px 0 0 #f85149;
}
.split-cell.empty {
  background-color: color-mix(in srgb, var(--peek-text) 2.2%, transparent);
  background-image: repeating-linear-gradient(
    -45deg,
    color-mix(in srgb, var(--peek-text) 8%, transparent) 0,
    color-mix(in srgb, var(--peek-text) 8%, transparent) 1px,
    transparent 1px,
    transparent 6px
  );
}
.split-cell.empty .split-gutter,
.split-cell.empty .split-code {
  background: transparent;
  box-shadow: none;
}
.split-code .mark {
  border-radius: 2px;
}
.split-cell.addition .mark {
  background: color-mix(in srgb, #2ea043 38%, transparent);
}
.split-cell.deletion .mark {
  background: color-mix(in srgb, #f85149 36%, transparent);
}
</style>
