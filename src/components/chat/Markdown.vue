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
  CodeXml,
  Copy,
  Database,
  FileCode2,
  GitCompareArrows,
  Hash,
  TextWrap,
} from "@lucide/vue";
import DOMPurify from "dompurify";
import hljs from "highlight.js/lib/common";
import { marked } from "marked";
import {
  computed,
  h,
  nextTick,
  onMounted,
  onUnmounted,
  onUpdated,
  ref,
  render,
  watch,
  type Component,
} from "vue";
import { copyText } from "@/services/clipboard";
import { parseChartSpec } from "@/services/chat/chartSpec";
import { normalizeMarkdownInput } from "@/services/chat/markdownNormalize";
import { buildMermaidPlaceholder, shouldRenderMermaidBlock } from "@/services/chat/mermaidDiagram";
import { resolveChatImageSrc } from "@/services/chat/localImageSrc";
import { disposeChartBlocks, hydrateChartBlocks } from "./chartHydration";
import { disposeMermaidBlocks, hydrateMermaidBlocks } from "./mermaidHydration";

const props = defineProps<{
  content: string;
}>();
const emit = defineEmits<{
  previewImage: [source: string];
}>();
const rootRef = ref<HTMLElement | null>(null);

const renderer = new marked.Renderer();

renderer.code = ({ text, lang }) => {
  const requestedLanguage = (lang ?? "").trim().split(/\s+/)[0]?.toLowerCase() || "";

  if (requestedLanguage === "chart") {
    const spec = parseChartSpec(text ?? "");
    if (spec) {
      return `<div class="chart-block" data-chart-spec="${escapeHtmlAttribute(JSON.stringify(spec))}"></div>\n`;
    }
  }

  if (shouldRenderMermaidBlock(requestedLanguage, text ?? "")) {
    return buildMermaidPlaceholder(text ?? "");
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
  const blockClass = language ? "code-block" : "code-block code-block--plain";

  return `<div class="${blockClass}"><div class="code-block-toolbar"><span class="code-language"><span class="code-language-icon" data-code-language-icon="${language}"></span><span class="code-language-label">${languageLabel}</span></span><div class="code-block-actions"><button type="button" class="code-action-button" data-code-wrap aria-label="Toggle wrap" title="Toggle wrap" aria-pressed="true"></button><button type="button" class="code-action-button" data-code-copy aria-label="Copy code" title="Copy code"></button></div></div><div class="code-block-body"><pre><code class="hljs${languageClass}">${highlighted}</code></pre></div></div>\n`;
};

renderer.image = ({ href, title, text }) => {
  const original = (href ?? "").trim();
  const src = resolveChatImageSrc(original);
  const titleAttr = title ? ` title="${escapeHtmlAttribute(title)}"` : "";
  return `<img src="${escapeHtmlAttribute(src)}" alt="${escapeHtmlAttribute(text ?? "")}" data-image-source="${escapeHtmlAttribute(original)}"${titleAttr} />`;
};

function iconForLanguage(language: string): Component {
  if (["diff", "patch"].includes(language)) {
    return GitCompareArrows;
  }
  if (
    ["bash", "shell", "sh", "zsh", "fish", "powershell", "ps1", "bat", "cmd"].includes(language)
  ) {
    return CodeXml;
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
    bash: "bash",
    sh: "bash",
    zsh: "zsh",
    shell: "shell",
    ps1: "powershell",
    powershell: "powershell",
    js: "javascript",
    jsx: "jsx",
    ts: "typescript",
    tsx: "tsx",
    cs: "csharp",
    csharp: "csharp",
    cpp: "cpp",
    yml: "yaml",
    md: "markdown",
    py: "python",
    rs: "rust",
  };
  return labels[language] || language || "text";
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
  root.querySelectorAll<HTMLButtonElement>("[data-code-wrap]").forEach((button) => {
    renderButtonIcon(button, TextWrap);
  });
  root.querySelectorAll<HTMLButtonElement>("[data-code-copy]").forEach((button) => {
    renderButtonIcon(button, Copy);
  });
}

let stickyScrollCleanup: (() => void) | null = null;

function findScrollContainer(element: HTMLElement | null): HTMLElement | Window {
  let current = element?.parentElement;
  while (current) {
    const style = window.getComputedStyle(current);
    if (style.overflowY === "auto" || style.overflowY === "scroll") {
      return current;
    }
    current = current.parentElement;
  }
  return window;
}

function updateStickyCodeBlocks() {
  const root = rootRef.value;
  if (!root) return;
  const blocks = root.querySelectorAll<HTMLElement>(".code-block");
  for (const block of blocks) {
    const toolbar = block.querySelector<HTMLElement>(".code-block-toolbar");
    if (!toolbar) continue;
    const bRect = block.getBoundingClientRect();
    const tRect = toolbar.getBoundingClientRect();
    const isStuck = bRect.top < tRect.top - 1 && bRect.bottom > tRect.bottom;
    block.classList.toggle("is-stuck", isStuck);
  }
}

function setupStickyScrollObserver() {
  stickyScrollCleanup?.();
  stickyScrollCleanup = null;

  const root = rootRef.value;
  if (!root) return;
  const blocks = root.querySelectorAll<HTMLElement>(".code-block");
  if (!blocks.length) return;

  const container = findScrollContainer(root);
  let rafId = 0;
  const onScroll = () => {
    if (rafId) return;
    rafId = window.requestAnimationFrame(() => {
      rafId = 0;
      updateStickyCodeBlocks();
    });
  };

  container.addEventListener("scroll", onScroll, { passive: true });
  window.addEventListener("resize", onScroll, { passive: true });
  updateStickyCodeBlocks();

  stickyScrollCleanup = () => {
    if (rafId) window.cancelAnimationFrame(rafId);
    container.removeEventListener("scroll", onScroll);
    window.removeEventListener("resize", onScroll);
  };
}

function hydrateAll() {
  hydrateCodeBlockIcons();
  setupStickyScrollObserver();
  const root = rootRef.value;
  if (root) {
    hydrateChartBlocks(root);
    hydrateMermaidBlocks(root);
  }
}

function unmountPortalHosts(root: HTMLElement) {
  root
    .querySelectorAll<HTMLElement>(
      "[data-code-language-icon], [data-code-copy], [data-code-wrap], [data-chart-spec]",
    )
    .forEach((el) => {
      render(null, el);
    });
  disposeChartBlocks(root);
  disposeMermaidBlocks(root);
}

onMounted(hydrateAll);
onUpdated(hydrateAll);
onUnmounted(() => {
  stickyScrollCleanup?.();
  stickyScrollCleanup = null;
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
    const raw = marked.parse(normalizeMarkdownInput(props.content || ""), {
      async: false,
    }) as string;
    return DOMPurify.sanitize(raw, {
      ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto|tel|file|sms):|[^&#]*?:|data:image\/)/i,
      ADD_ATTR: ["data-image-source", "data-mermaid-source", "data-mermaid-block", "hidden"],
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

watch(html, () => {
  void nextTick(hydrateAll);
});

async function onMarkdownClick(event: MouseEvent) {
  const target = event.target;
  if (!(target instanceof Element)) return;

  const wrapButton = target.closest("[data-code-wrap]");
  if (wrapButton instanceof HTMLButtonElement) {
    event.preventDefault();
    event.stopPropagation();
    const block = wrapButton.closest(".code-block");
    if (!(block instanceof HTMLElement)) return;
    const nowrap = block.classList.toggle("is-nowrap");
    wrapButton.setAttribute("aria-pressed", String(!nowrap));
    const label = nowrap ? "Wrap lines" : "Unwrap lines";
    wrapButton.setAttribute("aria-label", label);
    wrapButton.title = label;
    return;
  }

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
    const source =
      image.getAttribute("data-image-source")?.trim() || image.getAttribute("src")?.trim();
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
</script>

<style scoped>
.markdown-body {
  font-size: var(--peek-agent-font-size, 13px);
  font-weight: 500;
  line-height: 1.7;
  color: var(--peek-text);
  overflow-wrap: break-word;
  word-break: normal;
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

.markdown-body :deep(pre:not(.code-block pre)) {
  margin: 0.75em 0;
  padding: 12px 16px;
  border: 1px solid color-mix(in srgb, var(--peek-text) 8%, transparent);
  border-radius: 12px;
  background: var(--peek-code-body-bg);
  overflow-x: auto;
  line-height: 1.65;
  tab-size: 2;
}

.markdown-body :deep(.code-block) {
  margin: 0.85em 0;
  overflow: clip;
  border: 1px solid color-mix(in srgb, var(--peek-text) 8%, transparent);
  border-radius: 12px;
  background: var(--peek-code-body-bg);
}

.markdown-body :deep(.code-block-toolbar) {
  box-sizing: border-box;
  position: sticky;
  top: var(--code-block-sticky-top, 0);
  z-index: 2;
  flex: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  width: 100%;
  min-height: 36px;
  padding: 6px 8px 2px 14px;
  overflow: hidden;
  line-height: 1;
  background: var(--peek-code-body-bg);
  border-top-left-radius: 11px;
  border-top-right-radius: 11px;
}

.markdown-body :deep(.code-block.is-stuck),
.markdown-body :deep(.code-block.is-stuck .code-block-toolbar) {
  border-top-left-radius: 0;
  border-top-right-radius: 0;
}

.markdown-body :deep(.code-block.is-stuck .code-block-toolbar) {
  box-shadow: 0 8px 10px -10px color-mix(in srgb, var(--peek-text) 28%, transparent);
}

.markdown-body :deep(.code-language) {
  box-sizing: border-box;
  flex: 1;
  min-width: 0;
  display: inline-flex;
  align-items: center;
  gap: 7px;
  color: var(--peek-code-muted);
  font-family: var(--font-sans);
  font-size: 12px;
  font-weight: 500;
  line-height: 1.2;
}

.markdown-body :deep(.code-language-label) {
  display: inline-block;
  max-width: 100%;
  overflow: hidden;
  color: var(--peek-code-muted);
  font-family: var(--font-sans);
  font-size: 12px;
  font-weight: 500;
  letter-spacing: 0;
  text-overflow: ellipsis;
  text-transform: none;
  white-space: nowrap;
}

.markdown-body :deep(.code-language-icon) {
  flex: none;
  width: 14px;
  height: 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  opacity: 0.78;
}

.markdown-body :deep(.code-language-icon svg) {
  display: block;
  width: 14px;
  height: 14px;
  flex: none;
  color: var(--peek-code-muted);
}

.markdown-body :deep(.code-block-actions) {
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: 1px;
}

.markdown-body :deep(.code-action-button) {
  box-sizing: border-box;
  flex: none;
  align-self: center;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  padding: 0;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--peek-code-icon);
  cursor: pointer;
  opacity: 0.72;
  transition:
    opacity var(--motion-fast, 120ms) ease,
    background-color var(--motion-fast, 120ms) ease,
    color var(--motion-fast, 120ms) ease;
}

.markdown-body :deep(.code-action-button svg) {
  display: block;
  flex: none;
  width: 14px;
  height: 14px;
}

.markdown-body :deep(.code-action-button:hover) {
  background: color-mix(in srgb, var(--peek-text) 7%, transparent);
  color: var(--peek-code-fg);
  opacity: 1;
}

.markdown-body :deep(.code-action-button:focus-visible) {
  opacity: 1;
  outline: 2px solid color-mix(in srgb, var(--peek-accent) 55%, transparent);
  outline-offset: 1px;
}

.markdown-body :deep(.code-action-button[aria-pressed="true"]) {
  opacity: 0.92;
}

.markdown-body :deep(.code-action-button.copied) {
  opacity: 1;
  color: var(--peek-success, #36a269);
}

.markdown-body :deep(.code-action-button.copy-failed) {
  opacity: 1;
  color: var(--peek-danger, #d35f5f);
}

.markdown-body :deep(.code-block-body) {
  background: transparent;
  border-bottom-left-radius: 11px;
  border-bottom-right-radius: 11px;
}

.markdown-body :deep(.code-block pre) {
  margin: 0;
  border: 0;
  border-radius: 0;
  background: transparent;
  padding: 8px 16px 14px;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  word-break: break-word;
}

.markdown-body :deep(.code-block.is-nowrap .code-block-body) {
  overflow-x: auto;
}

.markdown-body :deep(.code-block.is-nowrap pre) {
  white-space: pre;
  overflow-wrap: normal;
  word-break: normal;
}

.markdown-body :deep(.code-block--plain pre code) {
  color: color-mix(in srgb, var(--peek-code-fg) 88%, var(--peek-code-muted));
}

.markdown-body :deep(code) {
  font-family: var(--font-mono);
  font-size: 12.5px;
  line-height: 1.65;
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
  padding: 0.1em 0.32em;
  border: 1px solid color-mix(in srgb, var(--peek-code-border) 80%, transparent);
  border-radius: 4px;
  background: color-mix(in srgb, var(--peek-code-body-bg) 76%, var(--peek-surface));
  color: var(--peek-code-fg);
  font-size: 0.9em;
  font-weight: 500;
  vertical-align: 0.04em;
}

.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  margin: 0.4em 0 0.75em;
  padding-left: 1.7em;
  list-style-position: outside;
}

.markdown-body :deep(ul) {
  list-style-type: disc;
}

.markdown-body :deep(li) {
  line-height: 1.7;
}

.markdown-body :deep(li + li) {
  margin-top: 0.38em;
}

.markdown-body :deep(li > p) {
  margin: 0;
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
  margin: 0.75em 0;
  border: 0;
  border-collapse: collapse;
  border-spacing: 0;
  overflow-x: auto;
}

.markdown-body :deep(.mermaid-block) {
  margin: 0.75em 0;
}

.markdown-body :deep(th),
.markdown-body :deep(td) {
  padding: 0.42em 1.4em 0.42em 0;
  border: 0;
  background: transparent;
  text-align: left;
  vertical-align: top;
  overflow-wrap: break-word;
}

.markdown-body :deep(th:last-child),
.markdown-body :deep(td:last-child) {
  padding-right: 0;
}

.markdown-body :deep(th) {
  color: var(--peek-text);
  font-weight: 650;
  padding-bottom: 0.55em;
}

.markdown-body :deep(td) {
  color: var(--peek-text);
}

.markdown-body :deep(img) {
  cursor: zoom-in;
}
</style>
