const MERMAID_LANG_ALIASES = new Set([
  "mermaid",
  "flowchart",
  "graph",
  "sequencediagram",
  "classdiagram",
  "statediagram",
  "statediagram-v2",
  "erdiagram",
  "journey",
  "gantt",
  "pie",
  "mindmap",
  "timeline",
  "gitgraph",
  "c4context",
  "block",
  "architecture",
]);

const MERMAID_FIRST_LINE =
  /^(flowchart|graph|sequenceDiagram|classDiagram|stateDiagram(?:-v2)?|erDiagram|journey|gantt|pie|mindmap|timeline|gitgraph|C4Context|block-beta|architecture-beta)\b/i;

export function isMermaidLanguage(lang: string): boolean {
  const normalized = lang.trim().toLowerCase();
  return MERMAID_LANG_ALIASES.has(normalized);
}

export function isMermaidDiagramSource(text: string): boolean {
  const firstLine =
    text
      .split(/\r?\n/)
      .map((line) => line.trim())
      .find(Boolean) ?? "";
  return MERMAID_FIRST_LINE.test(firstLine);
}

export function shouldRenderMermaidBlock(lang: string, text: string): boolean {
  if (isMermaidLanguage(lang)) return true;
  return !lang.trim() && isMermaidDiagramSource(text);
}

export function escapeMermaidHtmlText(value: string): string {
  return value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

export function buildMermaidPlaceholder(source: string): string {
  const body = escapeMermaidHtmlText(source);
  return `<div class="mermaid-block" data-mermaid-block><pre class="mermaid-source" hidden aria-hidden="true">${body}</pre></div>\n`;
}

export function readMermaidSource(node: HTMLElement): string {
  const pre = node.querySelector("pre.mermaid-source");
  if (pre?.textContent) return pre.textContent;
  return node.dataset.mermaidSource ?? "";
}
