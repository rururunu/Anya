/**
 * `#` resource mentions for Skills and MCP servers.
 *
 * Wire format in the sent message (human-readable + agent-parseable):
 * - `#skill:generate_bid_tech`
 * - `#mcp:server-id`
 *
 * Chips in the composer use the same tokens via {@link formatHashMention}.
 */

import {
  loadResourceUsage,
  resourceUsageScore,
  type ResourceUsageStore,
} from "@/services/usage/resourceUsage";

export type HashResourceKind = "skill" | "mcp";

export type HashMentionItem = {
  kind: HashResourceKind;
  /** Skill name or MCP server id. */
  id: string;
  /** Short UI label. */
  title: string;
  /** Optional one-line description. */
  description?: string;
  /** Remote or local icon URL (Smithery / cached install icon). */
  iconUrl?: string | null;
  /** Vendor / registry identity, e.g. `gmail` or `adamamer20/paper-search-mcp-openai`. */
  vendor?: string;
};

export type ActiveMention = {
  /** Text after the trigger character up to the caret (filter needle). */
  query: string;
  /** Index of `#` / `@` in the message. */
  start: number;
  /** Index just past the unfinished token (for replacement). */
  end: number;
};

/** Token written into the user message. */
export function formatHashMention(kind: HashResourceKind, id: string): string {
  const cleaned = id.trim().replace(/\s+/g, "-");
  return `#${kind}:${cleaned}`;
}

const HASH_TOKEN_RE = /#(skill|mcp):([A-Za-z0-9_.-]+)/g;

/** Parse all `#skill:` / `#mcp:` tokens from free text. */
export function parseHashMentions(text: string): Array<{ kind: HashResourceKind; id: string }> {
  const out: Array<{ kind: HashResourceKind; id: string }> = [];
  const re = new RegExp(HASH_TOKEN_RE.source, "g");
  let match: RegExpExecArray | null;
  while ((match = re.exec(text)) !== null) {
    const kind = match[1] as HashResourceKind;
    const id = match[2] ?? "";
    if (id) out.push({ kind, id });
  }
  return out;
}

/**
 * Active `#…` query relative to the caret.
 * Supports typing at the start, middle, or end of already-entered text
 * (mention must start at beginning-of-string or after whitespace).
 */
export function activeHashMention(
  message: string,
  caret: number = message.length,
): ActiveMention | null {
  return activeTriggerMention(message, caret, "#");
}

/**
 * Active `@…` file query relative to the caret (same rules as `#`).
 */
export function activeFilePathMention(
  message: string,
  caret: number = message.length,
): ActiveMention | null {
  return activeTriggerMention(message, caret, "@");
}

function activeTriggerMention(
  message: string,
  caret: number,
  trigger: "#" | "@",
): ActiveMention | null {
  if (!message) return null;
  const safeCaret = Math.max(0, Math.min(caret, message.length));
  const before = message.slice(0, safeCaret);
  const escaped = trigger === "#" ? "#" : "@";
  const match = before.match(new RegExp(`(?:^|[\\s\\n])${escaped}([^\\s${escaped}]*)$`));
  if (!match || match.index === undefined) return null;

  const start = match.index + match[0].indexOf(trigger);
  // Replace only through the caret so inserting `#` / `@` in front of existing
  // CJK or prose never treats that prose as part of the mention token.
  const end = safeCaret;
  const query = message.slice(start + 1, end);

  // A completed `#skill:id` / `#mcp:id` token should not keep the picker open.
  if (trigger === "#" && /^(skill|mcp):[A-Za-z0-9_.-]+$/.test(query)) {
    return null;
  }

  return { query, start, end };
}

/**
 * Filter skill/MCP catalog by the typed query after `#`.
 * Supports prefixes like `skill:`, `mcp:`, or free-text against id/title/vendor/desc.
 * Frequent / recent usage ranks above alphabetical order.
 */
export function filterHashMentionItems(
  items: readonly HashMentionItem[],
  rawQuery: string,
  usage: ResourceUsageStore = loadResourceUsage(),
  now = Date.now(),
): HashMentionItem[] {
  const query = rawQuery.trim().toLowerCase();
  if (!query) {
    return [...items]
      .sort((left, right) => compareMentionItems(left, right, "", usage, now))
      .slice(0, 24);
  }

  let kindFilter: HashResourceKind | null = null;
  let needle = query;

  if (query === "skill" || query === "skills") {
    kindFilter = "skill";
    needle = "";
  } else if (query === "mcp") {
    kindFilter = "mcp";
    needle = "";
  } else if (query.startsWith("skill:") || query.startsWith("skill/")) {
    kindFilter = "skill";
    needle = query.slice(6);
  } else if (query.startsWith("mcp:") || query.startsWith("mcp/")) {
    kindFilter = "mcp";
    needle = query.slice(4);
  }

  return items
    .filter((item) => {
      if (kindFilter && item.kind !== kindFilter) return false;
      if (!needle) return true;
      const hay =
        `${item.id} ${item.title} ${item.vendor ?? ""} ${item.description ?? ""}`.toLowerCase();
      return hay.includes(needle);
    })
    .sort((left, right) => compareMentionItems(left, right, needle, usage, now))
    .slice(0, 24);
}

function compareMentionItems(
  left: HashMentionItem,
  right: HashMentionItem,
  needle: string,
  usage: ResourceUsageStore,
  now: number,
): number {
  const leftId = left.id.toLowerCase();
  const rightId = right.id.toLowerCase();
  const leftVendor = (left.vendor ?? "").toLowerCase();
  const rightVendor = (right.vendor ?? "").toLowerCase();
  const leftPrefix = needle && (leftId.startsWith(needle) || leftVendor.startsWith(needle)) ? 0 : 1;
  const rightPrefix =
    needle && (rightId.startsWith(needle) || rightVendor.startsWith(needle)) ? 0 : 1;
  if (leftPrefix !== rightPrefix) return leftPrefix - rightPrefix;

  // Keep Skills and MCP in separate blocks (skills first), then rank by usage.
  if (left.kind !== right.kind) return left.kind === "skill" ? -1 : 1;

  const leftUsage = resourceUsageScore(left.kind, left.id, usage, now);
  const rightUsage = resourceUsageScore(right.kind, right.id, usage, now);
  if (leftUsage !== rightUsage) return rightUsage - leftUsage;

  return leftId.localeCompare(rightId);
}
