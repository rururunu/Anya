import { computed, onScopeDispose, ref, watch } from "vue";
import {
  codeChangesFromUnifiedDiff,
  extractCodeChanges,
  lastRoundMessages,
  type CodeChangeEntry,
} from "@/services/chat/codeChanges";
import { workingTreeDiff } from "@/services/ipc/commands";
import type { ChatMessage } from "@/types/chat";

export type DiffScope = "round" | "uncommitted" | "session";

const STORAGE_KEY = "anya.diffScope";

/** Statuses that mean the round has stopped touching files. */
const TERMINAL_STATUSES = new Set<ChatMessage["status"]>(["done", "error", "cancelled"]);
/** Collapses a burst of rounds settling at the same time into one refresh. */
const REFRESH_DEBOUNCE_MS = 400;

function readStoredScope(): DiffScope {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "round" || stored === "uncommitted" || stored === "session") return stored;
  return "round";
}

/** Agent-edit vs git working-tree source for the review diff pane. */
export function useDiffScope(messages: () => ChatMessage[]) {
  const scope = ref<DiffScope>(readStoredScope());
  const uncommitted = ref<CodeChangeEntry[]>([]);
  const loading = ref(false);

  function setScope(next: DiffScope) {
    if (scope.value === next) return;
    scope.value = next;
    localStorage.setItem(STORAGE_KEY, next);
  }

  async function refreshUncommitted() {
    loading.value = true;
    try {
      uncommitted.value = codeChangesFromUnifiedDiff(await workingTreeDiff());
    } catch {
      uncommitted.value = [];
    } finally {
      loading.value = false;
    }
  }

  let refreshTimer: ReturnType<typeof setTimeout> | undefined;

  function scheduleRefresh() {
    if (refreshTimer) clearTimeout(refreshTimer);
    refreshTimer = setTimeout(() => {
      refreshTimer = undefined;
      void refreshUncommitted();
    }, REFRESH_DEBOUNCE_MS);
  }

  onScopeDispose(() => {
    if (refreshTimer) clearTimeout(refreshTimer);
  });

  watch(
    scope,
    (value) => {
      if (value === "uncommitted") void refreshUncommitted();
    },
    { immediate: true },
  );

  watch(
    () => {
      const list = messages();
      const last = list[list.length - 1];
      return `${list.length}:${last?.id ?? ""}:${last?.status ?? ""}`;
    },
    () => {
      if (scope.value !== "uncommitted") return;
      const list = messages();
      const last = list[list.length - 1];
      // A streaming round rewrites files continuously, so re-running the whole
      // working-tree diff on every status tick is wasted work. Wait for the
      // round to settle, then collapse a burst of settles into one refresh.
      if (last && !TERMINAL_STATUSES.has(last.status)) return;
      scheduleRefresh();
    },
  );

  const changes = computed(() => {
    if (scope.value === "uncommitted") return uncommitted.value;
    const list = messages();
    const source = scope.value === "round" ? lastRoundMessages(list) : list;
    const entries = extractCodeChanges(source);
    return [...entries].reverse();
  });

  return { scope, setScope, changes, loading };
}
