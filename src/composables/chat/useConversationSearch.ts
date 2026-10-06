import {
  computed,
  nextTick,
  onMounted,
  onUnmounted,
  ref,
  watch,
  type ComputedRef,
  type Ref,
} from "vue";
import {
  applyFindHits,
  clearFindHits,
  paintCurrentFindHit,
} from "@/services/chat/conversationFind";
import { provideConversationFind } from "./useConversationFind";
import { tr } from "@/services/i18n";
import type { ChatMessage } from "@/types/chat";
import type { AppLanguage } from "@/types/setting";
interface ConversationSearchOptions {
  listRef: Ref<HTMLElement | null>;
  stickToBottom: Ref<boolean>;
  sessionId: ComputedRef<string | undefined>;
  messages: ComputedRef<ChatMessage[]>;
  language: ComputedRef<AppLanguage>;
}
/** Owns conversation search state, keyboard shortcuts, highlighting and their cleanup. */
export function useConversationSearch(options: ConversationSearchOptions) {
  const { listRef, stickToBottom, sessionId, messages, language } = options;
  const findOpen = ref(false);
  const findQuery = ref("");
  const findIndex = ref(0);
  const findHits = ref<HTMLElement[]>([]);
  const findInputRef = ref<HTMLInputElement | null>(null);
  provideConversationFind({
    active: findOpen,
    query: findQuery,
  });

  const findCountLabel = computed(() => {
    const query = findQuery.value.trim();
    if (!query) return "";
    if (findHits.value.length === 0) return tr(language.value, "findNoResults");
    return tr(language.value, "findMatchCount", {
      current: String(findIndex.value + 1),
      total: String(findHits.value.length),
    });
  });

  async function refreshFindHits(options: { scroll: boolean; resetIndex?: boolean }) {
    await nextTick();
    await nextTick();
    const hits = applyFindHits(listRef.value, findOpen.value ? findQuery.value : "");
    findHits.value = hits;
    if (!hits.length) {
      findIndex.value = 0;
      return;
    }
    if (options.resetIndex || findIndex.value >= hits.length) findIndex.value = 0;
    paintCurrentFindHit(hits, findIndex.value);
    if (options.scroll) scrollFindHit(hits[findIndex.value]);
  }

  function scrollFindHit(mark: HTMLElement | undefined) {
    const container = listRef.value;
    if (!container || !mark) return;
    stickToBottom.value = false;
    const containerRect = container.getBoundingClientRect();
    const markRect = mark.getBoundingClientRect();
    const offset = markRect.top - containerRect.top - Math.max(56, container.clientHeight * 0.28);
    container.scrollTo({ top: Math.max(0, container.scrollTop + offset), behavior: "smooth" });
  }

  function nextFind() {
    if (!findHits.value.length) return;
    findIndex.value = (findIndex.value + 1) % findHits.value.length;
    paintCurrentFindHit(findHits.value, findIndex.value);
    scrollFindHit(findHits.value[findIndex.value]);
  }

  function prevFind() {
    if (!findHits.value.length) return;
    findIndex.value = (findIndex.value - 1 + findHits.value.length) % findHits.value.length;
    paintCurrentFindHit(findHits.value, findIndex.value);
    scrollFindHit(findHits.value[findIndex.value]);
  }

  function openFind() {
    findOpen.value = true;
    void nextTick(() => {
      findInputRef.value?.focus();
      findInputRef.value?.select();
      refreshFindHits({ scroll: Boolean(findQuery.value.trim()), resetIndex: false });
    });
  }

  function closeFind() {
    if (!findOpen.value) return;
    findOpen.value = false;
    clearFindHits(listRef.value);
    findHits.value = [];
    findIndex.value = 0;
  }

  function onFindInputKeydown(event: KeyboardEvent) {
    if (event.isComposing) return;
    if (event.key === "ArrowDown" || (event.key === "Enter" && !event.shiftKey)) {
      event.preventDefault();
      nextFind();
      return;
    }
    if (event.key === "ArrowUp" || (event.key === "Enter" && event.shiftKey)) {
      event.preventDefault();
      prevFind();
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeFind();
    }
  }

  function onFindWindowKeydown(event: KeyboardEvent) {
    const mod = event.ctrlKey || event.metaKey;
    const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
    if (mod && !event.altKey && !event.shiftKey && key === "f") {
      event.preventDefault();
      openFind();
      return;
    }
    if (!findOpen.value) return;
    if (event.key === "Escape") {
      event.preventDefault();
      closeFind();
      return;
    }
    if (event.key === "F3") {
      event.preventDefault();
      if (event.shiftKey) prevFind();
      else nextFind();
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      if (event.target === findInputRef.value) return;
      if (event.target instanceof HTMLElement) {
        if (event.target.closest(".search-palette, textarea, [contenteditable='true']")) return;
        if (event.target.tagName === "INPUT") return;
      }
      event.preventDefault();
      if (event.key === "ArrowDown") nextFind();
      else prevFind();
    }
  }

  watch(findQuery, () => {
    if (!findOpen.value) return;
    void nextTick(() =>
      refreshFindHits({ scroll: Boolean(findQuery.value.trim()), resetIndex: true }),
    );
  });

  watch(sessionId, () => {
    closeFind();
    findQuery.value = "";
  });

  watch(
    () => {
      const last = messages.value[messages.value.length - 1];
      return `${messages.value.length}:${last?.id ?? ""}:${last?.content.length ?? 0}:${last?.status ?? ""}`;
    },
    () => {
      if (!findOpen.value || !findQuery.value.trim()) return;
      void nextTick(() => refreshFindHits({ scroll: false }));
    },
  );

  onMounted(() => globalThis.addEventListener("keydown", onFindWindowKeydown));
  onUnmounted(() => {
    globalThis.removeEventListener("keydown", onFindWindowKeydown);
    clearFindHits(listRef.value);
  });
  return {
    findOpen,
    findQuery,
    findHits,
    findInputRef,
    findCountLabel,
    nextFind,
    prevFind,
    openFind,
    closeFind,
    onFindInputKeydown,
  };
}
