/**
 * Format session preview / titlebar text so @/# tokens read like composer chips
 * (basename / short label) instead of raw paths leaking into chrome.
 */

import { prettyHashInstallId } from "@/services/chat/hashMentionDisplay";

const INLINE_TOKEN_RE = /@(?:"([^"]+)"|([^\s@#]+))|#(?:skill|mcp):([A-Za-z0-9_.-]+)/g;
const LEADING_CHIP_RE =
  /^(?:@(?:"[^"]+"|[^\s@#]+)|#(?:skill|mcp):[A-Za-z0-9_.-]+|#\S+)(?:\s+(?:@(?:"[^"]+"|[^\s@#]+)|#(?:skill|mcp):[A-Za-z0-9_.-]+|#\S+))*\s*/;

function fileBaseName(path: string | undefined | null): string {
  const value = path ?? "";
  return value.split(/[/\\]/).pop() || value;
}

function prettifyTokens(text: string): string {
  return text.replace(
    INLINE_TOKEN_RE,
    (_match, quoted: string | undefined, bare: string | undefined, hashId: string | undefined) => {
      if (hashId) return prettyHashInstallId(hashId);
      return `@${fileBaseName(quoted || bare || "")}`;
    },
  );
}

function truncateChars(value: string, max: number): string {
  if ([...value].length <= max) return value;
  return `${[...value].slice(0, max).join("")}…`;
}

/** Human-facing session title for titlebar / sidebar lists. */
export function formatSessionPreview(preview: string | undefined | null, maxLen = 48): string {
  const normalized = (preview ?? "").replace(/\s+/g, " ").trim();
  if (!normalized) return "";

  // Strip leading wire tokens first so prose wins; then prettify any remaining chips.
  const prose = normalized.replace(LEADING_CHIP_RE, "").trim();
  if (prose) return truncateChars(prettifyTokens(prose), maxLen);
  return truncateChars(prettifyTokens(normalized), maxLen);
}
