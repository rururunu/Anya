/**
 * Persisted expand/collapse state of the left navigation sidebar.
 *
 * Only *expanded* ids are stored: anything absent counts as collapsed, so a
 * fresh install — and any workspace or section created later — starts
 * collapsed, which is the intended default.
 */
const STORAGE_KEY = "anya.workbenchNavigationExpanded.v1";

export interface NavigationExpandedState {
  sections: Set<string>;
  workspaces: Set<string>;
}

function readIds(value: unknown): Set<string> {
  if (!Array.isArray(value)) return new Set();
  return new Set(value.filter((id): id is string => typeof id === "string" && id.length > 0));
}

function emptyState(): NavigationExpandedState {
  return { sections: new Set(), workspaces: new Set() };
}

/** Expanded ids from storage; anything unreadable degrades to "all collapsed". */
export function readExpandedNavigationState(): NavigationExpandedState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return emptyState();
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return emptyState();
    const record = parsed as { sections?: unknown; workspaces?: unknown };
    return { sections: readIds(record.sections), workspaces: readIds(record.workspaces) };
  } catch {
    // Unavailable or corrupted storage must not block the sidebar.
    return emptyState();
  }
}

export function persistExpandedNavigationState(state: NavigationExpandedState): void {
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ sections: [...state.sections], workspaces: [...state.workspaces] }),
    );
  } catch {
    // Best effort: a blocked or full storage must not break toggling.
  }
}
