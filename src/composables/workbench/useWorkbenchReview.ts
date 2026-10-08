import { FileCode2, Bug, FileDiff, FileText } from "@lucide/vue";
import { useCodeReadPreviewStore } from "@/stores/codeReadPreview";
import { useSharedReviewState } from "@/composables/chat/useSharedReviewState";
import { computed, onMounted, ref, watch, type ComputedRef, type Ref } from "vue";
import { tr } from "@/services/i18n";

import {
  useReviewSidebarResize,
  readStoredReviewSidebarWidth,
  REVIEW_RESIZE_HANDLE_WIDTH,
  REVIEW_SIDEBAR_MIN_WIDTH,
  REVIEW_SIDEBAR_MAX_WIDTH,
} from "@/composables/useReviewSidebarResize";
import {
  useNavigationSidebarResize,
  readStoredNavigationSidebarWidth,
  NAVIGATION_RESIZE_HANDLE_WIDTH,
  NAVIGATION_SIDEBAR_MIN_WIDTH,
  NAVIGATION_SIDEBAR_MAX_WIDTH,
} from "@/composables/useNavigationSidebarResize";
import { SUBAGENT_TOOLS } from "@/services/chat/subagentTools";
import { findSubagentEntry, resolvePanelSessionId } from "@/services/chat/subagentPanel";
import { rootSessionId } from "@/services/chat/subagentSession";
import { useSubagentSessionStore } from "@/stores/subagentSessions";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage, ChatSessionSummary } from "@/types/chat";
import type { ReviewView } from "./types";
import type { WorkbenchLabels } from "./useWorkbenchLabels";

export {
  REVIEW_RESIZE_HANDLE_WIDTH,
  REVIEW_SIDEBAR_MIN_WIDTH,
  REVIEW_SIDEBAR_MAX_WIDTH,
  NAVIGATION_RESIZE_HANDLE_WIDTH,
  NAVIGATION_SIDEBAR_MIN_WIDTH,
  NAVIGATION_SIDEBAR_MAX_WIDTH,
};

const REVIEW_MEMORY_STORAGE_KEY = "anya.workbenchReviewBySession.v1";
const DEFAULT_REVIEW_VIEW = "diff";
const TRANSIENT_REVIEW_VIEWS = new Set(["agents", "subagents", "image"]);
const MAX_REVIEW_MEMORY_ENTRIES = 80;

type SessionReviewMemory = { view: string; open: boolean; at: number };

function normalizeReviewView(view: string): string {
  const stored = view.trim();
  if (!stored || TRANSIENT_REVIEW_VIEWS.has(stored)) return DEFAULT_REVIEW_VIEW;
  if (stored === "runtime" && !import.meta.env.DEV) return DEFAULT_REVIEW_VIEW;
  return stored;
}

function reviewMemoryId(sessionId: string): string {
  return rootSessionId(sessionId) || sessionId;
}

function readReviewMemoryMap(): Record<string, SessionReviewMemory> {
  try {
    const raw = localStorage.getItem(REVIEW_MEMORY_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    const out: Record<string, SessionReviewMemory> = {};
    for (const [id, value] of Object.entries(parsed)) {
      if (!id || !value || typeof value !== "object" || Array.isArray(value)) continue;
      const row = value as { view?: unknown; open?: unknown; at?: unknown };
      out[id] = {
        view: typeof row.view === "string" ? normalizeReviewView(row.view) : DEFAULT_REVIEW_VIEW,
        open: row.open === true,
        at: typeof row.at === "number" && Number.isFinite(row.at) ? row.at : 0,
      };
    }
    return out;
  } catch {
    return {};
  }
}

function readSessionReviewMemory(sessionId: string): SessionReviewMemory {
  const id = reviewMemoryId(sessionId);
  if (!id) return { view: DEFAULT_REVIEW_VIEW, open: false, at: 0 };
  return readReviewMemoryMap()[id] ?? { view: DEFAULT_REVIEW_VIEW, open: false, at: 0 };
}

function persistSessionReviewMemory(sessionId: string, memory: Omit<SessionReviewMemory, "at">) {
  const id = reviewMemoryId(sessionId);
  if (!id) return;
  try {
    const all = readReviewMemoryMap();
    all[id] = {
      view: normalizeReviewView(memory.view),
      open: memory.open,
      at: Date.now(),
    };
    const ids = Object.keys(all);
    if (ids.length > MAX_REVIEW_MEMORY_ENTRIES) {
      ids
        .sort((left, right) => (all[left]?.at ?? 0) - (all[right]?.at ?? 0))
        .slice(0, ids.length - MAX_REVIEW_MEMORY_ENTRIES)
        .forEach((extra) => {
          if (extra !== id) delete all[extra];
        });
    }
    localStorage.setItem(REVIEW_MEMORY_STORAGE_KEY, JSON.stringify(all));
  } catch {
    /* private mode / quota */
  }
}

export interface UseWorkbenchReviewOptions {
  navigationOpen: Ref<boolean>;
  navigationWidth?: Ref<number>;
  activeSessionId: Ref<string>;
  messages: ComputedRef<ChatMessage[]>;
  sessions: Ref<ChatSessionSummary[]>;
  labels: WorkbenchLabels["labels"];
  clearSessionUnread: (sessionId: string) => void;
  selectConversation: (sessionId: string) => void | Promise<void>;
}

/**
 * Review sidebar (diff / runtime / image tabs), its resizable width, and image
 * preview bookkeeping.
 */
export function useWorkbenchReview(options: UseWorkbenchReviewOptions) {
  const {
    navigationOpen,
    activeSessionId,
    messages,
    sessions,
    labels,
    clearSessionUnread,
    selectConversation,
  } = options;
  const settingStore = useSettingStore();
  const subagentSessionStore = useSubagentSessionStore();

  const reviewOpen = ref(false);
  const reviewView = ref<string>(DEFAULT_REVIEW_VIEW);
  const imageLightboxOpen = ref(false);
  const openedImageSources = ref<string[]>([]);
  const selectedImageSource = ref("");
  const reviewFocusPath = ref("");
  const reviewFocusAt = ref(0);
  const navigationWidth = options.navigationWidth ?? ref(readStoredNavigationSidebarWidth());
  const reviewWidth = ref(readStoredReviewSidebarWidth());

  const {
    navigationResizing,
    startResize: startNavigationResize,
    handleResizeKey: handleNavigationResizeKey,
    resetWidth: resetNavigationWidth,
    updateWidth: updateNavigationWidth,
  } = useNavigationSidebarResize({
    navigationOpen,
    reviewOpen,
    reviewWidth,
    navigationWidth,
  });

  const {
    reviewResizing,
    startResize: startReviewResize,
    handleResizeKey: handleReviewResizeKey,
    resetWidth: resetReviewWidth,
    updateWidth: updateReviewWidth,
  } = useReviewSidebarResize({
    navigationOpen,
    reviewOpen,
    navigationWidth,
    reviewWidth,
  });

  const allToolActivities = computed(() =>
    messages.value.flatMap((message) => message.toolActivities ?? []),
  );
  const subagentActivities = computed(() =>
    allToolActivities.value.filter((activity) => SUBAGENT_TOOLS.has(activity.toolName)),
  );
  const runningSubagentCount = computed(
    () => subagentActivities.value.filter((activity) => activity.status === "running").length,
  );

  const codeReadPreview = useCodeReadPreviewStore();
  const reviewViews = computed(() => [
    ...(codeReadPreview.selection
      ? [{ id: "code", label: tr(settingStore.language, "readCodeOpen"), icon: FileCode2 }]
      : []),
    { id: "diff" as const, label: labels.value.diff, icon: FileDiff },
    { id: "plan" as const, label: tr(settingStore.language, "planProposalTitle"), icon: FileText },
    ...(import.meta.env.DEV
      ? [{ id: "runtime" as const, label: labels.value.runtime, icon: Bug }]
      : []),
  ]);

  function openReview(view: ReviewView | string) {
    if (view === "agents") {
      reviewView.value = "diff";
    } else {
      reviewView.value = view;
    }
    reviewOpen.value = true;
    updateReviewWidth();
    updateNavigationWidth();
  }

  function openReviewFile(path: string) {
    reviewFocusPath.value = path;
    reviewFocusAt.value += 1;
    openReview("diff");
  }

  function toggleReviewSidebar() {
    if (reviewOpen.value) {
      reviewOpen.value = false;
      return;
    }

    reviewOpen.value = true;
    updateReviewWidth();
    updateNavigationWidth();
  }

  function openAgentReview(entryId: string) {
    const parentId = rootSessionId(activeSessionId.value);
    const entry = findSubagentEntry(allToolActivities.value, entryId, settingStore.language);
    if (!entry) return;

    const sessionId = resolvePanelSessionId(parentId, allToolActivities.value, entry.entryId);
    const parentSummary = sessions.value.find((session) => session.sessionId === parentId);

    subagentSessionStore.upsert({
      sessionId,
      parentSessionId: parentId,
      entryId: entry.entryId,
      preview: entry.title,
      workspaceId: parentSummary?.workspaceId ?? null,
      visible: true,
    });

    if (!sharedSubagentIds.value.includes(entryId))
      sharedSubagentIds.value = [...sharedSubagentIds.value, entryId];
    sharedSelectedSubagent.value = entryId;
    sharedView.value = "subagents";
    void selectConversation(sessionId);
  }

  function previewImage(source: string) {
    if (!openedImageSources.value.includes(source)) {
      openedImageSources.value = [...openedImageSources.value, source];
    }
    selectedImageSource.value = source;
    imageLightboxOpen.value = true;
  }

  function closeImageLightbox() {
    imageLightboxOpen.value = false;
  }

  function closeImageTab(source: string) {
    const index = openedImageSources.value.indexOf(source);
    if (index < 0) return;

    const remaining = openedImageSources.value.filter((item) => item !== source);
    openedImageSources.value = remaining;
    if (selectedImageSource.value === source) {
      selectedImageSource.value = remaining[index] ?? remaining[index - 1] ?? "";
    }
    if (!remaining.length) {
      imageLightboxOpen.value = false;
    }
  }

  watch(
    activeSessionId,
    (sessionId) => {
      clearSessionUnread(sessionId);
      const memory = readSessionReviewMemory(sessionId);
      reviewView.value = memory.view;
      reviewOpen.value = memory.open;
      if (memory.open) {
        updateReviewWidth();
        updateNavigationWidth();
      }
    },
    { immediate: true },
  );

  watch(navigationWidth, () => {
    if (reviewOpen.value) updateReviewWidth();
  });

  watch(reviewWidth, () => {
    if (navigationOpen.value) updateNavigationWidth();
  });

  watch([reviewView, reviewOpen], () => {
    persistSessionReviewMemory(activeSessionId.value, {
      view: reviewView.value,
      open: reviewOpen.value,
    });
  });

  onMounted(() => {
    if (!reviewOpen.value) return;
    updateReviewWidth();
    updateNavigationWidth();
  });

  const sharedSubagentIds = ref<string[]>([]);
  const sharedSelectedSubagent = ref("");
  const sharedView = ref("");
  const sharedReview = useSharedReviewState(
    () => activeSessionId.value,
    {
      images: openedImageSources,
      selectedImage: selectedImageSource,
      imageOpen: imageLightboxOpen,
      subagents: sharedSubagentIds,
      selectedSubagent: sharedSelectedSubagent,
      view: sharedView,
    },
    (state) => {
      if (state.subagents) {
        for (const record of subagentSessionStore.visibleForParent(
          rootSessionId(activeSessionId.value),
        )) {
          if (!state.subagents.includes(record.entryId))
            subagentSessionStore.hide(record.sessionId);
        }
      }
      for (const entryId of state.subagents ?? []) {
        const parentId = rootSessionId(activeSessionId.value);
        const entry = findSubagentEntry(allToolActivities.value, entryId, settingStore.language);
        if (!entry) continue;
        const sessionId = resolvePanelSessionId(parentId, allToolActivities.value, entry.entryId);
        subagentSessionStore.upsert({
          sessionId,
          parentSessionId: parentId,
          entryId: entry.entryId,
          preview: entry.title,
          workspaceId:
            sessions.value.find((session) => session.sessionId === parentId)?.workspaceId ?? null,
          visible: true,
        });
      }
      if (
        state.view === "subagents" &&
        state.selectedSubagent &&
        activeSessionId.value === rootSessionId(activeSessionId.value)
      )
        openAgentReview(state.selectedSubagent);
      else if (state.view && ["diff", "plan", "runtime"].includes(state.view))
        reviewView.value = state.view;
    },
  );
  watch(
    () => allToolActivities.value.map((item) => item.id).join("|"),
    () => sharedReview.load(),
  );
  watch([reviewView, reviewOpen], () => {
    if (reviewOpen.value)
      sharedView.value =
        activeSessionId.value !== rootSessionId(activeSessionId.value)
          ? "subagents"
          : reviewView.value;
  });
  watch(imageLightboxOpen, (open) => {
    if (open) sharedView.value = "image";
  });
  watch(
    () =>
      subagentSessionStore
        .visibleForParent(rootSessionId(activeSessionId.value))
        .map((record) => record.entryId),
    (ids) => {
      if (JSON.stringify(ids) !== JSON.stringify(sharedSubagentIds.value))
        sharedSubagentIds.value = ids;
      if (!ids.includes(sharedSelectedSubagent.value))
        sharedSelectedSubagent.value = ids[ids.length - 1] ?? "";
    },
  );
  return {
    reviewOpen,
    reviewView,
    reviewViews,
    reviewWidth,
    reviewResizing,
    startReviewResize,
    handleReviewResizeKey,
    resetReviewWidth,
    updateReviewWidth,
    navigationWidth,
    navigationResizing,
    startNavigationResize,
    handleNavigationResizeKey,
    resetNavigationWidth,
    updateNavigationWidth,
    allToolActivities,
    subagentActivities,
    runningSubagentCount,
    imageLightboxOpen,
    openedImageSources,
    selectedImageSource,
    reviewFocusPath,
    reviewFocusAt,
    openReview,
    openReviewFile,
    toggleReviewSidebar,
    openAgentReview,
    previewImage,
    closeImageLightbox,
    closeImageTab,
  };
}
