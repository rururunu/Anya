<template>
  <div class="message-list-shell" :class="{ 'has-find': findOpen }">
    <div
      v-if="findOpen"
      class="conversation-find-bar"
      role="search"
      :aria-label="tr(settingStore.language, 'findInConversation')"
    >
      <input
        ref="findInputRef"
        v-model="findQuery"
        class="conversation-find-input"
        type="text"
        :placeholder="tr(settingStore.language, 'findPlaceholder')"
        spellcheck="false"
        autocomplete="off"
        @keydown="onFindInputKeydown"
      />
      <span
        class="conversation-find-count"
        :class="{ empty: Boolean(findQuery.trim()) && findHits.length === 0 }"
      >
        {{ findCountLabel }}
      </span>
      <button
        type="button"
        class="conversation-find-btn"
        :title="tr(settingStore.language, 'findPrevious')"
        :aria-label="tr(settingStore.language, 'findPrevious')"
        :disabled="findHits.length === 0"
        @click="prevFind"
      >
        <ChevronUp :size="14" :stroke-width="2" />
      </button>
      <button
        type="button"
        class="conversation-find-btn"
        :title="tr(settingStore.language, 'findNext')"
        :aria-label="tr(settingStore.language, 'findNext')"
        :disabled="findHits.length === 0"
        @click="nextFind"
      >
        <ChevronDown :size="14" :stroke-width="2" />
      </button>
      <button
        type="button"
        class="conversation-find-btn"
        :title="tr(settingStore.language, 'findClose')"
        :aria-label="tr(settingStore.language, 'findClose')"
        @click="closeFind"
      >
        <X :size="14" :stroke-width="2" />
      </button>
    </div>

    <nav
      v-if="userMessages.length"
      ref="railRef"
      class="message-preview-rail"
      tabindex="0"
      :aria-label="tr(settingStore.language, 'userMessageNav')"
      @keydown="onRailKeydown"
    >
      <button
        v-for="(message, index) in userMessages"
        :key="message.id"
        type="button"
        class="message-preview-mark"
        :class="{ active: message.id === activeUserMessageId }"
        :aria-label="tr(settingStore.language, 'jumpMessage', { count: index + 1 })"
        :aria-current="message.id === activeUserMessageId ? 'true' : undefined"
        @click="scrollToMessage(message.id)"
      >
        <span class="mark-line" aria-hidden="true"></span>
        <span class="message-preview-tooltip">{{ messagePreview(message) }}</span>
      </button>
    </nav>

    <Transition name="scroll-to-bottom">
      <button
        v-if="!stickToBottom && displayItems.length"
        type="button"
        class="scroll-to-bottom"
        :aria-label="tr(settingStore.language, 'scrollToBottom')"
        @click="scrollToLatest"
      >
        <ArrowDown :size="16" :stroke-width="1.75" />
      </button>
    </Transition>

    <div
      ref="listRef"
      class="message-list peek-scrollbar"
      data-tauri-drag-region="false"
      @scroll="handleScroll"
    >
      <div v-if="displayItems.length === 0" class="empty-thread">
        {{ emptyThreadPrompt }}
      </div>
      <article
        v-for="item in displayItems"
        :key="item.key"
        class="message-item"
        :class="item.kind"
        :data-message-id="item.message.id"
        v-memo="messageMemoDeps(item)"
      >
        <div v-if="item.kind === 'user'" class="user-turn">
          <div
            v-if="userContent(item.message).images?.length"
            class="user-images"
            data-tauri-drag-region="false"
          >
            <button
              v-for="(img, idx) in userContent(item.message).images"
              :key="idx"
              type="button"
              class="user-image-btn"
              data-tauri-drag-region="false"
              data-no-drag
              :aria-label="'Preview image'"
              @mousedown.stop
              @click.stop.prevent="previewImage(img)"
            >
              <img :src="img" class="user-image" alt="" draggable="false" />
            </button>
          </div>
          <div
            v-if="userContent(item.message).attachedFiles?.length"
            class="user-attached-files"
            data-tauri-drag-region="false"
          >
            <div
              v-for="(file, idx) in userContent(item.message).attachedFiles"
              :key="`${file.path}-${idx}`"
              class="user-file-chip"
              :class="{ skipped: Boolean(file.skipped) }"
              :title="file.skipped ? `${file.path} (${file.skipped})` : file.path"
            >
              <img
                v-if="fileIconForPath(file.path)"
                class="user-file-icon-img"
                :src="fileIconForPath(file.path) || ''"
                alt=""
              />
              <File v-else :size="12" :stroke-width="1.75" aria-hidden="true" />
              <span class="user-file-name">{{ file.name }}</span>
            </div>
          </div>
          <div
            v-if="userContent(item.message).message || userContent(item.message).selection"
            class="user-bubble"
          >
            <span v-if="userContent(item.message).message" class="user-message-text">
              <template
                v-for="(part, partIdx) in inlineMessageParts(userContent(item.message).message)"
                :key="`${item.message.id}-part-${partIdx}`"
              >
                <span
                  v-if="part.kind === 'mention'"
                  class="inline-token inline-token-mark inline-token-file"
                  :class="{ 'is-dir': part.isDir }"
                  :title="normalizeMentionPath(part.path)"
                >
                  <Folder
                    v-if="part.isDir"
                    :size="12"
                    class="inline-token-logo-fallback"
                    aria-hidden="true"
                  />
                  <img
                    v-else-if="fileIconForPath(part.path)"
                    class="inline-token-logo"
                    :src="fileIconForPath(part.path) || ''"
                    alt=""
                  />
                  <File v-else :size="12" class="inline-token-logo-fallback" aria-hidden="true" />
                  <span class="inline-token-label">@{{ part.name }}</span>
                </span>
                <span
                  v-else-if="part.kind === 'skill'"
                  class="inline-token inline-token-mark inline-token-skill"
                  :title="hashChipTitle('skill', part.id)"
                >
                  <img
                    v-if="hashChipIcon('skill', part.id)"
                    class="inline-token-logo"
                    :src="hashChipIcon('skill', part.id) || ''"
                    alt=""
                    referrerpolicy="no-referrer"
                    @error="markHashIconBroken('skill', part.id)"
                  />
                  <Zap v-else :size="12" class="inline-token-logo-fallback" aria-hidden="true" />
                  <span class="inline-token-label">{{ hashChipLabel("skill", part.id) }}</span>
                </span>
                <span
                  v-else-if="part.kind === 'mcp'"
                  class="inline-token inline-token-mark inline-token-mcp"
                  :title="hashChipTitle('mcp', part.id)"
                >
                  <img
                    v-if="hashChipIcon('mcp', part.id)"
                    class="inline-token-logo"
                    :src="hashChipIcon('mcp', part.id) || ''"
                    alt=""
                    referrerpolicy="no-referrer"
                    @error="markHashIconBroken('mcp', part.id)"
                  />
                  <Bot v-else :size="12" class="inline-token-logo-fallback" aria-hidden="true" />
                  <span class="inline-token-label">{{ hashChipLabel("mcp", part.id) }}</span>
                </span>
                <template v-else>{{ part.text }}</template>
              </template>
            </span>
            <span v-if="userContent(item.message).selection" class="user-selection-quote">
              {{ userContent(item.message).selection }}
            </span>
          </div>
          <div
            v-if="copyableUserText(item.message) || checkpointFor(item.message)"
            class="message-actions user-message-actions"
          >
            <button
              v-if="copyableUserText(item.message)"
              type="button"
              class="message-action-btn"
              :class="copyButtonClass(item.message.id)"
              :aria-label="copyButtonLabel(item.message.id)"
              :title="copyButtonLabel(item.message.id)"
              @click.stop="copyMessage(item.message, 'user')"
            >
              <Check
                v-if="copyStatus?.id === item.message.id && copyStatus.state === 'copied'"
                :size="14"
                :stroke-width="2"
                aria-hidden="true"
              />
              <Copy v-else :size="14" :stroke-width="2" aria-hidden="true" />
            </button>
            <button
              v-if="checkpointFor(item.message)"
              type="button"
              class="message-action-btn"
              :disabled="rewindBusy"
              :aria-label="tr(settingStore.language, 'rewind')"
              :title="tr(settingStore.language, 'rewind')"
              @click.stop="confirmRewind(item.message)"
            >
              <Undo2 :size="14" :stroke-width="2" aria-hidden="true" />
            </button>
          </div>
        </div>
        <div v-else class="assistant-bubble">
          <AgentWorkDetails
            :message="item.message"
            :language="settingStore.language"
            :show-reasoning="settingStore.showReasoning"
            :display-mode="settingStore.agentWorkDisplay"
            :suppress-content="needsProviderSetup(item.message)"
            @inspect-subagent="emit('inspectSubagent', $event)"
            @preview-image="emit('previewImage', $event)"
            @edit-from-image="emit('editFromImage', $event)"
          />
          <AskUserAnswerCard
            v-if="item.message.askUserAnswer?.length"
            :items="item.message.askUserAnswer"
          />
          <ImageAnalysisDetails
            v-for="(analysis, idx) in imageAnalysesForAssistant(item.message)"
            :key="`${item.message.id}-analysis-${idx}`"
            :model="analysis.model"
            :text="analysis.text"
          />
          <EnvironmentContextCard
            v-if="item.message.environmentContext"
            :context="item.message.environmentContext"
          />
          <div v-else-if="needsProviderSetup(item.message)" class="provider-setup-card">
            <p class="provider-setup-text">
              {{ providerSetupText(item.message) }}
            </p>
            <button type="button" class="provider-setup-btn" @click="openProviderSettings">
              {{ tr(settingStore.language, "configureProviderAction") }}
            </button>
          </div>
          <div v-if="item.injects.length" class="soft-inject-list">
            <div
              v-for="inject in item.injects"
              :key="inject.id"
              class="soft-inject-chip"
              :data-message-id="inject.id"
            >
              <span class="soft-inject-label">{{ tr(settingStore.language, "softInjected") }}</span>
              <span class="soft-inject-text">{{ softInjectText(inject) }}</span>
            </div>
          </div>
          <AssistantActivityIndicator
            v-if="activityLabel(item.message)"
            :label="activityLabel(item.message)!"
            :icon="activityIcon(item.message)"
          />
          <CodeChangesSummary
            v-if="item.message.status === 'done'"
            :message="item.message"
            :can-undo="Boolean(checkpointForAssistant(item.message))"
            :busy="rewindBusy"
            @undo="confirmAssistantRewind(item.message)"
            @review="$emit('reviewChanges')"
            @review-file="$emit('reviewFile', $event)"
          />
          <SharedOfferCards :message="item.message" />
          <PlanApprovalCard
            v-if="showPlanCardFor(item.message)"
            :tasks="planTasksForMessage(item.message, isApprovedPlan(item.message))"
            :busy="planBusy || isSessionSending"
            :executing="isApprovedPlan(item.message)"
            :auto-countdown="isApprovedPlan(item.message) ? null : planCountdownInfo"
            :allow-empty-approve="isPlanGateStopMessage(item.message)"
            @approve="approvePlanMode"
            @reject="rejectAutoExecute"
          />
          <div
            v-if="
              item.message.content.trim() ||
              processingDuration(item.message) ||
              turnTokenCount(item) ||
              turnCacheHit(item) != null ||
              canBranchMessage(item.message)
            "
            class="message-actions assistant-message-actions"
          >
            <span v-if="processingDuration(item.message)" class="processing-duration">
              {{
                tr(settingStore.language, "processedFor", {
                  duration: processingDuration(item.message)!,
                })
              }}
            </span>
            <span
              v-if="turnTokenCount(item)"
              class="token-usage"
              :title="tokenEstimateTitle(turnTokenCount(item))"
            >
              ≈ {{ formatTokenCount(turnTokenCount(item), settingStore.language) }} tokens
            </span>
            <span
              v-if="turnCacheHit(item) != null"
              class="cache-hit"
              :title="turnCacheHitTitle(item)"
            >
              {{
                tr(settingStore.language, "tokens.cacheHit", {
                  percent: turnCacheHit(item) ?? 0,
                })
              }}
            </span>
            <button
              v-if="item.message.content.trim()"
              type="button"
              class="message-action-btn"
              :class="copyButtonClass(item.message.id)"
              :aria-label="copyButtonLabel(item.message.id)"
              :title="copyButtonLabel(item.message.id)"
              @click.stop="copyMessage(item.message, 'assistant')"
            >
              <Check
                v-if="copyStatus?.id === item.message.id && copyStatus.state === 'copied'"
                :size="14"
                :stroke-width="2"
                aria-hidden="true"
              />
              <Copy v-else :size="14" :stroke-width="2" aria-hidden="true" />
            </button>
            <button
              v-if="canBranchMessage(item.message)"
              type="button"
              class="message-action-btn"
              :aria-label="tr(settingStore.language, 'branchConversation')"
              :title="tr(settingStore.language, 'branchConversation')"
              @click.stop="branchFromMessage(item)"
            >
              <GitBranch :size="14" :stroke-width="2" aria-hidden="true" />
            </button>
          </div>
        </div>
      </article>
    </div>

    <AppConfirmDialog ref="confirmDialogRef" />
  </div>
</template>

<script setup lang="ts">
import {
  computed,
  nextTick,
  onMounted,
  onUnmounted,
  reactive,
  ref,
  watch,
  type Component,
} from "vue";
import {
  ArrowDown,
  Bot,
  Check,
  ChevronDown,
  ChevronUp,
  Copy,
  File,
  Folder,
  GitBranch,
  Paintbrush,
  Undo2,
  X,
  Zap,
} from "@lucide/vue";
import { codeLanguageForPath } from "@/services/chat/codeLanguage";
import { normalizeMentionPath } from "@/services/chat/composerSegments";
import { splitInlineTokenParts } from "@/services/chat/inlineTokenMarks";
import "@/services/chat/inlineTokenMarks.css";
import {
  mcpMentionIconUrl,
  mcpMentionLabel,
  prettyHashInstallId,
  skillMentionIconUrl,
  skillMentionLabel,
} from "@/services/chat/hashMentionDisplay";
import { lookupInstallIcon, peekInstallIcon, warmInstallIcons } from "@/services/iconCache";
import AgentWorkDetails from "@/components/chat/AgentWorkDetails.vue";
import CodeChangesSummary from "@/components/chat/CodeChangesSummary.vue";
import SharedOfferCards from "@/components/chat/SharedOfferCards.vue";
import PlanApprovalCard from "@/components/chat/PlanApprovalCard.vue";
import AssistantActivityIndicator from "@/components/chat/AssistantActivityIndicator.vue";
import AskUserAnswerCard from "@/components/chat/AskUserAnswerCard.vue";
import ImageAnalysisDetails from "@/components/chat/ImageAnalysisDetails.vue";
import EnvironmentContextCard from "@/components/chat/EnvironmentContextCard.vue";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import { openSettings as ipcOpenSettings, rewindSession, setPlanMode } from "@/services/ipc";
import { useSettingStore } from "@/stores/setting";
import { useChatStore } from "@/stores/chat";
import type { ChatMessage, CheckpointInfo, TaskItem } from "@/types/chat";
import { parseSelectionAttachment } from "@/services/chat/selectionAttachment";
import { isSoftInjectContent, stripSoftInjectMarker } from "@/services/chat/softInject";
import { isCompactionSummary } from "@/services/chat/compactMarker";
import { tr } from "@/services/i18n";
import { createLogger } from "@/services/logger";
import { gsapScrollContainerTo } from "@/services/motion/gsapPresets";
import { copyText } from "@/services/clipboard";
import {
  estimateMessageTokens,
  formatTokenCount,
  promptCacheHitPercent,
  promptTokenTotal,
} from "@/services/chat/tokenEstimate";
import { isConfigureProviderError } from "@/services/chat/ensureDefaultModel";
import {
  applyFindHits,
  clearFindHits,
  paintCurrentFindHit,
} from "@/services/chat/conversationFind";
import { provideConversationFind } from "@/composables/chat/useConversationFind";
import { useAppStore } from "@/stores/app";
import { storeToRefs } from "pinia";

type DisplayItem =
  | { kind: "user"; key: string; message: ChatMessage }
  | { kind: "assistant"; key: string; message: ChatMessage; injects: ChatMessage[] };
function previewImage(url: string) {
  emit("previewImage", url);
}

const SCROLL_NEAR_BOTTOM_THRESHOLD = 96;
const props = defineProps<{
  messages: ChatMessage[];
  sessionId?: string;
  workspaceName?: string;
  checkpoints?: CheckpointInfo[];
}>();
const workspaceName = computed(() => props.workspaceName?.trim() || "");
const emptyThreadPrompt = computed(() =>
  workspaceName.value
    ? tr(settingStore.language, "emptyWorkspaceThread", { workspace: workspaceName.value })
    : tr(settingStore.language, "emptyThreadGeneral"),
);
const emit = defineEmits<{
  rewound: [payload: { text: string }];
  branch: [messageId: string];
  reviewChanges: [];
  reviewFile: [path: string];
  inspectSubagent: [activityId: string];
  previewImage: [source: string];
  editFromImage: [payload: import("@/services/chat/imageEditReference").ImageEditReferencePayload];
}>();
const settingStore = useSettingStore();
const appStore = useAppStore();
const chatStore = useChatStore();
const log = createLogger("message-list");
const { sending } = storeToRefs(chatStore);
const planBusy = ref(false);

/** Auto-execute window for auto-entered plans (agent-mode complexity detection). */
const AUTO_EXECUTE_SECONDS = 30;
const planCountdown = ref<number | null>(null);
let planTimer: ReturnType<typeof setInterval> | null = null;
function clearPlanTimer() {
  if (planTimer) {
    clearInterval(planTimer);
    planTimer = null;
  }
}
const isSessionSending = computed(() => Boolean(props.sessionId && sending.value[props.sessionId]));
/** Plan message that was approved — keep the checklist visible while it runs. */
const approvedPlanMessageId = ref<string | null>(null);
/** Plan message whose auto-execute countdown was rejected — keep waiting for manual approve. */
const rejectedAutoExecuteMessageId = ref<string | null>(null);

watch(
  () => props.sessionId,
  () => {
    approvedPlanMessageId.value = null;
    rejectedAutoExecuteMessageId.value = null;
  },
);

const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const brokenHashIcons = reactive<Record<string, boolean>>({});
const resolvedHashIcons = reactive<Record<string, string>>({});

function hashChipKey(kind: "skill" | "mcp", id: string) {
  return `${kind}:${id}`;
}

function hashChipLabel(kind: "skill" | "mcp", id: string): string {
  if (kind === "mcp") return mcpMentionLabel(id, settingStore.mcpServers ?? []);
  return skillMentionLabel(id);
}

function hashChipTitle(kind: "skill" | "mcp", id: string): string {
  if (kind === "mcp") {
    const server = (settingStore.mcpServers ?? []).find((item) => item.id === id);
    return server?.qualifiedName?.trim() || prettyHashInstallId(id);
  }
  return prettyHashInstallId(id);
}

function hashChipIcon(kind: "skill" | "mcp", id: string): string | null {
  const key = hashChipKey(kind, id);
  if (brokenHashIcons[key]) return null;
  if (resolvedHashIcons[key]) return resolvedHashIcons[key];
  const sync =
    kind === "mcp" ? mcpMentionIconUrl(id, settingStore.mcpServers ?? []) : skillMentionIconUrl(id);
  if (sync) {
    resolvedHashIcons[key] = sync;
    return sync;
  }
  void lookupInstallIcon(kind, id).then((local) => {
    if (local && !brokenHashIcons[key]) resolvedHashIcons[key] = local;
  });
  return null;
}

function markHashIconBroken(kind: "skill" | "mcp", id: string) {
  brokenHashIcons[hashChipKey(kind, id)] = true;
}

function warmHashIcons() {
  const servers = settingStore.mcpServers ?? [];
  void warmInstallIcons(
    servers.map((server) => ({
      kind: "mcp" as const,
      cacheKey: server.id,
      url: server.iconUrl,
    })),
  ).then(() => {
    for (const server of servers) {
      const local = peekInstallIcon("mcp", server.id);
      const key = hashChipKey("mcp", server.id);
      if (local && !brokenHashIcons[key]) resolvedHashIcons[key] = local;
    }
  });
}

watch(
  () => settingStore.mcpServers,
  () => warmHashIcons(),
  { deep: true, immediate: true },
);

function needsProviderSetup(message: ChatMessage): boolean {
  return message.status === "error" && isConfigureProviderError(message.content);
}

function providerSetupText(_message: ChatMessage): string {
  return tr(settingStore.language, "configureProviderHint");
}

function openProviderSettings() {
  appStore.openSettings("provider");
  void ipcOpenSettings().catch(() => {
    // Workbench may already be focused; app-store signal still opens settings.
  });
}

const planModeActive = computed(() =>
  Boolean(props.sessionId && chatStore.sessionPlanMode[props.sessionId]),
);

/** Stable plan identity: content + nesting only (ignore status churn). */
function planStructureFingerprint(tasks: TaskItem[]): string {
  return tasks
    .map(
      (task) =>
        `${typeof task.level === "number" ? task.level : 0}\t${String(task.content ?? "").trim()}`,
    )
    .join("\n");
}

function pendingPlanFingerprint(message: ChatMessage | undefined): string | null {
  if (!message || !looksLikePendingPlan(message)) return null;
  const tasks = tasksFromMessage(message);
  if (!tasks.length) return null;
  return planStructureFingerprint(tasks);
}

function clearRejectedPlanIfUpdated(fingerprint: string | null) {
  if (!props.sessionId || !fingerprint) return false;
  const rejected = chatStore.rejectedPlanFingerprint(props.sessionId);
  if (!rejected || rejected === fingerprint) return false;
  // New or revised checklist — allow auto-execute countdown again.
  chatStore.setSessionRejectedPlanFingerprint(props.sessionId, null);
  rejectedAutoExecuteMessageId.value = null;
  return true;
}

const planAutoActive = computed(() => {
  if (!props.sessionId) return false;
  if (!planModeActive.value) return false;
  if (isSessionSending.value) return false;
  // Explicit Plan mode always waits for the user.
  if (chatStore.sessionCompose[props.sessionId]?.chatMode === "plan") return false;

  const messageId = lastDoneAssistantId.value;
  if (!messageId || messageId === approvedPlanMessageId.value) return false;
  if (hasUserMessageAfter(messageId)) return false;
  const message = props.messages.find((item) => item.id === messageId);
  const fingerprint = pendingPlanFingerprint(message);
  if (!fingerprint) return false;

  const rejected = chatStore.rejectedPlanFingerprint(props.sessionId);
  // Same rejected checklist: keep waiting for manual approve (no countdown).
  if (rejected && rejected === fingerprint) return false;
  if (messageId === rejectedAutoExecuteMessageId.value && rejected === fingerprint) return false;

  // New/updated plan after a reject may still have trigger=manual until we re-arm.
  if (chatStore.sessionPlanTrigger[props.sessionId] === "auto") return true;
  return Boolean(rejected && rejected !== fingerprint);
});

const planCountdownInfo = computed<{ remaining: number; total: number } | null>(() =>
  planCountdown.value === null
    ? null
    : { remaining: planCountdown.value, total: AUTO_EXECUTE_SECONDS },
);

watch(
  planAutoActive,
  (active) => {
    clearPlanTimer();
    if (active) {
      planCountdown.value = AUTO_EXECUTE_SECONDS;
      planTimer = setInterval(() => {
        planCountdown.value = Math.max(0, (planCountdown.value ?? 0) - 0.1);
        if ((planCountdown.value ?? 0) <= 0) {
          clearPlanTimer();
          void approvePlanMode();
        }
      }, 100);
    } else {
      planCountdown.value = null;
    }
  },
  { immediate: true },
);
onUnmounted(clearPlanTimer);

const lastDoneAssistantId = computed(() => {
  for (let i = props.messages.length - 1; i >= 0; i -= 1) {
    const message = props.messages[i];
    if (!message) continue;
    if (String(message.role).toLowerCase() === "assistant" && message.status === "done") {
      return message.id;
    }
  }
  return null;
});

function tasksFromMessage(message: ChatMessage): TaskItem[] {
  const activities = message.toolActivities ?? [];
  for (let i = activities.length - 1; i >= 0; i -= 1) {
    const activity = activities[i];
    if (!activity) continue;
    if (activity.toolName !== "update_tasks" && activity.toolName !== "todo_write") continue;
    const raw = activity.arguments?.tasks;
    if (!Array.isArray(raw)) continue;
    const tasks = raw.flatMap((value) => {
      if (!value || typeof value !== "object") return [];
      const item = value as Record<string, unknown>;
      const content = String(item.content ?? "").trim();
      if (!content) return [];
      return [
        {
          content,
          status: String(item.status ?? "pending"),
          activeForm:
            typeof item.activeForm === "string"
              ? item.activeForm
              : typeof item.active_form === "string"
                ? item.active_form
                : undefined,
          level: typeof item.level === "number" ? item.level : undefined,
        } satisfies TaskItem,
      ];
    });
    if (tasks.length) return tasks;
  }
  return [];
}

function planTasksForMessage(message: ChatMessage, preferLive = false): TaskItem[] {
  if (preferLive && props.sessionId) {
    const live = chatStore.sessionTasks[props.sessionId];
    if (live?.length) return live;
  }
  const fromMessage = tasksFromMessage(message);
  if (fromMessage.length) return fromMessage;
  if (!props.sessionId) return [];
  return chatStore.sessionTasks[props.sessionId] ?? [];
}

function hasUserMessageAfter(messageId: string): boolean {
  const idx = props.messages.findIndex((message) => message.id === messageId);
  if (idx === -1) return false;
  for (let i = idx + 1; i < props.messages.length; i += 1) {
    const message = props.messages[i];
    if (message && String(message.role).toLowerCase() === "user") return true;
  }
  return false;
}

function isApprovedPlan(message: ChatMessage): boolean {
  return message.id === approvedPlanMessageId.value;
}

function isPlanGateStopMessage(message: ChatMessage): boolean {
  return message.content.includes("计划尚未批准");
}

function hasApprovablePlan(message: ChatMessage): boolean {
  return looksLikePendingPlan(message) || isPlanGateStopMessage(message);
}

/** A planning turn: task checklist on THIS message, without mutating tool work yet. */
function looksLikePendingPlan(message: ChatMessage): boolean {
  // Message-local only — sessionTasks would make every later reply look like a plan.
  const tasks = tasksFromMessage(message);
  if (!tasks.length) return false;
  const hadMutations = (message.toolActivities ?? []).some((activity) => {
    if (activity.success === false || activity.status === "error") return false;
    const kind = String(activity.kind ?? "").toLowerCase();
    // Shell may be used for read-only inspection while drafting a plan; only
    // successful file mutations mean this turn already started implementing.
    return kind === "edit" || kind === "create" || kind === "delete" || kind === "move";
  });
  if (hadMutations) return false;
  return tasks.some((task) => {
    const status = String(task.status ?? "pending").toLowerCase();
    return (
      status === "pending" ||
      status === "in_progress" ||
      status === "active" ||
      status === "running"
    );
  });
}

function showPlanCardFor(message: ChatMessage): boolean {
  if (message.status !== "done") return false;
  if (isApprovedPlan(message)) {
    return planTasksForMessage(message, true).length > 0;
  }
  if (message.id !== lastDoneAssistantId.value) return false;
  if (hasUserMessageAfter(message.id)) return false;
  // Plan mode by itself is not an approval request. Only show the card
  // when there is a checklist, or the writer gate stopped the turn.
  return hasApprovablePlan(message);
}

/** If Agent left a pending plan checklist but the gate never flipped, recover it. */
function ensurePlanGateForPendingChecklist() {
  if (!props.sessionId || planBusy.value || isSessionSending.value) return;
  const messageId = lastDoneAssistantId.value;
  if (!messageId || messageId === approvedPlanMessageId.value) return;
  if (hasUserMessageAfter(messageId)) return;
  const message = props.messages.find((item) => item.id === messageId);
  if (message && isPlanGateStopMessage(message) && !planModeActive.value) {
    chatStore.setSessionPlanMode(props.sessionId, true);
    void setPlanMode(props.sessionId, true).catch((error) => {
      log.warn("recover plan gate after writer block failed", error);
    });
    return;
  }
  // Manual Plan mode never auto-executes.
  if (chatStore.sessionCompose[props.sessionId]?.chatMode === "plan") return;
  const fingerprint = pendingPlanFingerprint(message);
  if (!fingerprint) return;

  const rejected = chatStore.rejectedPlanFingerprint(props.sessionId);
  // Still the same rejected checklist — keep manual approve only.
  if (rejected && rejected === fingerprint) {
    if (chatStore.sessionPlanTrigger[props.sessionId] === "auto") {
      chatStore.setSessionPlanTrigger(props.sessionId, "manual");
    }
    return;
  }

  // New or updated plan after a reject: clear the block and allow countdown.
  clearRejectedPlanIfUpdated(fingerprint);

  const armAuto = () => {
    chatStore.setSessionPlanMode(props.sessionId!, true);
    chatStore.setSessionPlanTrigger(props.sessionId!, "auto");
    void setPlanMode(props.sessionId!, true, "auto")
      .then(() => {
        chatStore.setSessionPlanMode(props.sessionId!, true);
        chatStore.setSessionPlanTrigger(props.sessionId!, "auto");
      })
      .catch((error) => {
        log.warn("recover plan gate for pending checklist failed", error);
      });
  };

  if (planModeActive.value) {
    if (chatStore.sessionPlanTrigger[props.sessionId] !== "auto") {
      armAuto();
    }
    return;
  }

  armAuto();
}

watch(
  () =>
    [
      props.sessionId,
      lastDoneAssistantId.value,
      isSessionSending.value,
      planModeActive.value,
      props.messages
        .map((message) => `${message.id}:${message.status}:${message.toolActivities?.length ?? 0}`)
        .join("|"),
      JSON.stringify(props.sessionId ? (chatStore.sessionTasks[props.sessionId] ?? []) : []),
    ] as const,
  () => {
    ensurePlanGateForPendingChecklist();
  },
);

async function approvePlanMode() {
  if (!props.sessionId || planBusy.value || sending.value?.[props.sessionId]) return;
  planBusy.value = true;
  clearPlanTimer();
  try {
    // Keep the checklist on this plan message; hide only the approve actions.
    if (lastDoneAssistantId.value) {
      approvedPlanMessageId.value = lastDoneAssistantId.value;
    }
    chatStore.setSessionRejectedPlanFingerprint(props.sessionId, null);
    rejectedAutoExecuteMessageId.value = null;
    // Ensure writers unlock even if the gate was recovered only on the frontend.
    if (!planModeActive.value) {
      chatStore.setSessionPlanMode(props.sessionId, true);
      await setPlanMode(props.sessionId, true).catch(() => undefined);
    }
    await chatStore.send(tr(settingStore.language, "planModeExecuteMessage"), props.sessionId, {
      skipAutoPlan: true,
      resumePlan: true,
    });
  } catch (error) {
    log.error("approve plan mode failed", error);
  } finally {
    planBusy.value = false;
  }
}

function rejectAutoExecute() {
  // Stop auto-run only — keep the plan checklist and manual approve available.
  // Remember this checklist structure so identical plans don't restart countdown;
  // a new/updated plan fingerprint will allow auto-execute again.
  clearPlanTimer();
  planCountdown.value = null;
  if (lastDoneAssistantId.value) {
    rejectedAutoExecuteMessageId.value = lastDoneAssistantId.value;
  }
  if (props.sessionId) {
    chatStore.setSessionPlanTrigger(props.sessionId, "manual");
    const message = props.messages.find((item) => item.id === lastDoneAssistantId.value);
    const fingerprint =
      pendingPlanFingerprint(message) ??
      (message ? planStructureFingerprint(tasksFromMessage(message)) : null);
    if (fingerprint) {
      chatStore.setSessionRejectedPlanFingerprint(props.sessionId, fingerprint);
    }
  }
}

const visibleMessages = computed(() =>
  props.messages.filter((message) => {
    if (isCompactionSummary(message)) return false;
    const role = String(message.role).toLowerCase();
    return role !== "system" && role !== "tool";
  }),
);

const displayItems = computed((): DisplayItem[] => {
  const items: DisplayItem[] = [];
  for (const message of visibleMessages.value) {
    if (isUserMessage(message)) {
      if (isSoftInjectMessage(message)) {
        let folded = false;
        for (let i = items.length - 1; i >= 0; i -= 1) {
          const item = items[i];
          if (item?.kind === "assistant") {
            item.injects.push(message);
            folded = true;
            break;
          }
        }
        // Mis-tagged first message (no prior assistant): show as a normal bubble.
        if (!folded) {
          items.push({ kind: "user", key: message.id, message });
        }
        continue;
      }
      items.push({ kind: "user", key: message.id, message });
      continue;
    }
    items.push({
      kind: "assistant",
      key: message.id,
      message,
      injects: [],
    });
  }
  return items;
});

/** Stable deps for v-memo — completed bubbles skip re-render when only siblings stream. */
function messageMemoDeps(item: DisplayItem) {
  const message = item.message;
  const live = message.status === "pending" || message.status === "streaming";
  const tools =
    message.toolActivities
      ?.map((activity) => `${activity.id}:${activity.status}:${activity.detail?.length ?? 0}`)
      .join(",") ?? "";
  const asks = message.askUserAnswer?.map((answer) => answer.selected.join(",")).join(";") ?? "";
  return [
    item.key,
    item.kind,
    message.status,
    message.content.length,
    message.reasoning?.length ?? 0,
    message.activityStatus ?? "",
    message.workTimeline?.length ?? 0,
    tools,
    asks,
    item.kind === "assistant" ? item.injects.map((inject) => inject.id).join(",") : "",
    copyStatus.value?.id === message.id ? copyStatus.value.state : "",
    rewindBusy.value ? 1 : 0,
    live ? durationClock.value : 0,
    showPlanCardFor(message) ? (planCountdownInfo.value?.remaining ?? -1) : -1,
    planBusy.value ? 1 : 0,
    isSessionSending.value ? 1 : 0,
    settingStore.showReasoning ? 1 : 0,
    settingStore.agentWorkDisplay,
    settingStore.language,
  ];
}

const userMessages = computed(() =>
  displayItems.value
    .filter((item): item is Extract<DisplayItem, { kind: "user" }> => item.kind === "user")
    .map((item) => item.message),
);
const listRef = ref<HTMLElement | null>(null);
const railRef = ref<HTMLElement | null>(null);
const findOpen = ref(false);
const findQuery = ref("");
const findIndex = ref(0);
const findHits = ref<HTMLElement[]>([]);
const findInputRef = ref<HTMLInputElement | null>(null);
const stickToBottom = ref(true);
const activeUserMessageId = ref("");
const rewindBusy = ref(false);
const copyStatus = ref<{ id: string; state: "copied" | "failed" } | null>(null);
const durationClock = ref(Date.now());
let copyStatusTimer: number | undefined;
let durationTimer: number | undefined;

provideConversationFind({
  active: findOpen,
  query: findQuery,
});

const findCountLabel = computed(() => {
  const query = findQuery.value.trim();
  if (!query) return "";
  if (findHits.value.length === 0) return tr(settingStore.language, "findNoResults");
  return tr(settingStore.language, "findMatchCount", {
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

watch(
  () => props.sessionId,
  () => {
    closeFind();
    findQuery.value = "";
  },
);

function normalizeRole(role: ChatMessage["role"] | string) {
  return String(role).toLowerCase();
}
function isUserMessage(message: ChatMessage) {
  return normalizeRole(message.role) === "user";
}
function isSoftInjectMessage(message: ChatMessage) {
  return message.injected === true || isSoftInjectContent(message.content);
}
function softInjectText(message: ChatMessage) {
  return parseSelectionAttachment(stripSoftInjectMarker(message.content)).message;
}
function userContent(message: ChatMessage) {
  return parseSelectionAttachment(stripSoftInjectMarker(message.content));
}

type InlineMessagePart =
  | { kind: "text"; text: string }
  | { kind: "mention"; path: string; name: string; isDir: boolean }
  | { kind: "skill"; id: string }
  | { kind: "mcp"; id: string };

/** Match `@file`, `#skill:id`, and `#mcp:id` for markdown-like inline marks. */
function inlineMessageParts(text: string): InlineMessagePart[] {
  return splitInlineTokenParts(text).map((part) => {
    if (part.kind === "mention") {
      return { kind: "mention", path: part.path, name: part.name, isDir: part.isDir };
    }
    if (part.kind === "skill" || part.kind === "mcp") {
      return { kind: part.kind, id: part.id };
    }
    return { kind: "text", text: part.text };
  });
}

function fileIconForPath(path: string) {
  return codeLanguageForPath(normalizeMentionPath(path)).icon;
}

function copyableUserText(message: ChatMessage) {
  const content = userContent(message);
  return [content.message.trim(), content.selection?.trim() ?? ""].filter(Boolean).join("\n\n");
}

function copyButtonLabel(messageId: string) {
  if (copyStatus.value?.id !== messageId) return tr(settingStore.language, "copy");
  return tr(settingStore.language, copyStatus.value.state === "copied" ? "copied" : "copyFailed");
}

function copyButtonClass(messageId: string) {
  if (copyStatus.value?.id !== messageId) return undefined;
  return copyStatus.value.state;
}

async function copyMessage(message: ChatMessage, kind: "user" | "assistant") {
  const text = kind === "user" ? copyableUserText(message) : message.content;
  if (!text) return;
  if (copyStatusTimer) window.clearTimeout(copyStatusTimer);
  try {
    await copyText(text);
    copyStatus.value = { id: message.id, state: "copied" };
  } catch (error) {
    console.error("failed to copy message:", error);
    copyStatus.value = { id: message.id, state: "failed" };
  }
  copyStatusTimer = window.setTimeout(() => {
    if (copyStatus.value?.id === message.id) copyStatus.value = null;
    copyStatusTimer = undefined;
  }, 1600);
}

function canBranchMessage(message: ChatMessage) {
  if (!props.sessionId) return false;
  return message.status !== "pending" && message.status !== "streaming";
}

function branchFromMessage(item: Extract<DisplayItem, { kind: "assistant" }>) {
  if (!canBranchMessage(item.message)) return;
  const lastInject = item.injects[item.injects.length - 1];
  emit("branch", lastInject?.id || item.message.id);
}

/** Image analyses are persisted on the preceding user message; show them on the assistant turn. */
function precedingUserMessage(assistant: ChatMessage): ChatMessage | undefined {
  const list = visibleMessages.value;
  const index = list.findIndex((item) => item.id === assistant.id);
  if (index <= 0) return undefined;
  for (let i = index - 1; i >= 0; i -= 1) {
    if (isUserMessage(list[i]!) && !isSoftInjectMessage(list[i]!)) {
      return list[i];
    }
  }
  return undefined;
}

function imageAnalysesForAssistant(message: ChatMessage) {
  const user = precedingUserMessage(message);
  if (!user) return [];
  return userContent(user).imageAnalyses ?? [];
}

function checkpointFor(message: ChatMessage) {
  return (props.checkpoints ?? []).find((item) => item.userMessageId === message.id);
}

function turnTokenCount(item: Extract<DisplayItem, { kind: "assistant" }>) {
  const user = precedingUserMessage(item.message);
  return [user, item.message, ...item.injects]
    .filter((message): message is ChatMessage => Boolean(message))
    .reduce((total, message) => total + estimateMessageTokens(message), 0);
}

function tokenEstimateTitle(tokens: number) {
  return tr(settingStore.language, "tokens.estimated", {
    count: new Intl.NumberFormat(settingStore.language).format(tokens),
  });
}

function turnCacheUsage(item: Extract<DisplayItem, { kind: "assistant" }>) {
  const sessionId = item.message.sessionId || props.sessionId;
  if (!sessionId) return undefined;
  return chatStore.messageCacheUsage[sessionId]?.[item.message.id];
}

function turnCacheHit(item: Extract<DisplayItem, { kind: "assistant" }>) {
  const usage = turnCacheUsage(item);
  if (!usage) return null;
  return promptCacheHitPercent(usage.inputTokens, usage.cacheReadTokens);
}

function turnCacheHitTitle(item: Extract<DisplayItem, { kind: "assistant" }>) {
  const usage = turnCacheUsage(item);
  const percent = turnCacheHit(item);
  if (!usage || percent == null) return "";
  return tr(settingStore.language, "tokens.turnCacheHitTitle", {
    percent,
    cached: formatTokenCount(usage.cacheReadTokens, settingStore.language),
    prompt: formatTokenCount(
      promptTokenTotal(usage.inputTokens, usage.cacheReadTokens),
      settingStore.language,
    ),
  });
}

function processingDuration(message: ChatMessage): string | undefined {
  const startedAt = precedingUserMessage(message)?.timestamp;
  if (!startedAt) return undefined;

  const running = isPending(message);
  const finishedAt = running ? durationClock.value : message.completedAt;
  if (!finishedAt || finishedAt < startedAt) return undefined;

  const totalSeconds = Math.max(0, Math.floor((finishedAt - startedAt) / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return minutes > 0 ? `${minutes} m ${seconds} s` : `${seconds} s`;
}
function checkpointForAssistant(message: ChatMessage) {
  const userMessage = precedingUserMessage(message);
  return userMessage ? checkpointFor(userMessage) : undefined;
}
function confirmAssistantRewind(message: ChatMessage) {
  const userMessage = precedingUserMessage(message);
  if (userMessage) void confirmRewind(userMessage);
}
async function confirmRewind(message: ChatMessage) {
  const checkpoint = checkpointFor(message);
  if (!checkpoint || !props.sessionId || rewindBusy.value) return;

  const confirmed = await confirmDialogRef.value?.ask({
    title: tr(settingStore.language, "rewindConfirmTitle"),
    description: tr(settingStore.language, "rewindConfirm"),
    confirmLabel: tr(settingStore.language, "rewindConfirmAction"),
    cancelLabel: tr(settingStore.language, "rewindCancel"),
  });
  if (!confirmed) return;

  const text = userContent(message).message.trim();
  rewindBusy.value = true;
  try {
    await rewindSession({
      sessionId: props.sessionId,
      turn: checkpoint.turn,
      restore: "both",
    });
    emit("rewound", { text });
  } catch (error) {
    console.error("rewind_session failed:", error);
  } finally {
    rewindBusy.value = false;
  }
}
function isPending(message: ChatMessage) {
  return message.status === "pending" || message.status === "streaming";
}
function getFilename(path: string | undefined): string {
  if (!path) return "";
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

function isWaitingForAskUser(message: ChatMessage) {
  return (message.toolActivities ?? []).some(
    (activity) => activity.toolName === "ask_user" && activity.status === "running",
  );
}

function activityLabel(message: ChatMessage) {
  if (message.activityStatus === "context_compacting") {
    return tr(settingStore.language, "compactingContext");
  }
  if (message.activityStatus?.startsWith("stream_retry")) {
    const [, attemptRaw, maxRaw] = message.activityStatus.split(":");
    const attempt = Number.parseInt(attemptRaw ?? "1", 10) || 1;
    const max = Number.parseInt(maxRaw ?? "5", 10) || 5;
    return tr(settingStore.language, "streamRetrying", { attempt, max });
  }
  if (message.activityStatus === "reject_empty_completion") {
    return "检测到空完成，正在纠正并强制重试修改...";
  }
  if (!isPending(message) || isWaitingForAskUser(message)) return "";

  // Prefer real reply progress over a stale analyzing label.
  if (
    message.activityStatus === "analyzing_images" &&
    !message.content.trim() &&
    !message.reasoning?.trim()
  ) {
    return tr(settingStore.language, "analyzingImages");
  }

  const running = [...(message.toolActivities ?? [])]
    .reverse()
    .find((activity) => activity.status === "running");
  if (running) {
    if (running.toolName === "ask_user") {
      return tr(settingStore.language, "waitingAnswer");
    }

    const args = (running.arguments || {}) as Record<string, string | undefined>;

    // 1. Reading file
    if (running.toolName === "read_file" || running.toolName === "view_file") {
      const path = args.AbsolutePath || args.TargetFile || args.path;
      const file = getFilename(path);
      return file ? `正在读取 ${file}` : "正在读取文件";
    }

    // 2. Listing directory / Getting workspace details
    if (
      running.toolName === "list_dir" ||
      running.toolName === "list_folder" ||
      running.toolName === "list_workspace_files"
    ) {
      return "正在获取目录信息";
    }

    // 3. Writing or editing file
    if (
      running.toolName === "write_to_file" ||
      running.toolName === "replace_file_content" ||
      running.toolName === "multi_replace_file_content" ||
      ["create", "edit", "delete", "move"].includes(running.kind)
    ) {
      const path = args.TargetFile || args.AbsolutePath || args.path || args.to || args.from;
      const file = getFilename(path);
      return file ? `正在编写 ${file}` : "正在编写中";
    }

    // 4. Searching / Grep search
    if (
      running.toolName === "grep_search" ||
      running.toolName === "find_files" ||
      running.toolName === "search_files"
    ) {
      const query = args.Query || args.pattern || args.query;
      return query ? `正在查找 "${query}"` : "正在查找";
    }

    // 5. Web Search / Read URL
    if (running.toolName === "search_web") {
      const query = args.query;
      return query ? `正在搜索 "${query}"` : "正在进行网页搜索";
    }
    if (running.toolName === "read_url_content") {
      return "正在读取网页内容";
    }

    // 6. Shell Command
    if (running.toolName === "run_command" || running.kind === "shell") {
      const cmd = args.CommandLine || args.command || args.commandLine;
      return cmd ? `正在执行: ${cmd}` : "正在执行命令";
    }

    if (running.kind === "image" || running.toolName === "generate_image") {
      return tr(settingStore.language, "image.generating");
    }

    if (running.kind === "read") return tr(settingStore.language, "reading");
    return tr(settingStore.language, "working");
  }
  if (message.content) return tr(settingStore.language, "responding");
  return tr(settingStore.language, "thinking");
}

function activityIcon(message: ChatMessage): Component | undefined {
  const running = [...(message.toolActivities ?? [])]
    .reverse()
    .find((activity) => activity.status === "running");
  if (running?.kind === "image" || running?.toolName === "generate_image") {
    return Paintbrush;
  }
  return undefined;
}
function isNearBottom(element: HTMLElement) {
  const padBottom = Number.parseFloat(getComputedStyle(element).paddingBottom) || 0;
  // Ignore composer clearance padding — it would otherwise make scrollTop=0
  // look "not at bottom" on short threads and break stick-to-bottom.
  const contentBottom = element.scrollHeight - padBottom;
  const viewportBottom = element.scrollTop + element.clientHeight;
  return contentBottom - viewportBottom <= SCROLL_NEAR_BOTTOM_THRESHOLD;
}
function handleScroll() {
  const element = listRef.value;
  if (!element) return;
  stickToBottom.value = isNearBottom(element) || isLastTurnOnScreen(element);
  updateActiveUserMessage();
}
function updateActiveUserMessage() {
  const element = listRef.value;
  const users = userMessages.value;
  if (!element || !users.length) {
    activeUserMessageId.value = "";
    return;
  }
  // Geometric bottom, pinned last-turn follow, and "last item still on screen"
  // all mean the latest user tick — not a 40% spy-line that often still sits
  // inside the previous turn while the last reply is visible.
  if (stickToBottom.value || isNearBottom(element) || isLastTurnOnScreen(element)) {
    activeUserMessageId.value = users[users.length - 1]?.id ?? "";
    return;
  }

  const listRect = element.getBoundingClientRect();
  const inset = 8;
  let active = users[0]?.id ?? "";
  for (let index = 0; index < users.length; index += 1) {
    const message = users[index];
    const node = element.querySelector<HTMLElement>(
      `[data-message-id="${CSS.escape(message.id)}"]`,
    );
    if (!node) continue;
    const next = users[index + 1]
      ? element.querySelector<HTMLElement>(`[data-message-id="${CSS.escape(users[index + 1].id)}"]`)
      : null;
    const turnTop = node.getBoundingClientRect().top;
    let turnBottom = listRect.bottom;
    if (next) {
      turnBottom = next.getBoundingClientRect().top;
    } else {
      const items = element.querySelectorAll<HTMLElement>(".message-item");
      const lastItem = items[items.length - 1];
      if (lastItem) turnBottom = lastItem.getBoundingClientRect().bottom;
    }
    const intersects = turnBottom > listRect.top + inset && turnTop < listRect.bottom - inset;
    if (intersects) active = message.id;
  }
  activeUserMessageId.value = active;
}

function isLastTurnOnScreen(element: HTMLElement) {
  const items = element.querySelectorAll<HTMLElement>(".message-item");
  const lastItem = items[items.length - 1];
  if (!lastItem) return false;
  const listRect = element.getBoundingClientRect();
  const lastRect = lastItem.getBoundingClientRect();
  return lastRect.top < listRect.bottom && lastRect.bottom <= listRect.bottom + 48;
}
function scrollToMessage(messageId: string) {
  const container = listRef.value;
  const node = container?.querySelector<HTMLElement>(
    `[data-message-id="${CSS.escape(messageId)}"]`,
  );
  if (!container || !node) return;
  stickToBottom.value = false;
  activeUserMessageId.value = messageId;
  // Scroll the message list scroller only — not the overlay / window.
  // Offset accounts for the absolute thread header overlay.
  gsapScrollContainerTo(container, node, { offsetY: 42 });
  railRef.value?.focus();
}
function jumpRail(delta: number) {
  const users = userMessages.value;
  if (!users.length) return;
  const index = users.findIndex((message) => message.id === activeUserMessageId.value);
  const from = index < 0 ? (delta > 0 ? -1 : users.length) : index;
  const next = users[Math.min(users.length - 1, Math.max(0, from + delta))];
  if (next) scrollToMessage(next.id);
}
function onRailKeydown(event: KeyboardEvent) {
  if (findOpen.value) return;
  if (event.key === "ArrowUp") {
    event.preventDefault();
    jumpRail(-1);
    return;
  }
  if (event.key === "ArrowDown") {
    event.preventDefault();
    jumpRail(1);
    return;
  }
  if (event.key === "Home") {
    event.preventDefault();
    const first = userMessages.value[0];
    if (first) scrollToMessage(first.id);
    return;
  }
  if (event.key === "End") {
    event.preventDefault();
    const last = userMessages.value[userMessages.value.length - 1];
    if (last) scrollToMessage(last.id);
  }
}
function scrollToLatest() {
  const element = listRef.value;
  if (!element) return;
  stickToBottom.value = true;
  element.scrollTo({ top: element.scrollHeight, behavior: "smooth" });
  updateActiveUserMessage();
}
function messagePreview(message: ChatMessage) {
  const parsed = userContent(message);
  const compact = parsed.message.replace(/\s+/g, " ").trim();
  if (compact) {
    return compact.length > 72 ? `${compact.slice(0, 72)}...` : compact;
  }
  if (parsed.attachedFiles?.length) {
    return parsed.attachedFiles.map((file) => file.name).join(", ");
  }
  if (parsed.images?.length) {
    return parsed.images.length === 1 ? "image" : `${parsed.images.length} images`;
  }
  return "";
}
/**
 * Keep the latest user turn on-screen.
 *
 * Absolute scroll-to-bottom fights the large composer `padding-bottom`: on short
 * turns (especially while the overlay is still expanding) it scrolls the first
 * user bubble above the viewport. If the turn still fits, pin to the user
 * message (scrollTop 0 for the first turn); only follow the true bottom once
 * the reply no longer fits with that user message.
 */
async function scrollToBottomIfNeeded() {
  await nextTick();
  const element = listRef.value;
  if (!element) return;

  if (!stickToBottom.value) {
    updateActiveUserMessage();
    return;
  }

  const padBottom = Number.parseFloat(getComputedStyle(element).paddingBottom) || 0;
  const maxScroll = element.scrollHeight - element.clientHeight;
  if (maxScroll <= 1) {
    element.scrollTop = 0;
    updateActiveUserMessage();
    return;
  }

  const users = element.querySelectorAll<HTMLElement>(".message-item.user");
  const lastUser = users[users.length - 1];
  if (lastUser) {
    const listTop = element.getBoundingClientRect().top;
    const userTop = lastUser.getBoundingClientRect().top - listTop + element.scrollTop;
    const contentBottom = element.scrollHeight - padBottom;
    const turnHeight = contentBottom - userTop;

    // Whole turn still fits: keep the user bubble visible (top of thread for
    // the first message; otherwise align that user message near the top).
    if (turnHeight <= element.clientHeight - 4) {
      element.scrollTop = users.length <= 1 ? 0 : Math.max(0, userTop - 8);
      updateActiveUserMessage();
      return;
    }
  }

  element.scrollTop = element.scrollHeight;
  updateActiveUserMessage();
}

watch(
  () => props.messages.length,
  (length, previousLength) => {
    if (length > (previousLength ?? 0)) stickToBottom.value = true;
  },
);
watch(
  () => {
    const messages = props.messages;
    const last = messages[messages.length - 1];
    if (!last) return "0";
    const tools =
      last.toolActivities
        ?.map((activity) => `${activity.id}:${activity.status}:${activity.detail?.length ?? 0}`)
        .join(",") ?? "";
    const asks = last.askUserAnswer?.map((answer) => answer.selected.join(",")).join(";") ?? "";
    return `${messages.length}|${last.id}:${last.content.length}:${last.reasoning?.length ?? 0}:${tools}:${asks}:${last.status}:${last.activityStatus ?? ""}`;
  },
  () => void scrollToBottomIfNeeded(),
  { immediate: true },
);

watch(
  () => {
    const last = props.messages[props.messages.length - 1];
    return `${props.messages.length}:${last?.id ?? ""}:${last?.content.length ?? 0}:${last?.status ?? ""}`;
  },
  () => {
    if (!findOpen.value || !findQuery.value.trim()) return;
    void nextTick(() => refreshFindHits({ scroll: false }));
  },
);

let resizeObserver: ResizeObserver | null = null;
onMounted(() => {
  durationTimer = window.setInterval(() => {
    if (visibleMessages.value.some(isPending)) durationClock.value = Date.now();
  }, 1000);
  const element = listRef.value;
  if (!element || typeof ResizeObserver === "undefined") return;
  resizeObserver = new ResizeObserver(() => {
    if (element.clientHeight < 8) return;
    void scrollToBottomIfNeeded();
  });
  resizeObserver.observe(element);
  globalThis.addEventListener("keydown", onFindWindowKeydown);
});
onUnmounted(() => {
  resizeObserver?.disconnect();
  resizeObserver = null;
  if (copyStatusTimer) window.clearTimeout(copyStatusTimer);
  if (durationTimer) window.clearInterval(durationTimer);
  globalThis.removeEventListener("keydown", onFindWindowKeydown);
  clearFindHits(listRef.value);
});

defineExpose({ openFind, closeFind });
</script>

<style scoped>
.message-list-shell {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
  width: 100%;
}
.message-list-shell.has-find .message-list {
  scroll-padding-top: 52px;
}
.conversation-find-bar {
  position: absolute;
  z-index: 8;
  top: 8px;
  right: 36px;
  display: flex;
  align-items: center;
  gap: 2px;
  max-width: min(360px, calc(100% - 48px));
  padding: 4px 4px 4px 8px;
  border: 1px solid var(--peek-border, rgba(0, 0, 0, 0.12));
  border-radius: 8px;
  background: var(--peek-surface, #fff);
  box-shadow: 0 8px 24px color-mix(in srgb, #000 14%, transparent);
}
.conversation-find-input {
  min-width: 0;
  flex: 1;
  height: 26px;
  padding: 0 6px 0 2px;
  border: 0;
  background: transparent;
  color: var(--peek-text);
  font: inherit;
  font-size: 12px;
  outline: none;
}
.conversation-find-count {
  flex: none;
  min-width: 3.5em;
  padding: 0 6px;
  color: var(--peek-muted);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  text-align: right;
  white-space: nowrap;
}
.conversation-find-count.empty {
  color: var(--peek-danger, #f14c4c);
}
.conversation-find-btn {
  flex: none;
  width: 24px;
  height: 24px;
  display: inline-grid;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}
.conversation-find-btn:hover:not(:disabled) {
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 8%, transparent);
}
.conversation-find-btn:disabled {
  cursor: default;
  opacity: 0.35;
}
:deep(mark.conversation-find-hit) {
  padding: 0;
  border-radius: 2px;
  background: color-mix(in srgb, #eab308 58%, transparent);
  color: inherit;
  box-decoration-break: clone;
}
:deep(mark.conversation-find-hit.is-current) {
  background: color-mix(in srgb, #f59e0b 82%, transparent);
  outline: 1px solid color-mix(in srgb, #d97706 65%, transparent);
}
.message-list {
  flex: 1;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: 12px 28px 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  scroll-padding-top: 12px;
}
.empty-thread {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 120px;
  color: var(--peek-muted);
  font-size: 13px;
  text-align: center;
  user-select: none;
}
.message-preview-rail {
  position: absolute;
  z-index: 4;
  top: 42px;
  right: 2px;
  bottom: 10px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 28px;
  min-width: 28px;
  max-width: 28px;
  overflow-x: hidden;
  overflow-y: auto;
  scrollbar-width: none;
  outline: none;
  pointer-events: none;
}
.message-preview-rail::-webkit-scrollbar {
  display: none;
}
.scroll-to-bottom {
  position: absolute;
  z-index: 6;
  left: 50%;
  bottom: 72px;
  width: 34px;
  height: 34px;
  margin-left: -17px;
  display: grid;
  place-items: center;
  padding: 0;
  border: 1px solid color-mix(in srgb, var(--peek-border) 72%, transparent);
  border-radius: 50%;
  background: var(--peek-list-bg, #fff);
  color: var(--peek-text);
  box-shadow: 0 1px 3px color-mix(in srgb, var(--peek-shadow) 22%, transparent);
  cursor: pointer;
}
.scroll-to-bottom:hover {
  border-color: var(--peek-border);
  background: var(--peek-surface);
}
.scroll-to-bottom-enter-active,
.scroll-to-bottom-leave-active {
  transition: opacity 140ms ease;
}
.scroll-to-bottom-enter-from,
.scroll-to-bottom-leave-to {
  opacity: 0;
}
.message-preview-mark {
  position: relative;
  flex: none;
  width: 28px;
  height: 14px;
  padding: 0;
  border: 0;
  background: transparent;
  cursor: pointer;
  pointer-events: auto;
}
.mark-line {
  position: absolute;
  top: 6px;
  right: 7px;
  width: 7px;
  height: 2px;
  border-radius: 1px;
  background: var(--peek-faint);
  transition:
    width 120ms ease,
    background 120ms ease;
}
.message-preview-mark:hover .mark-line,
.message-preview-mark:focus-visible .mark-line,
.message-preview-mark.active .mark-line {
  width: 11px;
  background: var(--peek-accent);
}
.message-preview-tooltip {
  position: fixed;
  z-index: 20;
  right: 30px;
  width: min(250px, calc(100vw - 48px));
  padding: 6px 8px;
  border: 1px solid var(--peek-border);
  border-radius: var(--peek-radius-sm, 6px);
  background: var(--peek-list-bg);
  color: var(--peek-text);
  box-shadow: var(--peek-elev-md);
  font-size: var(--peek-font-xs, 11px);
  line-height: 1.45;
  text-align: left;
  opacity: 0;
  visibility: hidden;
  pointer-events: none;
  transform: translateY(-4px);
  transition:
    opacity 100ms ease,
    transform 100ms ease;
}
.message-preview-mark:hover .message-preview-tooltip,
.message-preview-mark:focus-visible .message-preview-tooltip {
  opacity: 1;
  visibility: visible;
  transform: translateY(0);
}
.message-item.user {
  display: flex;
  justify-content: flex-end;
  width: 100%;
}
.message-item.assistant {
  display: flex;
  justify-content: flex-start;
  width: 100%;
}
.user-turn {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 6px;
  max-width: 78%;
}
.user-images {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
  max-width: 100%;
  padding: 1px;
}
.user-attached-files {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 6px;
  max-width: 100%;
}
.user-file-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: min(220px, 100%);
  height: var(--peek-control-icon, 28px);
  padding: 0 10px;
  border: 1px solid var(--peek-border);
  border-radius: var(--peek-radius-sm, 6px);
  background: color-mix(in srgb, var(--peek-user-bubble-bg) 88%, var(--peek-surface));
  color: var(--peek-user-bubble-text);
  font-size: 12px;
  font-weight: 500;
  line-height: 1;
}
.user-file-icon-img {
  flex: none;
  width: 13px;
  height: 13px;
  object-fit: contain;
}
.user-file-chip.skipped {
  opacity: 0.55;
}
.user-file-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.user-image-btn {
  display: block;
  margin: 0;
  padding: 0;
  border: none;
  background: transparent;
  border-radius: 12px;
  overflow: hidden;
  cursor: zoom-in;
  max-width: min(280px, 72vw);
  line-height: 0;
  box-shadow: 0 0 0 1px var(--peek-border);
  transform: translateZ(0);
  transition: box-shadow 140ms ease;
}
.user-image-btn:hover {
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--peek-accent) 55%, var(--peek-border));
}
.user-image {
  display: block;
  width: auto;
  height: auto;
  max-width: min(280px, 72vw);
  max-height: 360px;
  object-fit: contain;
  border-radius: inherit;
  user-select: none;
}
.user-bubble {
  width: fit-content;
  max-width: 100%;
  padding: 9px 12px;
  border: 1px solid color-mix(in srgb, var(--peek-user-bubble-border) 70%, transparent);
  border-radius: var(--peek-radius-lg, 12px) var(--peek-radius-lg, 12px) var(--peek-radius-sm, 6px)
    var(--peek-radius-lg, 12px);
  background: var(--peek-user-bubble-bg);
  color: var(--peek-user-bubble-text);
  font-size: var(--peek-font-md, 13px);
  line-height: 1.65;
  white-space: pre-wrap;
  word-break: break-word;
  overflow-wrap: anywhere;
  box-shadow: var(--peek-elev-sm);
}
.user-message-text {
  display: inline;
  min-width: 0;
}
.message-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  min-height: var(--peek-control-icon, 28px);
}
.user-message-actions {
  justify-content: flex-end;
}
.assistant-message-actions {
  justify-content: flex-start;
}
.processing-duration,
.token-usage,
.cache-hit {
  margin-right: 4px;
  color: var(--peek-muted);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  opacity: 0.72;
}
.message-action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--peek-control-icon, 28px);
  height: var(--peek-control-icon, 28px);
  padding: 0;
  border: 0;
  border-radius: var(--peek-radius-sm, 6px);
  background: transparent;
  color: var(--peek-icon, var(--peek-muted));
  cursor: pointer;
  opacity: 0.88;
}
.message-action-btn:hover:not(:disabled) {
  opacity: 1;
  color: var(--peek-accent);
  background: color-mix(in srgb, var(--peek-accent) 14%, transparent);
}
.message-action-btn:focus-visible {
  outline: none;
  box-shadow: var(--peek-focus-ring);
}
.message-action-btn.copied {
  color: var(--peek-success);
  opacity: 1;
}
.message-action-btn.failed {
  color: var(--peek-danger);
  opacity: 1;
}
.message-action-btn:disabled {
  cursor: default;
  opacity: 0.4;
}
.user-selection-quote {
  display: block;
  margin-top: 6px;
  color: color-mix(in srgb, var(--peek-user-bubble-text) 70%, var(--peek-muted));
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}
.assistant-bubble {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
  max-width: 94%;
  min-width: 0;
  padding: 0;
  color: var(--peek-text);
}

.provider-setup-card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 12px;
  max-width: 420px;
}

.provider-setup-text {
  margin: 0;
  font-size: 13px;
  line-height: 1.6;
  color: var(--peek-text);
  white-space: pre-wrap;
}

.provider-setup-btn {
  appearance: none;
  border: 1px solid color-mix(in srgb, var(--peek-text) 14%, transparent);
  background: var(--peek-text);
  color: var(--peek-bg, #fff);
  border-radius: 999px;
  padding: 7px 14px;
  font-size: 12.5px;
  font-weight: 560;
  cursor: pointer;
  transition:
    opacity 0.15s ease,
    transform 0.15s ease;
}

.provider-setup-btn:hover {
  opacity: 0.92;
}

.provider-setup-btn:active {
  transform: translateY(0.5px);
}
.assistant-bubble :deep(.markdown-body) {
  font-size: 13px;
  line-height: 1.6;
}
.assistant-bubble :deep(.agent-work),
.assistant-bubble :deep(.tool-activity-list),
.assistant-bubble :deep(.reasoning-block),
.assistant-bubble :deep(.ask-answer-card),
.assistant-bubble :deep(.image-analysis-card) {
  width: 100%;
  max-width: none;
  box-sizing: border-box;
}

.soft-inject-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 2px;
  max-width: 100%;
}

.soft-inject-chip {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: fit-content;
  max-width: 100%;
  padding: 6px 10px;
  border: 1px dashed color-mix(in srgb, var(--peek-accent) 35%, var(--peek-border));
  border-radius: 10px;
  background: color-mix(in srgb, var(--peek-accent) 8%, transparent);
  color: var(--peek-text);
}

.soft-inject-label {
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: color-mix(in srgb, var(--peek-accent) 75%, var(--peek-muted));
}

.soft-inject-text {
  font-size: 12px;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-word;
  color: color-mix(in srgb, var(--peek-text) 88%, var(--peek-muted));
}
</style>
