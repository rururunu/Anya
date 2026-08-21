<template>
  <!-- Content is sanitized with DOMPurify before it reaches the template. -->
  <!-- eslint-disable-next-line vue/no-v-html -->
  <div ref="rootRef" class="markdown-body" v-html="html" @click="onMarkdownClick" />
</template>

<script setup lang="ts">
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Braces,
  Check,
  CircleX,
  Code2,
  Copy,
  Database,
  FileCode2,
  GitCompareArrows,
  Hash,
  SquareTerminal,
} from "@lucide/vue";
import DOMPurify from "dompurify";
import hljs from "highlight.js/lib/common";
import markedKatex from "marked-katex-extension";
import { marked } from "marked";
import { computed, h, onMounted, onUnmounted, onUpdated, ref, render, type Component } from "vue";
import "katex/dist/katex.min.css";
import { copyText } from "@/services/clipboard";
import { parseChartSpec } from "@/services/chat/chartSpec";
import { normalizeMarkdownInput } from "@/services/chat/markdownNormalize";
import { disposeChartBlocks, hydrateChartBlocks } from "./chartHydration";

const props = defineProps<{
  content: string;
}>();
const emit = defineEmits<{
  previewImage: [source: string];
}>();
const rootRef = ref<HTMLElement | null>(null);

const renderer = new marked.Renderer();

marked.use(
  markedKatex({
    nonStandard: true,
    throwOnError: false,
  }),
);

renderer.code = ({ text, lang }) => {
  const requestedLanguage = (lang ?? "").trim().split(/\s+/)[0]?.toLowerCase() || "";

  if (requestedLanguage === "chart") {
    const spec = parseChartSpec(text ?? "");
    if (spec) {
      return `<div class="chart-block" data-chart-spec="${escapeHtmlAttribute(JSON.stringify(spec))}"></div>\n`;
    }
  }

  const language = /^[a-z0-9_+-]+$/.test(requestedLanguage)
    ? requestedLanguage === "chart"
      ? "json"
      : requestedLanguage
    : "";
  const source = text ?? "";
  const highlighted =
    language && hljs.getLanguage(language)
      ? hljs.highlight(source, { language }).value
      : hljs.highlightAuto(source).value;
  const languageClass = language ? ` language-${language}` : "";
  const languageLabel = displayLanguage(language);

  return `<div class="code-block"><div class="code-block-toolbar"><span class="code-language"><span class="code-language-icon" data-code-language-icon="${language}"></span><span>${languageLabel}</span></span><button type="button" class="code-copy-button" data-code-copy aria-label="Copy code" title="Copy code"></button></div><pre><code class="hljs${languageClass}">${highlighted}</code></pre></div>\n`;
};

function iconForLanguage(language: string): Component {
  if (["diff", "patch"].includes(language)) {
    return GitCompareArrows;
  }
  if (
    ["bash", "shell", "sh", "zsh", "fish", "powershell", "ps1", "bat", "cmd"].includes(language)
  ) {
    return SquareTerminal;
  }
  if (["sql", "mysql", "pgsql", "postgresql", "graphql"].includes(language)) {
    return Database;
  }
  if (["json", "jsonc", "yaml", "yml", "toml", "xml"].includes(language)) {
    return Braces;
  }
  if (
    [
      "html",
      "css",
      "scss",
      "less",
      "javascript",
      "js",
      "typescript",
      "ts",
      "jsx",
      "tsx",
      "vue",
      "svelte",
    ].includes(language)
  ) {
    return Code2;
  }
  if (["csharp", "cs", "fsharp", "fs"].includes(language)) {
    return Hash;
  }
  return language ? FileCode2 : Braces;
}

function displayLanguage(language: string) {
  const labels: Record<string, string> = {
    bash: "Shell",
    sh: "Shell",
    ps1: "PowerShell",
    js: "JavaScript",
    jsx: "JavaScript JSX",
    ts: "TypeScript",
    tsx: "TypeScript JSX",
    cs: "C#",
    csharp: "C#",
    cpp: "C++",
    yml: "YAML",
    md: "Markdown",
    py: "Python",
    rs: "Rust",
  };
  return labels[language] || language || "Code";
}

function escapeHtmlAttribute(value: string) {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function renderButtonIcon(button: HTMLButtonElement, icon: Component) {
  render(h(icon, { size: 14, strokeWidth: 2, "aria-hidden": "true" }), button);
}

function hydrateCodeBlockIcons() {
  const root = rootRef.value;
  if (!root) return;
  root.querySelectorAll<HTMLElement>("[data-code-language-icon]").forEach((element) => {
    const language = element.dataset.codeLanguageIcon ?? "";
    render(
      h(iconForLanguage(language), { size: 13, strokeWidth: 2, "aria-hidden": "true" }),
      element,
    );
  });
  root.querySelectorAll<HTMLButtonElement>("[data-code-copy]").forEach((button) => {
    renderButtonIcon(button, Copy);
  });
}

function hydrateAll() {
  hydrateCodeBlockIcons();
  const root = rootRef.value;
  if (root) hydrateChartBlocks(root);
}

function unmountPortalHosts(root: HTMLElement) {
  root
    .querySelectorAll<HTMLElement>("[data-code-language-icon], [data-code-copy], [data-chart-spec]")
    .forEach((el) => {
      render(null, el);
    });
  disposeChartBlocks(root);
}

onMounted(hydrateAll);
onUpdated(hydrateAll);
onUnmounted(() => {
  const root = rootRef.value;
  if (root) unmountPortalHosts(root);
});

marked.setOptions({
  breaks: true,
  gfm: true,
  renderer,
});

const html = computed(() => {
  try {
    const raw = marked.parse(normalizeMarkdownInput(normalizeLegacyMath(props.content || "")), {
      async: false,
    }) as string;
    return DOMPurify.sanitize(raw, {
      ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto|tel|file|sms):|[^&#]*?:|data:image\/)/i,
      // Tables are part of the default allowlist, but some DOM environments
      // (e.g. certain test harnesses or webviews) resolve tag checks
      // differently. Declare the full table family explicitly so markdown
      // tables can never be silently stripped and shown as raw pipe text.
      ADD_TAGS: [
        "table",
        "thead",
        "tbody",
        "tfoot",
        "tr",
        "th",
        "td",
        "caption",
        "colgroup",
        "col",
      ],
    });
  } catch (error) {
    console.error("markdown render failed:", error);
    return DOMPurify.sanitize(
      `<pre class="markdown-fallback">${escapeHtmlAttribute(props.content || "")}</pre>`,
    );
  }
});

async function onMarkdownClick(event: MouseEvent) {
  const target = event.target;
  if (!(target instanceof Element)) return;

  const copyButton = target.closest("[data-code-copy]");
  if (copyButton instanceof HTMLButtonElement) {
    event.preventDefault();
    event.stopPropagation();
    const code = copyButton.closest(".code-block")?.querySelector("code");
    if (!(code instanceof HTMLElement)) return;
    try {
      await copyText(code.textContent || "");
      showCopyResult(copyButton, "Copied", true);
    } catch (error) {
      console.error("failed to copy code block:", error);
      showCopyResult(copyButton, "Failed", false);
    }
    return;
  }

  const image = target.closest("img");
  if (image instanceof HTMLImageElement) {
    const source = image.getAttribute("src")?.trim();
    if (!source) return;
    event.preventDefault();
    event.stopPropagation();
    emit("previewImage", source);
    return;
  }

  const anchor = target.closest("a");
  if (!(anchor instanceof HTMLAnchorElement)) return;

  const href = anchor.getAttribute("href")?.trim();
  if (!href || href.startsWith("#")) return;
  if (!/^(https?:|mailto:|tel:)/i.test(href)) return;

  event.preventDefault();
  event.stopPropagation();
  try {
    await openUrl(href);
  } catch (error) {
    console.error("failed to open url in default browser:", href, error);
  }
}

function showCopyResult(button: HTMLButtonElement, label: string, success: boolean) {
  const previousTimer = Number(button.dataset.copyResetTimer || 0);
  if (previousTimer) window.clearTimeout(previousTimer);
  renderButtonIcon(button, success ? Check : CircleX);
  button.setAttribute("aria-label", label);
  button.title = label;
  button.classList.toggle("copied", success);
  button.classList.toggle("copy-failed", !success);
  const timer = window.setTimeout(() => {
    if (!button.isConnected) return;
    renderButtonIcon(button, Copy);
    button.setAttribute("aria-label", "Copy code");
    button.title = "Copy code";
    button.classList.remove("copied", "copy-failed");
    delete button.dataset.copyResetTimer;
  }, 1600);
  button.dataset.copyResetTimer = String(timer);
}

function normalizeLegacyMath(content: string) {
  return content
    .split(/(```[\s\S]*?```|~~~[\s\S]*?~~~)/g)
    .map((part, index) => {
      if (index % 2 === 1) {
        return part;
      }

      const escapedBlocks = part.replace(/\\\[\s*([\s\S]*?)\s*\\\]/g, (match, formula: string) =>
        isLikelyTex(formula) ? asDisplayMath(formula) : match,
      );
      const withBlocks = escapedBlocks.replace(
        /^\s*\[\s*\r?\n([\s\S]*?)\r?\n\s*\]\s*$/gm,
        (match, formula: string) => (isLikelyTex(formula) ? asDisplayMath(formula) : match),
      );

      return withBlocks
        .replace(/\\\(\s*([\s\S]*?)\s*\\\)/g, (_match, formula: string) => `$${formula.trim()}$`)
        .replace(/\(\s*([^()\r\n]+?)\s*\)/g, (match, formula: string) =>
          isLikelyTex(formula) ? `$${formula.trim()}$` : match,
        );
    })
    .join("");
}

function asDisplayMath(value: string) {
  let formula = value.trim();
  if (/\\begin\{aligned\}/.test(formula)) {
    formula = formula.replace(/(?<!\\)\\\s*$/gm, "\\\\");
  }
  return `\n$$\n${formula}\n$$\n`;
}

function isLikelyTex(value: string) {
  const text = value.trim();
  return /\\[a-zA-Z]+|[_^=]/.test(text) || /^[a-zA-Z](?:_\{[^}]+\})?$/.test(text);
}
</script>

<style scoped>
.markdown-body {
  font-size: 13px;
  line-height: 1.65;
  color: var(--peek-text);
  overflow-wrap: anywhere;
}
.markdown-body :deep(img) {
  max-width: 100%;
  max-height: 280px;
  border-radius: 6px;
  object-fit: contain;
  margin: 8px 0;
  border: 1px solid color-mix(in srgb, var(--peek-border) 40%, transparent);
}

.markdown-body :deep(p) {
  margin: 0 0 0.65em;
}

.markdown-body :deep(p:last-child) {
  margin-bottom: 0;
}

.markdown-body :deep(h1),
.markdown-body :deep(h2),
.markdown-body :deep(h3),
.markdown-body :deep(h4) {
  margin: 0.8em 0 0.35em;
  color: var(--peek-text);
  font-weight: 650;
  line-height: 1.35;
}

.markdown-body :deep(h1:first-child),
.markdown-body :deep(h2:first-child),
.markdown-body :deep(h3:first-child) {
  margin-top: 0;
}

.markdown-body :deep(h1) {
  font-size: 1.18em;
}
.markdown-body :deep(h2) {
  font-size: 1.12em;
}
.markdown-body :deep(h3) {
  font-size: 1.06em;
}
.markdown-body :deep(h4) {
  font-size: 1em;
}

.markdown-body :deep(pre) {
  margin: 0.65em 0;
  padding: 10px 12px;
  border: 1px solid var(--peek-code-border, var(--peek-border));
  border-radius: 6px;
  background: color-mix(in srgb, var(--peek-input-bg) 82%, transparent);
  overflow-x: hidden;
  line-height: 1.55;
  tab-size: 2;
}

.markdown-body :deep(.code-block) {
  margin: 0.65em 0;
  overflow: hidden;
  border: 1px solid var(--peek-code-border, var(--peek-border));
  border-radius: 6px;
  background: var(--peek-code-bg, color-mix(in srgb, var(--peek-input-bg) 82%, transparent));
}

.markdown-body :deep(.code-block-toolbar) {
  box-sizing: border-box;
  position: relative;
  flex: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  height: 32px;
  min-height: 32px;
  max-height: 32px;
  padding: 0 7px 0 11px;
  overflow: hidden;
  line-height: 1;
  border-bottom: 1px solid var(--peek-code-border, var(--peek-border));
  background: var(--peek-code-toolbar-bg, color-mix(in srgb, var(--peek-text) 5%, transparent));
}

.markdown-body :deep(.code-language) {
  box-sizing: border-box;
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 31px;
  color: var(--peek-code-muted, var(--peek-muted));
  font-family: var(--font-mono);
  font-size: 10px;
  line-height: 1;
  text-transform: uppercase;
}

.markdown-body :deep(.code-language-icon) {
  flex: none;
  width: 13px;
  height: 13px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.markdown-body :deep(.code-language-icon svg) {
  display: block;
  width: 13px;
  height: 13px;
  flex: none;
  color: var(--peek-syntax-type, color-mix(in srgb, var(--peek-accent) 78%, var(--peek-muted)));
}

.markdown-body :deep(.code-copy-button) {
  box-sizing: border-box;
  flex: none;
  align-self: center;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 24px;
  padding: 0;
  border: 1px solid
    var(--peek-code-border, color-mix(in srgb, var(--peek-text) 14%, var(--peek-border)));
  border-radius: 5px;
  background: color-mix(in srgb, var(--peek-code-fg, var(--peek-text)) 8%, transparent);
  color: var(--peek-code-icon, var(--peek-code-fg, var(--peek-text)));
  cursor: pointer;
}

.markdown-body :deep(.code-copy-button svg) {
  display: block;
  flex: none;
  width: 14px;
  height: 14px;
}

.markdown-body :deep(.code-copy-button:hover) {
  border-color: var(--peek-code-border, var(--peek-border));
  background: var(--peek-code-hover-bg, color-mix(in srgb, var(--peek-text) 8%, transparent));
  color: var(--peek-code-fg, var(--peek-text));
}

.markdown-body :deep(.code-copy-button:focus-visible) {
  outline: 2px solid color-mix(in srgb, var(--peek-accent) 55%, transparent);
  outline-offset: 1px;
}

.markdown-body :deep(.code-copy-button.copied) {
  color: #36a269;
}

.markdown-body :deep(.code-copy-button.copy-failed) {
  color: #d35f5f;
}

.markdown-body :deep(.code-block pre) {
  margin: 0;
  border: 0;
  border-radius: 0;
  background: transparent;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  word-break: break-word;
}

.markdown-body :deep(code) {
  font-family: var(--font-mono);
  font-size: 12px;
}

.markdown-body :deep(pre code) {
  display: block;
  width: 100%;
  min-width: 0;
  background: transparent;
  padding: 0;
  color: var(--peek-code-fg, var(--peek-text));
  white-space: inherit;
  overflow-wrap: inherit;
  word-break: inherit;
}

.markdown-body :deep(.code-block code::selection),
.markdown-body :deep(.code-block code *::selection) {
  background: var(--peek-code-selection, var(--peek-list-active));
}

.markdown-body :deep(:not(pre) > code) {
  padding: 1px 4px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--peek-text) 10%, transparent);
}

.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  margin: 0.3em 0 0.65em;
  padding-left: 1.45em;
}

.markdown-body :deep(li + li) {
  margin-top: 0.2em;
}

.markdown-body :deep(li > p) {
  margin-bottom: 0.25em;
}

.markdown-body :deep(input[type="checkbox"]) {
  margin: 0 0.4em 0 -1.2em;
  accent-color: var(--peek-accent);
}

.markdown-body :deep(a) {
  color: var(--peek-accent);
  text-decoration: underline;
  text-decoration-color: color-mix(in srgb, var(--peek-accent) 45%, transparent);
  text-underline-offset: 2px;
}

.markdown-body :deep(blockquote) {
  margin: 0.65em 0;
  padding: 0.15em 0 0.15em 0.8em;
  border-left: 3px solid color-mix(in srgb, var(--peek-accent) 55%, transparent);
  color: var(--peek-muted);
}

.markdown-body :deep(blockquote > :last-child) {
  margin-bottom: 0;
}

.markdown-body :deep(hr) {
  margin: 1em 0;
  border: 0;
  border-top: 1px solid var(--peek-border);
}

.markdown-body :deep(table) {
  display: block;
  width: max-content;
  max-width: 100%;
  margin: 0.65em 0;
  border-collapse: collapse;
  overflow-x: auto;
}

.markdown-body :deep(.katex-display) {
  max-width: 100%;
  margin: 0.8em 0;
  overflow-x: auto;
  overflow-y: hidden;
  padding: 0.15em 0;
}

.markdown-body :deep(.katex-display > .katex) {
  min-width: max-content;
  text-align: center;
}

.markdown-body :deep(.katex) {
  font-size: 1.05em;
}

.markdown-body :deep(th),
.markdown-body :deep(td) {
  padding: 5px 9px;
  border: 1px solid var(--peek-border);
  text-align: left;
}

.markdown-body :deep(th) {
  background: color-mix(in srgb, var(--peek-text) 7%, transparent);
  font-weight: 650;
}

.markdown-body :deep(img) {
  cursor: zoom-in;
}

.markdown-body :deep(.hljs-comment),
.markdown-body :deep(.hljs-quote) {
  color: #7f8c98;
  font-style: italic;
}
.markdown-body :deep(.hljs-keyword),
.markdown-body :deep(.hljs-selector-tag),
.markdown-body :deep(.hljs-literal),
.markdown-body :deep(.hljs-type) {
  color: #c792ea;
}
.markdown-body :deep(.hljs-string),
.markdown-body :deep(.hljs-regexp),
.markdown-body :deep(.hljs-addition),
.markdown-body :deep(.hljs-attribute) {
  color: #addb67;
}
.markdown-body :deep(.hljs-number),
.markdown-body :deep(.hljs-symbol),
.markdown-body :deep(.hljs-bullet) {
  color: #f78c6c;
}
.markdown-body :deep(.hljs-title),
.markdown-body :deep(.hljs-section),
.markdown-body :deep(.hljs-function .hljs-title) {
  color: #82aaff;
}
.markdown-body :deep(.hljs-variable),
.markdown-body :deep(.hljs-template-variable),
.markdown-body :deep(.hljs-params) {
  color: #f07178;
}
.markdown-body :deep(.hljs-built_in),
.markdown-body :deep(.hljs-meta),
.markdown-body :deep(.hljs-link) {
  color: #ffcb6b;
}
.markdown-body :deep(.hljs-deletion) {
  color: #ff5370;
}
.markdown-body :deep(pre code.language-diff .hljs-addition) {
  display: inline-block;
  min-width: 100%;
  background: color-mix(in srgb, #22c55e 18%, transparent);
}
.markdown-body :deep(pre code.language-diff .hljs-deletion) {
  display: inline-block;
  min-width: 100%;
  background: color-mix(in srgb, var(--destructive) 18%, transparent);
}

:global([data-theme="light"] .markdown-body .hljs-comment),
:global([data-theme="light"] .markdown-body .hljs-quote) {
  color: #66736f;
}
:global([data-theme="light"] .markdown-body .hljs-keyword),
:global([data-theme="light"] .markdown-body .hljs-type) {
  color: #7652a6;
}
:global([data-theme="light"] .markdown-body .hljs-string),
:global([data-theme="light"] .markdown-body .hljs-addition) {
  color: #267045;
}
:global([data-theme="light"] .markdown-body .hljs-number),
:global([data-theme="light"] .markdown-body .hljs-variable) {
  color: #a0492d;
}
:global([data-theme="light"] .markdown-body .hljs-title),
:global([data-theme="light"] .markdown-body .hljs-section) {
  color: #28699c;
}
:global([data-theme="light"] .markdown-body .hljs-built_in),
:global([data-theme="light"] .markdown-body .hljs-meta) {
  color: #8a661f;
}
:global([data-theme="light"] .markdown-body .hljs-deletion) {
  color: #b84f48;
}
</style>
