import type { ToolActivity } from "@/types/chat";

export const FIND_HIT_CLASS = "conversation-find-hit";
export const FIND_CURRENT_CLASS = "is-current";

const SKIP_CLOSEST =
  "script, style, textarea, input, svg, .conversation-find-bar, .message-actions";

/** Remove previous in-conversation find marks and merge split text nodes. */
export function clearFindHits(root: HTMLElement | null) {
  if (!root) return;
  const marks = [...root.querySelectorAll(`mark.${FIND_HIT_CLASS}`)];
  for (const mark of marks) {
    const parent = mark.parentNode;
    if (!parent) continue;
    while (mark.firstChild) parent.insertBefore(mark.firstChild, mark);
    parent.removeChild(mark);
    parent.normalize();
  }
}

function shouldSkip(node: Text) {
  const el = node.parentElement;
  if (!el) return true;
  if (el.closest(SKIP_CLOSEST)) return true;
  if (el.closest(`mark.${FIND_HIT_CLASS}`)) return true;
  return false;
}

function wrapRangesInTextNode(text: Text, ranges: Array<[number, number]>, marks: HTMLElement[]) {
  for (let i = ranges.length - 1; i >= 0; i -= 1) {
    const start = ranges[i]?.[0];
    const end = ranges[i]?.[1];
    if (start == null || end == null || end <= start) continue;
    text.splitText(end);
    const match = text.splitText(start);
    const owner = text.ownerDocument;
    if (!owner) continue;
    const mark = owner.createElement("mark");
    mark.className = FIND_HIT_CLASS;
    match.parentNode?.replaceChild(mark, match);
    mark.appendChild(match);
    marks.unshift(mark);
  }
}

/** Wrap case-insensitive matches in `root` and return marks in document order. */
export function applyFindHits(root: HTMLElement | null, query: string): HTMLElement[] {
  clearFindHits(root);
  if (!root) return [];
  const needle = query.trim();
  if (!needle) return [];
  const lowerNeedle = needle.toLowerCase();
  const needleLen = lowerNeedle.length;
  const owner = root.ownerDocument;
  const walker = owner.createTreeWalker(root, 4);
  const texts: Text[] = [];
  let current: Node | null = walker.nextNode();
  while (current) {
    const text = current as Text;
    if (text.nodeValue && !shouldSkip(text) && text.nodeValue.toLowerCase().includes(lowerNeedle)) {
      texts.push(text);
    }
    current = walker.nextNode();
  }

  const marks: HTMLElement[] = [];
  for (const text of texts) {
    const value = text.nodeValue ?? "";
    const lower = value.toLowerCase();
    const ranges: Array<[number, number]> = [];
    let from = 0;
    while (from <= lower.length - needleLen) {
      const idx = lower.indexOf(lowerNeedle, from);
      if (idx === -1) break;
      ranges.push([idx, idx + needleLen]);
      from = idx + needleLen;
    }
    if (ranges.length) wrapRangesInTextNode(text, ranges, marks);
  }
  return marks;
}

export function paintCurrentFindHit(marks: HTMLElement[], currentIndex: number) {
  marks.forEach((mark, index) => {
    mark.classList.toggle(FIND_CURRENT_CLASS, index === currentIndex);
  });
}

export function textIncludesQuery(haystack: string | null | undefined, query: string) {
  const needle = query.trim().toLowerCase();
  if (!needle || !haystack) return false;
  return haystack.toLowerCase().includes(needle);
}

export function jsonIncludesQuery(value: unknown, query: string): boolean {
  if (!query.trim() || value == null) return false;
  if (typeof value === "string") return textIncludesQuery(value, query);
  if (typeof value === "number" || typeof value === "boolean") {
    return textIncludesQuery(String(value), query);
  }
  if (Array.isArray(value)) return value.some((item) => jsonIncludesQuery(item, query));
  if (typeof value === "object") {
    return Object.values(value as Record<string, unknown>).some((item) =>
      jsonIncludesQuery(item, query),
    );
  }
  return false;
}

export function activityMatchesQuery(activity: ToolActivity, query: string) {
  if (textIncludesQuery(activity.title, query)) return true;
  if (textIncludesQuery(activity.detail, query)) return true;
  if (textIncludesQuery(activity.result, query)) return true;
  if (textIncludesQuery(activity.preview?.path, query)) return true;
  if (textIncludesQuery(activity.preview?.unifiedDiff, query)) return true;
  if (textIncludesQuery(activity.preview?.oldText, query)) return true;
  if (textIncludesQuery(activity.preview?.newText, query)) return true;
  if (activity.preview?.affectedPaths?.some((path) => textIncludesQuery(path, query))) return true;
  return jsonIncludesQuery(activity.arguments, query);
}
