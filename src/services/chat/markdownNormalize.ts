/** Characters allowed inside a URL when rejoining streamed vertical fragments. */
const URL_FRAGMENT_CHAR = /^[a-zA-Z0-9:/?#=&%._+\-~−–—]$/;

const FENCED_BLOCK = /(```[\s\S]*?```|~~~[\s\S]*?~~~)/g;

/**
 * Prepare assistant markdown for rendering: fix streamed URL artifacts before
 * marked runs with `breaks: true` (which would otherwise turn one-char-per-line
 * URLs into a vertical letter stack).
 */
export function normalizeMarkdownInput(content: string): string {
  return content
    .split(FENCED_BLOCK)
    .map((part, index) => (index % 2 === 1 ? part : normalizePlainMarkdown(part)))
    .join("");
}

function normalizePlainMarkdown(text: string): string {
  return normalizeUrlCharacters(collapseVerticalUrlRuns(text));
}

function isUrlFragmentLine(line: string): boolean {
  const trimmed = line.trim();
  if (!trimmed) return false;
  // Real sentences/words are not URL fragments.
  if (trimmed.length > 4) return false;
  return [...trimmed].every((ch) => URL_FRAGMENT_CHAR.test(ch));
}

function joinUrlFragments(lines: string[]): string {
  return lines
    .map((line) => line.trim())
    .join("")
    .replace(/\u2212|\u2013|\u2014/g, "-")
    .replace(/\u200b|\u200c|\u200d|\ufeff/g, "");
}

function looksLikeUrl(value: string): boolean {
  return /^https?:\/\/\S/i.test(value);
}

function collapseVerticalUrlRuns(text: string): string {
  const lines = text.split("\n");
  const result: string[] = [];
  let index = 0;

  while (index < lines.length) {
    const line = lines[index]!;
    if (!isUrlFragmentLine(line)) {
      result.push(line);
      index += 1;
      continue;
    }

    const start = index;
    while (index < lines.length && isUrlFragmentLine(lines[index]!)) {
      index += 1;
    }

    const joined = joinUrlFragments(lines.slice(start, index));
    if (looksLikeUrl(joined)) {
      result.push(joined);
    } else {
      result.push(...lines.slice(start, index));
    }
  }

  return mergeSplitUrlTail(result.join("\n"));
}

/** Join a full URL line with short continuation fragments on the next lines. */
function mergeSplitUrlTail(text: string): string {
  const lines = text.split("\n");
  const result: string[] = [];

  for (let index = 0; index < lines.length; index += 1) {
    let current = lines[index]!;
    if (!/^https?:\/\//i.test(current.trim())) {
      result.push(current);
      continue;
    }

    while (index + 1 < lines.length) {
      const next = lines[index + 1]!;
      const trimmed = next.trim();
      if (!trimmed || !isUrlFragmentLine(next)) break;
      const piece = joinUrlFragments([next]);
      if (!/^[a-zA-Z0-9/?#=&%._+\-~]+$/i.test(piece)) break;
      current += piece;
      index += 1;
    }

    result.push(normalizeUrlCharacters(current));
  }

  return result.join("\n");
}

function normalizeUrlCharacters(text: string): string {
  return text.replace(/https?:\/\/[^\s<>)]+/gi, (url) =>
    url.replace(/\u2212|\u2013|\u2014/g, "-").replace(/\u200b|\u200c|\u200d|\ufeff/g, ""),
  );
}
