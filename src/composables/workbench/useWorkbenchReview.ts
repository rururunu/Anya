import { computed, ref, watch, type ComputedRef, type Ref } from "vue";
import { Bug, FileDiff, Image as ImageIcon, Workflow } from "@lucide/vue";

import {
  useReviewSidebarResize,
  REVIEW_RESIZE_HANDLE_WIDTH,
  REVIEW_SIDEBAR_MIN_WIDTH,
  REVIEW_SIDEBAR_MAX_WIDTH,
} from "@/composables/useReviewSidebarResize";
import { tr } from "@/services/i18n";
import { SUBAGENT_TOOLS } from "@/services/chat/subagentTools";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage } from "@/types/chat";
import type { ReviewView } from "./types";
import type { WorkbenchLabels } from "./useWorkbenchLabels";

export { REVIEW_RESIZE_HANDLE_WIDTH, REVIEW_SIDEBAR_MIN_WIDTH, REVIEW_SIDEBAR_MAX_WIDTH };

export interface UseWorkbenchReviewOptions {
  navigationOpen: Ref<boolean>;
  activeSessionId: Ref<string>;
  messages: ComputedRef<ChatMessage[]>;
  labels: WorkbenchLabels["labels"];
  clearSessionUnread: (sessionId: string) => void;
}

/**
 * Review sidebar (diff / sub-agents / runtime / image tabs), its resizable
 * width, and the sub-agent / image tab bookkeeping that feeds it.
 */
export function useWorkbenchReview(options: UseWorkbenchReviewOptions) {
  const { navigationOpen, activeSessionId, messages, labels, clearSessionUnread } = options;
  const settingStore = useSettingStore();

  const reviewOpen = ref(false);
  const reviewView = ref<ReviewView>("diff");
  const openedSubagentIds = ref<string[]>([]);
  const selectedSubagentId = ref("");
  const openedImageSources = ref<string[]>([]);
  const selectedImageSource = ref("");
  const reviewFocusPath = ref("");
  const reviewFocusAt = ref(0);

  const {
    reviewWidth,
    reviewResizing,
    startResize: startReviewResize,
    handleResizeKey: handleReviewResizeKey,
    resetWidth: resetReviewWidth,
    updateWidth: updateReviewWidth,
  } = useReviewSidebarResize({ navigationOpen, reviewOpen });

  const allToolActivities = computed(() =>
    messages.value.flatMap((message) => message.toolActivities ?? []),
  );
  const subagentActivities = computed(() =>
    allToolActivities.value.filter((activity) => SUBAGENT_TOOLS.has(activity.toolName)),
  );
  const runningSubagentCount = computed(
    () => subagentActivities.value.filter((activity) => activity.status === "running").length,
  );

  // The runtime/debug panel is a development aid; keep it out of packaged builds.
  const reviewViews = computed(() => [
    { id: "diff" as const, label: labels.value.diff, icon: FileDiff },
    { id: "agents" as const, label: labels.value.agents, icon: Workflow },
    ...(openedImageSources.value.length
      ? [
          {
            id: "image" as const,
            label: tr(settingStore.language, "image.preview"),
            icon: ImageIcon,
          },
        ]
      : []),
    ...(import.meta.env.DEV
      ? [{ id: "runtime" as const, label: labels.value.runtime, icon: Bug }]
      : []),
  ]);

  function openReview(view: ReviewView) {
    reviewView.value = view;
    reviewOpen.value = true;
    updateReviewWidth();
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

    if (reviewView.value === "agents" && !openedSubagentIds.value.length) {
      reviewView.value = "diff";
    }
    reviewOpen.value = true;
    updateReviewWidth();
  }

  function openAgentReview(activityId: string) {
    if (!openedSubagentIds.value.includes(activityId)) openedSubagentIds.value.push(activityId);
    selectedSubagentId.value = activityId;
    openReview("agents");
  }

  function closeSubagent(activityId: string) {
    openedSubagentIds.value = openedSubagentIds.value.filter((id) => id !== activityId);
    if (selectedSubagentId.value === activityId) {
      selectedSubagentId.value = openedSubagentIds.value[openedSubagentIds.value.length - 1] ?? "";
    }
  }

  function previewImage(source: string) {
    if (!openedImageSources.value.includes(source)) {
      openedImageSources.value = [...openedImageSources.value, source];
    }
    selectedImageSource.value = source;
    openReview("image");
  }

  function closeImageTab(source: string) {
    const index = openedImageSources.value.indexOf(source);
    if (index < 0) return;

    const remaining = openedImageSources.value.filter((item) => item !== source);
    openedImageSources.value = remaining;
    if (selectedImageSource.value === source) {
      selectedImageSource.value = remaining[index] ?? remaining[index - 1] ?? "";
    }
    if (!remaining.length && reviewView.value === "image") {
      reviewView.value = "diff";
    }
  }

  watch(activeSessionId, () => {
    clearSessionUnread(activeSessionId.value);
    openedSubagentIds.value = [];
    selectedSubagentId.value = "";
    openedImageSources.value = [];
    selectedImageSource.value = "";
    if (reviewView.value === "image") reviewView.value = "diff";
  });

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
    allToolActivities,
    subagentActivities,
    runningSubagentCount,
    openedSubagentIds,
    selectedSubagentId,
    openedImageSources,
    selectedImageSource,
    reviewFocusPath,
    reviewFocusAt,
    openReview,
    openReviewFile,
    toggleReviewSidebar,
    openAgentReview,
    closeSubagent,
    previewImage,
    closeImageTab,
  };
}
