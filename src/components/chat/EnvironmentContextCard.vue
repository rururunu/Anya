<template>
  <section class="environment-context" aria-label="Environment context">
    <header class="context-header">
      <div class="context-heading">
        <Code2 :size="17" :stroke-width="1.8" aria-hidden="true" />
        <span>Environment Context</span>
      </div>
      <span class="context-source" :class="{ available: Boolean(ide) }">
        <span class="source-dot" aria-hidden="true"></span>
        {{ ideName }}
      </span>
    </header>

    <div class="context-summary">
      <div class="summary-item">
        <span class="summary-label">Language</span>
        <span class="summary-value">{{ language }}</span>
      </div>
      <div class="summary-item">
        <span class="summary-label">Position</span>
        <span class="summary-value">{{ cursor }}</span>
      </div>
      <div class="summary-item">
        <span class="summary-label">Git</span>
        <span class="summary-value" :title="gitBranch">{{ gitBranch }}</span>
      </div>
    </div>

    <div v-if="office" class="context-paths office-block">
      <div class="path-row">
        <FileText :size="15" :stroke-width="1.7" aria-hidden="true" />
        <div class="path-content">
          <span class="path-label">Office</span>
          <span class="path-primary">{{ officeAppLabel }}</span>
          <span class="path-secondary" :title="officeDocument">{{ officeDocument }}</span>
        </div>
      </div>
      <div v-if="officeSelectionPreview" class="path-row subdued">
        <Highlighter :size="15" :stroke-width="1.7" aria-hidden="true" />
        <div class="path-content office-selection">
          <span class="path-label">Selection preview</span>
          <span class="path-secondary office-preview" :title="officeSelectionPreview">
            {{ officeSelectionPreview }}
          </span>
        </div>
      </div>
    </div>

    <div class="context-paths">
      <div class="path-row">
        <FolderGit2 :size="15" :stroke-width="1.7" aria-hidden="true" />
        <div class="path-content">
          <span class="path-label">Workspace</span>
          <span class="path-primary">{{ workspaceName }}</span>
          <code class="path-secondary" :title="workspacePath">{{ workspacePath }}</code>
        </div>
      </div>
      <div class="path-row">
        <FileCode2 :size="15" :stroke-width="1.7" aria-hidden="true" />
        <div class="path-content">
          <span class="path-label">Active file</span>
          <code class="path-primary" :title="activeFile">{{ activeFileName }}</code>
          <code class="path-secondary" :title="activeFile">{{ activeFile }}</code>
        </div>
      </div>
      <div class="path-row subdued">
        <AppWindow :size="15" :stroke-width="1.7" aria-hidden="true" />
        <div class="path-content">
          <span class="path-label">Window</span>
          <span class="path-secondary" :title="windowTitle">{{ windowTitle }}</span>
        </div>
      </div>
    </div>

    <div class="context-details">
      <details v-if="selection">
        <summary>
          <ChevronRight :size="14" :stroke-width="2" aria-hidden="true" />
          <span>Selection</span>
          <span class="detail-meta">{{ selectionMeta }}</span>
        </summary>
        <pre><code>{{ selection }}</code></pre>
      </details>

      <details v-if="gitStatus">
        <summary>
          <ChevronRight :size="14" :stroke-width="2" aria-hidden="true" />
          <span>Git status</span>
          <span class="detail-meta">{{ gitMeta }}</span>
        </summary>
        <pre><code>{{ gitStatus }}</code></pre>
      </details>

      <details v-if="shell">
        <summary>
          <ChevronRight :size="14" :stroke-width="2" aria-hidden="true" />
          <span>Last shell execution</span>
        </summary>
        <pre><code>{{ shell }}</code></pre>
      </details>

      <div
        v-if="!selection && !gitStatus && !shell && !officeSelectionPreview"
        class="empty-details"
      >
        No selection, Git changes, or shell result
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import {
  AppWindow,
  ChevronRight,
  Code2,
  FileCode2,
  FileText,
  FolderGit2,
  Highlighter,
} from "@lucide/vue";
import type { CapturedContext } from "@/types/chat";

const props = defineProps<{ context: CapturedContext }>();

const unavailable = "Unavailable";
const ide = computed(() => props.context.ideContext);
const ideName = computed(() => {
  const value = ide.value?.ide.trim();
  switch (value?.toLowerCase()) {
    case "vscode":
    case "visual studio code":
      return "VSCode";
    case "idea":
    case "intellij":
    case "intellij idea":
      return "IntelliJ IDEA";
    default:
      return value || unavailable;
  }
});
const language = computed(() => ide.value?.language?.trim() || unavailable);
const cursor = computed(() => {
  const value = ide.value?.cursor;
  return value ? `${value.line}:${value.column}` : unavailable;
});
const workspaceName = computed(() => props.context.workspace?.name || unavailable);
const workspacePath = computed(() => props.context.workspace?.root || unavailable);
const activeFile = computed(() => props.context.activeFile?.trim() || unavailable);
const activeFileName = computed(() => {
  if (activeFile.value === unavailable) return unavailable;
  return activeFile.value.split(/[\\/]/).pop() || activeFile.value;
});
const windowTitle = computed(() => props.context.activeWindow?.trim() || unavailable);
const selection = computed(() => ide.value?.selection?.trim() || "");
const selectionMeta = computed(() => {
  const lines = selection.value ? selection.value.split(/\r?\n/).length : 0;
  return `${lines} ${lines === 1 ? "line" : "lines"}`;
});
const gitStatus = computed(() => props.context.gitStatus?.trim() || "");
const gitLines = computed(() => (gitStatus.value ? gitStatus.value.split(/\r?\n/) : []));
const gitBranch = computed(() => gitLines.value[0]?.replace(/^##\s*/, "") || unavailable);
const gitMeta = computed(() => {
  const changes = Math.max(0, gitLines.value.length - 1);
  return changes ? `${changes} ${changes === 1 ? "change" : "changes"}` : "Clean";
});
const shell = computed(() => props.context.lastShellExecution?.trim() || "");
const office = computed(() => props.context.officeContext);
const officeAppLabel = computed(() => {
  switch (office.value?.app) {
    case "excel":
      return "Microsoft Excel";
    case "powerpoint":
      return "Microsoft PowerPoint";
    case "word":
      return "Microsoft Word";
    default:
      return office.value?.app || unavailable;
  }
});
const officeDocument = computed(() => {
  if (!office.value) return unavailable;
  const name = office.value.documentName?.trim();
  const path = office.value.documentPath?.trim();
  if (name && path) return `${name} — ${path}`;
  return name || path || unavailable;
});
const officeSelectionPreview = computed(() => {
  const text = office.value?.selectedText?.trim() || "";
  if (!text) return "";
  if (text.length <= 240) return text;
  return `${text.slice(0, 240)}…`;
});
</script>

<style scoped>
.environment-context {
  width: min(100%, 720px);
  overflow: hidden;
  border: 1px solid var(--peek-border);
  border-radius: 8px;
  background: color-mix(in srgb, var(--peek-surface) 94%, var(--peek-text) 6%);
}

.context-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 42px;
  padding: 0 13px;
  border-bottom: 1px solid var(--peek-border);
}

.context-heading,
.context-source {
  display: inline-flex;
  align-items: center;
}

.context-heading {
  gap: 7px;
  color: var(--peek-text);
  font-size: 13px;
  font-weight: 650;
}

.context-source {
  gap: 6px;
  min-width: 0;
  color: var(--peek-muted);
  font-size: 11px;
}

.source-dot {
  width: 7px;
  height: 7px;
  flex: 0 0 auto;
  border-radius: 50%;
  background: var(--peek-muted);
}

.context-source.available .source-dot {
  background: #36a269;
  box-shadow: 0 0 0 3px color-mix(in srgb, #36a269 16%, transparent);
}

.context-summary {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  border-bottom: 1px solid var(--peek-border);
}

.summary-item {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 2px;
  padding: 9px 12px;
}

.summary-item + .summary-item {
  border-left: 1px solid var(--peek-border);
}

.summary-label,
.path-label {
  color: var(--peek-muted);
  font-size: 10px;
  line-height: 1.3;
  text-transform: uppercase;
}

.summary-value {
  overflow: hidden;
  color: var(--peek-text);
  font-family: var(--font-mono, ui-monospace, monospace);
  font-size: 12px;
  line-height: 1.4;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.context-paths {
  padding: 5px 0;
}

.path-row {
  display: grid;
  grid-template-columns: 22px minmax(0, 1fr);
  align-items: start;
  padding: 7px 12px;
  color: var(--peek-muted);
}

.path-row > svg {
  margin-top: 3px;
}

.path-content {
  display: grid;
  min-width: 0;
  grid-template-columns: minmax(0, 140px) minmax(0, 1fr);
  column-gap: 8px;
  align-items: baseline;
}

.path-label {
  grid-column: 1 / -1;
  margin-bottom: 1px;
}

.path-primary,
.path-secondary {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.path-primary {
  color: var(--peek-text);
  font-size: 12px;
  font-weight: 600;
}

.path-secondary {
  color: var(--peek-muted);
  font-family: var(--font-mono, ui-monospace, monospace);
  font-size: 11px;
}

.path-row.subdued .path-secondary {
  grid-column: 1 / -1;
}

.office-block {
  border-bottom: 1px solid var(--peek-border);
}

.office-selection {
  grid-template-columns: minmax(0, 1fr);
}

.office-preview {
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
  white-space: pre-wrap;
}

.context-details {
  border-top: 1px solid var(--peek-border);
}

details + details {
  border-top: 1px solid var(--peek-border);
}

summary {
  display: flex;
  align-items: center;
  gap: 7px;
  min-height: 36px;
  padding: 0 12px;
  color: var(--peek-text);
  font-size: 12px;
  font-weight: 550;
  cursor: pointer;
  list-style: none;
  user-select: none;
}

summary::-webkit-details-marker {
  display: none;
}

summary svg {
  flex: 0 0 auto;
  color: var(--peek-muted);
  transition: transform 140ms ease;
}

details[open] summary svg {
  transform: rotate(90deg);
}

.detail-meta {
  margin-left: auto;
  color: var(--peek-muted);
  font-size: 11px;
  font-weight: 400;
}

pre {
  max-height: 280px;
  margin: 0;
  overflow: auto;
  border-top: 1px solid var(--peek-border);
  background: color-mix(in srgb, var(--peek-surface) 88%, #000 12%);
  padding: 11px 13px;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

pre code {
  color: var(--peek-text);
  font-family: var(--font-mono, ui-monospace, monospace);
  font-size: 11.5px;
  line-height: 1.55;
}

.empty-details {
  padding: 10px 12px;
  color: var(--peek-muted);
  font-size: 11px;
}

@media (max-width: 520px) {
  .context-summary {
    grid-template-columns: 1fr 1fr;
  }

  .summary-item:nth-child(3) {
    grid-column: 1 / -1;
    border-top: 1px solid var(--peek-border);
    border-left: 0;
  }

  .path-content {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
