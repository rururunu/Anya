import { computed, ref, watch } from "vue";
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
      if (scope.value === "uncommitted") void refreshUncommitted();
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
