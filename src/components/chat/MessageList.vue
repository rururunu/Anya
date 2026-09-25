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
      @keydown="onRailKeydown($event, scrollToMessage)"
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
      @wheel.passive="handleWheel"
      @keydown="handleKeydown"
    >
      <div
        v-if="hasOlderHistory || loadingOlderHistory || hiddenLoadedTurns > 0"
        class="load-earlier-row"
      >
        <button
          type="button"
          class="load-earlier"
          :disabled="loadingOlderHistory"
          @click="loadEarlierMessages"
        >
          <span v-if="loadingOlderHistory" class="load-earlier-spinner" aria-hidden="true" />
          {{
            tr(
              settingStore.language,
              hiddenLoadedTurns > 0 ? "showEarlierMessages" : "loadEarlierMessages",
            )
          }}
        </button>
      </div>
      <div v-if="displayItems.length === 0" class="empty-thread">
        {{ emptyThreadPrompt }}
      </div>
      <div
        v-for="(turn, turnIndex) in renderedTurns.items"
        :key="turn.key"
        class="chat-turn"
        :class="{ 'is-first-turn': renderedTurns.start + turnIndex === 0 }"
        v-memo="turnMemoDeps(turn, renderedTurns.start + turnIndex)"
        :ref="(el) => bindTurnHead(turn.key, el)"
      >
        <div
          v-if="turn.user || turnBuildHost(turn)"
          class="chat-turn-sticky-sentinel"
          aria-hidden="true"
        />
        <div
          v-if="turn.user || turnBuildHost(turn)"
          class="chat-turn-head"
          :class="{ 'is-stuck': isTurnHeadStuck(turn.key) }"
        >
          <article
            v-if="turn.user"
            class="message-item user"
            :data-message-id="turn.user.message.id"
          >
            <UserMessageCard
              class="user-turn"
              :message="turn.user.message"
              :session-id="sessionId ?? ''"
              :can-resend="Boolean(checkpointFor(turn.user.message))"
              :busy="rewindBusy"
              @preview="previewImage"
              @resend="
                (text) => {
                  const user = turn.user;
                  if (user) void resendUserMessage(user.message, text);
                }
              "
              @rewind="
                () => {
                  const user = turn.user;
                  if (user) void confirmRewind(user.message);
                }
              "
            />
          </article>
          <PlanApprovalCard
            v-if="turnBuildHost(turn)"
            class="turn-head-build"
            variant="build"
            v-bind="turnBuildCardBind(turn)"
            @preview-plan="$emit('reviewPlan')"
          />
        </div>
        <template v-for="item in turn.body" :key="item.key">
          <article
            v-if="item.kind === 'inject'"
            class="message-item inject"
            :data-message-id="item.message.id"
          >
            <SoftInjectCard :message="item.message" />
          </article>
          <article v-else class="message-item assistant" :data-message-id="item.message.id">
            <div class="assistant-bubble">
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
              <PlanApprovalCard
                v-if="showCreatedPlanCard(item.message)"
                variant="proposal"
                :title="planCardTitle(item.message)"
                :summary="planCardSummary(item.message)"
                :busy="planBusy || isSessionSending"
                :show-actions="canApprovePlan(item.message)"
                @approve="approvePlanMode"
                @reject="rejectPlanMode"
                @preview-plan="$emit('reviewPlan')"
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
                <template v-if="processingDuration(item.message)">
                  <span
                    class="turn-mascot"
                    :title="turnMascotTitle(item.message)"
                    aria-hidden="true"
                  >
                    <MascotFace :state="turnMascotState(item.message)" :follow-pointer="false" />
                  </span>
                  <span v-if="turnMascotLabel(item.message)" class="turn-state">
                    {{ turnMascotLabel(item.message) }}
                  </span>
                </template>
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
        </template>
      </div>
      <div class="turn-spacer" aria-hidden="true" />
    </div>

    <AppConfirmDialog ref="confirmDialogRef" />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch, type Component } from "vue";
import {
  ArrowDown,
  Check,
  ChevronDown,
  ChevronUp,
  Copy,
  GitBranch,
  Paintbrush,
  X,
} from "@lucide/vue";
import AgentWorkDetails from "@/components/chat/AgentWorkDetails.vue";
import CodeChangesSummary from "@/components/chat/CodeChangesSummary.vue";
import PlanApprovalCard from "@/components/chat/PlanApprovalCard.vue";
import AssistantActivityIndicator from "@/components/chat/AssistantActivityIndicator.vue";
import MascotFace, { type MascotState } from "@/components/icons/MascotFace.vue";
import ImageAnalysisDetails from "@/components/chat/ImageAnalysisDetails.vue";
import EnvironmentContextCard from "@/components/chat/EnvironmentContextCard.vue";
import UserMessageCard from "@/components/chat/UserMessageCard.vue";
import SoftInjectCard from "@/components/chat/SoftInjectCard.vue";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import {
  chatCancel,
  openSettings as ipcOpenSettings,
  rewindSession,
  setPlanMode,
} from "@/services/ipc";
import { useSettingStore } from "@/stores/setting";
import { useChatStore } from "@/stores/chat";
import { useChatSessionsStore } from "@/stores/chatSessions";
import type { ChatMessage, CheckpointInfo, TaskItem } from "@/types/chat";
import {
  parseSelectionAttachment,
  replaceUserVisibleText,
} from "@/services/chat/selectionAttachment";
import { extractAttachedFileChips, type AttachedFileChip } from "@/services/chat/attachFiles";
import { isSoftInjectContent, stripSoftInjectMarker } from "@/services/chat/softInject";
import { withTimelineInject } from "@/stores/chatStream";
import { isAskUserTool } from "@/services/chat/askUserAnswer";
import { isCompactionSummary } from "@/services/chat/compactMarker";
import {
  extractPlanTitle,
  isCreatedPlanMessage,
  isPlanExecutePrompt,
  planApprovePrompt,
  planCardCopy,
  planPathFromMessages,
  savePlanFromMessage,
  tasksFromMessage,
} from "@/services/chat/planProposal";
import { tr, type I18nKey } from "@/services/i18n";
import { createLogger } from "@/services/logger";
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
import { useMessagePreviewRail } from "@/composables/chat/useMessagePreviewRail";
import { useMessageScroll } from "@/composables/chat/useMessageScroll";
import { useTurnHeadSticky } from "@/composables/chat/useTurnHeadSticky";
import { isImageGenActivity } from "@/services/chat/toolActivityEnrichment";
import { useAppStore } from "@/stores/app";
import { storeToRefs } from "pinia";

type DisplayItem =
  | { kind: "user"; key: string; message: ChatMessage }
  | { kind: "assistant"; key: string; message: ChatMessage }
  | { kind: "inject"; key: string; message: ChatMessage };

type DisplayTurn = {
  key: string;
  user: Extract<DisplayItem, { kind: "user" }> | null;
  body: Array<Extract<DisplayItem, { kind: "assistant" | "inject" }>>;
};

function turnAssistants(turn: DisplayTurn) {
  return turn.body.filter(
    (item): item is Extract<DisplayItem, { kind: "assistant" }> => item.kind === "assistant",
  );
}
function previewImage(url: string) {
  emit("previewImage", url);
}

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
  rewound: [
    payload: {
      text: string;
      images?: string[];
      attachedFiles?: AttachedFileChip[];
      resend?: boolean;
    },
  ];
  branch: [messageId: string];
  reviewChanges: [];
  reviewFile: [path: string];
  reviewPlan: [];
  inspectSubagent: [activityId: string];
  previewImage: [source: string];
  editFromImage: [payload: import("@/services/chat/imageEditReference").ImageEditReferencePayload];
}>();
const settingStore = useSettingStore();
const appStore = useAppStore();
const chatStore = useChatStore();
const chatSessionsStore = useChatSessionsStore();
const log = createLogger("message-list");
const { sending } = storeToRefs(chatStore);
const planBusy = ref(false);

const isSessionSending = computed(() => Boolean(props.sessionId && sending.value[props.sessionId]));
/** Plan message that was approved — keep the checklist visible while it runs. */
const approvedPlanMessageId = ref<string | null>(null);
/** Plan message rejected by user. */
const rejectedPlanMessageId = ref<string | null>(null);

watch(
  () => props.sessionId,
  () => {
    approvedPlanMessageId.value = null;
    rejectedPlanMessageId.value = null;
    stickToBottom.value = true;
  },
);

const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);

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

const lastAssistantId = computed(() => {
  for (let i = props.messages.length - 1; i >= 0; i -= 1) {
    const message = props.messages[i];
    if (message && String(message.role).toLowerCase() === "assistant") return message.id;
  }
  return null;
});

function planTasksForMessage(message: ChatMessage, preferLive = false): TaskItem[] {
  const fromMessage = tasksFromMessage(message);
  if (preferLive && props.sessionId && shouldBindLiveTasks(message, fromMessage)) {
    const live = chatStore.sessionTasks[props.sessionId];
    if (live?.length) return live;
  }
  return fromMessage;
}

function shouldBindLiveTasks(message: ChatMessage, fromMessage: TaskItem[]): boolean {
  if (fromMessage.length) return true;
  const user = precedingUserMessage(message);
  return Boolean(
    user && isPlanExecutePrompt(userContent(user).message || String(user.content ?? "")),
  );
}

function planCardTitle(message: ChatMessage) {
  const fallback = tr(settingStore.language, "planProposalTitle");
  if (savePlanFromMessage(message)) return planCardCopy(message, fallback).title;
  const plan = props.sessionId ? chatStore.sessionPlans[props.sessionId] : undefined;
  if (plan?.content) return extractPlanTitle(plan.content) || fallback;
  return planCardCopy(message, fallback).title;
}

function planCardSummary(message: ChatMessage) {
  const copy = planCardCopy(message, tr(settingStore.language, "planProposalTitle"));
  if (copy.summary) return copy.summary;
  if (isPlanGateStopMessage(message) && !savePlanFromMessage(message)) {
    return tr(settingStore.language, "planModeNoTasksYet");
  }
  return copy.summary;
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
  return isCreatedPlanMessage(message) || isPlanGateStopMessage(message);
}

function canApprovePlan(message: ChatMessage): boolean {
  if (isApprovedPlan(message) || message.id === rejectedPlanMessageId.value) return false;
  if (hasUserMessageAfter(message.id)) return false;
  return hasApprovablePlan(message);
}

function isLivePlanHost(message: ChatMessage): boolean {
  const status = message.status;
  if (status === "pending" || status === "streaming") return true;
  return isSessionSending.value && lastAssistantId.value === message.id;
}

function showCreatedPlanCard(message: ChatMessage): boolean {
  return isCreatedPlanMessage(message) || isPlanGateStopMessage(message);
}

function showBuildTodoCard(message: ChatMessage): boolean {
  if (isCreatedPlanMessage(message)) return false;
  return planTasksForMessage(message, isLivePlanHost(message)).length > 0;
}

function turnBuildHost(turn: DisplayTurn) {
  const assistants = turnAssistants(turn);
  const live = assistants.find(
    (item) => showBuildTodoCard(item.message) && isLivePlanHost(item.message),
  );
  if (live) return live;
  for (let i = assistants.length - 1; i >= 0; i -= 1) {
    const item = assistants[i];
    if (item && showBuildTodoCard(item.message)) return item;
  }
  return null;
}

function turnBuildCardBind(turn: DisplayTurn) {
  const host = turnBuildHost(turn);
  if (!host) return { title: "", tasks: [] as TaskItem[] };
  return {
    title: planCardTitle(host.message),
    tasks: planTasksForMessage(host.message, isLivePlanHost(host.message)),
  };
}

/** If Agent left a pending plan checklist but the gate never flipped, recover it. */
function ensurePlanGateForPendingChecklist() {
  if (!props.sessionId || planBusy.value || isSessionSending.value) return;
  const messageId = lastDoneAssistantId.value;
  if (
    !messageId ||
    messageId === approvedPlanMessageId.value ||
    messageId === rejectedPlanMessageId.value
  )
    return;
  if (hasUserMessageAfter(messageId)) return;
  const message = props.messages.find((item) => item.id === messageId);
  if (!message) return;
  if (!hasApprovablePlan(message)) return;
  if (!planModeActive.value) {
    chatStore.setSessionPlanMode(props.sessionId, true);
    void setPlanMode(props.sessionId, true).catch((error) => {
      log.warn("recover plan gate for pending checklist failed", error);
    });
  }
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
  try {
    // Keep the checklist on this plan message; hide only the approve actions.
    if (lastDoneAssistantId.value) {
      approvedPlanMessageId.value = lastDoneAssistantId.value;
    }
    rejectedPlanMessageId.value = null;
    // Ensure writers unlock even if the gate was recovered only on the frontend.
    if (!planModeActive.value) {
      chatStore.setSessionPlanMode(props.sessionId, true);
      await setPlanMode(props.sessionId, true).catch(() => undefined);
    }
    // The approve prompt must name the real plan file: save_plan defaults to
    // .anya/plans/<stamp>-<slug>.md unless the agent passed an explicit path.
    const planPath =
      chatStore.sessionPlans[props.sessionId]?.path || planPathFromMessages(props.messages);
    await chatStore.send(planApprovePrompt(settingStore.language, planPath), props.sessionId, {
      skipAutoPlan: true,
      resumePlan: true,
    });
    chatStore.setCompose(props.sessionId, { chatMode: "agent" });
  } catch (error) {
    log.error("approve plan mode failed", error);
  } finally {
    planBusy.value = false;
  }
}

async function rejectPlanMode() {
  if (!props.sessionId || planBusy.value) return;
  if (lastDoneAssistantId.value) {
    rejectedPlanMessageId.value = lastDoneAssistantId.value;
  }
  await setPlanMode(props.sessionId, false).catch(() => undefined);
  chatStore.setSessionPlanMode(props.sessionId, false);
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
      items.push({
        kind: isSoftInjectMessage(message) ? "inject" : "user",
        key: message.id,
        message,
      });
      continue;
    }
    items.push({
      kind: "assistant",
      key: message.id,
      message,
    });
  }
  return items;
});

const displayTurns = computed((): DisplayTurn[] => {
  void lastDoneAssistantId.value;
  void approvedPlanMessageId.value;
  if (props.sessionId) {
    void chatStore.sessionTasks[props.sessionId];
    void chatStore.sessionPlans[props.sessionId];
    void chatStore.sessionPlanMode[props.sessionId];
  }
  const turns: DisplayTurn[] = [];
  let current: DisplayTurn | null = null;
  for (const item of displayItems.value) {
    if (item.kind === "user") {
      if (current) turns.push(current);
      current = { key: `turn-${item.key}`, user: item, body: [] };
      continue;
    }
    if (item.kind === "inject") {
      if (current) {
        let assistantIndex = -1;
        for (let index = current.body.length - 1; index >= 0; index -= 1) {
          if (current.body[index]?.kind === "assistant") {
            assistantIndex = index;
            break;
          }
        }
        if (assistantIndex !== -1) {
          const assistant = current.body[assistantIndex]!;
          current.body[assistantIndex] = {
            ...assistant,
            message: withTimelineInject(assistant.message, item.message),
          };
          continue;
        }
        current.body.push(item);
      } else {
        current = { key: `turn-${item.key}`, user: null, body: [item] };
      }
      continue;
    }
    if (!current) {
      current = { key: `turn-${item.key}`, user: null, body: [item] };
    } else {
      current.body.push(item);
    }
  }
  if (current) turns.push(current);
  return turns;
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
    copyStatus.value?.id === message.id ? copyStatus.value.state : "",
    rewindBusy.value ? 1 : 0,
    item.kind === "user" && checkpointFor(item.message) ? 1 : 0,
    live ? durationClock.value : 0,
    showCreatedPlanCard(message) ? 1 : 0,
    showBuildTodoCard(message) ? 1 : 0,
    showBuildTodoCard(message)
      ? planTasksForMessage(message, isLivePlanHost(message))
          .map((task) => `${task.status}:${task.content}`)
          .join("|")
      : "",
    planBusy.value ? 1 : 0,
    isSessionSending.value ? 1 : 0,
    settingStore.showReasoning ? 1 : 0,
    settingStore.agentWorkDisplay,
    settingStore.language,
  ];
}

const { bindTurnHead, isTurnHeadStuck } = useTurnHeadSticky();

function turnMemoDeps(turn: DisplayTurn, turnIndex = 0) {
  return [
    turn.key,
    turnIndex === 0 ? 1 : 0,
    isTurnHeadStuck(turn.key) ? 1 : 0,
    ...(turn.user ? messageMemoDeps(turn.user) : []),
    ...turn.body.flatMap((item) => messageMemoDeps(item)),
  ];
}

const userMessages = computed(() =>
  displayItems.value
    .filter((item): item is Extract<DisplayItem, { kind: "user" }> => item.kind === "user")
    .map((item) => item.message),
);
/**
 * Hard ceiling on rendered turns. Paging bounds how much history is *fetched*;
 * this bounds how much of it reaches the DOM. Without it, a user who keeps
 * paging back rebuilds the very DOM cost paging was meant to avoid.
 */
const RENDER_WINDOW_STEP = 80;
/** Keyed by session so switching conversations resets the window. */
const renderWindowState = ref<{ sessionId: string; size: number }>({
  sessionId: "",
  size: RENDER_WINDOW_STEP,
});
const renderWindow = computed(() =>
  renderWindowState.value.sessionId === props.sessionId
    ? renderWindowState.value.size
    : RENDER_WINDOW_STEP,
);
const renderedTurns = computed(() => {
  const turns = displayTurns.value;
  const start = Math.max(0, turns.length - renderWindow.value);
  return { start, items: turns.slice(start) };
});
/** Loaded but held back by the window: reveal these before fetching a page. */
const hiddenLoadedTurns = computed(() => renderedTurns.value.start);
const listRef = ref<HTMLElement | null>(null);
const stickToBottom = ref(true);
/** Older pages still live on the server: the window holds only the newest page. */
const hasOlderHistory = computed(() =>
  Boolean(props.sessionId && chatSessionsStore.historyPaging[props.sessionId]?.hasOlder),
);
const loadingOlderHistory = computed(() =>
  Boolean(props.sessionId && chatStore.historyLoadingOlder[props.sessionId]),
);
/**
 * Prepend the next older page. Content inserted above the viewport shifts every
 * row down, so the offset is corrected by the height delta once they are in.
 */
async function loadEarlierMessages() {
  const sessionId = props.sessionId;
  const container = listRef.value;
  if (!sessionId || !container) return;
  const previousHeight = container.scrollHeight;
  const previousTop = container.scrollTop;
  if (hiddenLoadedTurns.value > 0) {
    // Already in memory, just outside the rendered window.
    renderWindowState.value = {
      sessionId,
      size: renderWindow.value + RENDER_WINDOW_STEP,
    };
  } else {
    await chatStore.loadOlderMessages(sessionId);
  }
  await nextTick();
  // Rows inserted above shift everything down; subtract the delta so the rows
  // the user was reading stay put.
  container.scrollTop = container.scrollHeight - previousHeight + previousTop;
}
const findOpen = ref(false);
const findQuery = ref("");
const findIndex = ref(0);
const findHits = ref<HTMLElement[]>([]);
const findInputRef = ref<HTMLInputElement | null>(null);
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
function userContent(message: ChatMessage) {
  return parseSelectionAttachment(stripSoftInjectMarker(message.content));
}

const { railRef, activeUserMessageId, messagePreview, onRailKeydown, updateActiveUserMessage } =
  useMessagePreviewRail({
    listRef,
    userMessages,
    stickToBottom,
    findOpen,
    userContent,
  });

const { handleScroll, handleWheel, handleKeydown, scrollToMessage, scrollToLatest } =
  useMessageScroll({
    listRef,
    stickToBottom,
    messages: computed(() => props.messages),
    displayItems,
    activeUserMessageId,
    railRef,
    sessionId: computed(() => props.sessionId),
    isSending: isSessionSending,
    updateActiveUserMessage,
  });

function copyButtonLabel(messageId: string) {
  if (copyStatus.value?.id !== messageId) return tr(settingStore.language, "copy");
  return tr(settingStore.language, copyStatus.value.state === "copied" ? "copied" : "copyFailed");
}

function copyButtonClass(messageId: string) {
  if (copyStatus.value?.id !== messageId) return undefined;
  return copyStatus.value.state;
}

async function copyMessage(message: ChatMessage, kind: "user" | "assistant") {
  const text = kind === "user" ? userContent(message).message.trim() : message.content;
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
  const following = injectsFollowing(item.message);
  emit("branch", following[following.length - 1]?.id || item.message.id);
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

function injectsFollowing(assistant: ChatMessage): ChatMessage[] {
  const list = visibleMessages.value;
  const index = list.findIndex((item) => item.id === assistant.id);
  if (index < 0) return [];
  const found: ChatMessage[] = [];
  for (let i = index + 1; i < list.length; i += 1) {
    const next = list[i];
    if (!next || !isUserMessage(next) || !isSoftInjectMessage(next)) break;
    found.push(next);
  }
  return found;
}

function turnTokenCount(item: Extract<DisplayItem, { kind: "assistant" }>) {
  const user = precedingUserMessage(item.message);
  return [user, item.message, ...injectsFollowing(item.message)]
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

  const parsedContent = userContent(message);
  const text = parsedContent.message.trim();
  const images = parsedContent.images;
  const attachedFiles = extractAttachedFileChips(message.content);
  rewindBusy.value = true;
  try {
    await rewindSession({
      sessionId: props.sessionId,
      turn: checkpoint.turn,
      restore: "both",
    });
    emit("rewound", { text, images, attachedFiles });
  } catch (error) {
    console.error("rewind_session failed:", error);
  } finally {
    rewindBusy.value = false;
  }
}

async function resendUserMessage(message: ChatMessage, visibleText: string) {
  const checkpoint = checkpointFor(message);
  if (!checkpoint || !props.sessionId || rewindBusy.value) return;
  const nextContent = replaceUserVisibleText(message.content, visibleText);
  if (!nextContent.trim()) return;

  const confirmed = await confirmDialogRef.value?.ask({
    title: tr(settingStore.language, "resendConfirmTitle"),
    description: tr(settingStore.language, "resendConfirm"),
    confirmLabel: tr(settingStore.language, "resendConfirmAction"),
    cancelLabel: "esc",
    confirmOnEnter: true,
  });
  if (!confirmed) return;

  rewindBusy.value = true;
  try {
    if (isSessionSending.value) {
      const pending = props.messages.find(
        (item) => normalizeRole(item.role) === "assistant" && isPending(item),
      );
      chatStore.clearSending(props.sessionId);
      if (pending) {
        try {
          await chatCancel({ messageId: pending.id });
        } catch (error) {
          console.error("chat_cancel failed:", error);
        }
      }
    }
    await rewindSession({
      sessionId: props.sessionId,
      turn: checkpoint.turn,
      restore: "both",
    });
    chatStore.clearSending(props.sessionId);
    emit("rewound", { text: nextContent, resend: true });
  } catch (error) {
    console.error("resend_user_message failed:", error);
  } finally {
    rewindBusy.value = false;
  }
}
function isPending(message: ChatMessage) {
  return message.status === "pending" || message.status === "streaming";
}

function isWaitingForAskUser(message: ChatMessage) {
  return (message.toolActivities ?? []).some(
    (activity) => isAskUserTool(activity.toolName) && activity.status === "running",
  );
}

/** Face shown next to the turn stats, following what the agent is doing right now. */
function turnMascotState(message: ChatMessage): MascotState {
  switch (message.status) {
    case "error":
      return "error";
    case "cancelled":
      return "idle";
    case "done":
      return "done";
  }
  if (isWaitingForAskUser(message)) return "waiting";
  if ((message.toolActivities ?? []).some((activity) => activity.status === "running")) {
    return "working";
  }
  if (message.content.trim()) return "talking";
  return "thinking";
}

const MASCOT_STATE_LABEL_KEYS: Record<MascotState, I18nKey> = {
  idle: "mascotState.idle",
  thinking: "mascotState.thinking",
  working: "mascotState.working",
  talking: "mascotState.talking",
  waiting: "mascotState.waiting",
  done: "mascotState.done",
  error: "mascotState.error",
};
function turnMascotTitle(message: ChatMessage) {
  return tr(settingStore.language, MASCOT_STATE_LABEL_KEYS[turnMascotState(message)]);
}
/** Visible caption next to the face: only while something is actually happening. */
function turnMascotLabel(message: ChatMessage) {
  const state = turnMascotState(message);
  if (state === "idle" || state === "done") return "";
  return tr(settingStore.language, MASCOT_STATE_LABEL_KEYS[state]);
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

  const hasWorkStream =
    Boolean(message.reasoning?.trim()) || (message.toolActivities?.length ?? 0) > 0;
  if (hasWorkStream) {
    return "";
  }
  // The mascot in the stats row already narrates the generic thinking / replying
  // states; showing this indicator too would stack two faces and shift the layout.
  if (processingDuration(message)) {
    return "";
  }

  if (message.content.trim()) return tr(settingStore.language, "responding");
  return tr(settingStore.language, "agentTurnActive");
}

function activityIcon(message: ChatMessage): Component | undefined {
  const running = [...(message.toolActivities ?? [])]
    .reverse()
    .find((activity) => activity.status === "running");
  if (running && isImageGenActivity(running)) {
    return Paintbrush;
  }
  return undefined;
}

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

onMounted(() => {
  durationTimer = window.setInterval(() => {
    if (visibleMessages.value.some(isPending)) durationClock.value = Date.now();
  }, 1000);
  globalThis.addEventListener("keydown", onFindWindowKeydown);
});
onUnmounted(() => {
  if (copyStatusTimer) window.clearTimeout(copyStatusTimer);
  if (durationTimer) window.clearInterval(durationTimer);
  globalThis.removeEventListener("keydown", onFindWindowKeydown);
  clearFindHits(listRef.value);
});

defineExpose({ openFind, closeFind });
</script>

<style scoped>
/* "Load earlier" affordance: the loaded window holds only the newest page. */
.load-earlier-row {
  display: flex;
  justify-content: center;
  padding: 10px 16px 2px;
}

.load-earlier {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: transparent;
  color: var(--muted-foreground);
  font-size: 11px;
  cursor: pointer;
}

.load-earlier:hover:not(:disabled) {
  border-color: var(--muted-foreground);
}

.load-earlier:disabled {
  cursor: default;
  opacity: 0.6;
}

.load-earlier-spinner {
  width: 10px;
  height: 10px;
  border: 1.5px solid currentColor;
  border-top-color: transparent;
  border-radius: 50%;
  animation: load-earlier-spin 0.7s linear infinite;
}

@keyframes load-earlier-spin {
  to {
    transform: rotate(360deg);
  }
}

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
  --code-block-sticky-top: calc(-1 * var(--peek-space-4, 16px));
  --chat-sticky-top: calc(-1 * var(--peek-space-4, 16px) + 2px);
  --chat-reply-inset: var(--peek-radius-composer, 16px);
  flex: 1;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: var(--peek-space-4, 16px) 28px var(--peek-space-3, 12px) var(--peek-space-4, 16px);
  display: flex;
  flex-direction: column;
  gap: var(--peek-space-4, 16px);
  scroll-padding-top: max(0px, var(--chat-sticky-top, 0px));
  contain: layout style;
}
.message-item {
  content-visibility: auto;
  contain-intrinsic-size: auto 120px;
}
.empty-thread {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 120px;
  color: var(--peek-muted);
  font-size: var(--peek-font-md, 14px);
  line-height: 1.5;
  text-align: center;
  user-select: none;
}
.turn-spacer {
  display: none;
  height: 0;
  flex: none;
  min-height: 0;
  pointer-events: none;
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
  justify-content: flex-start;
  width: 100%;
  contain: layout style;
}
.message-item.assistant {
  display: flex;
  justify-content: flex-start;
  width: 100%;
  contain: layout style;
}
.chat-turn:has(.chat-turn-head) .message-item.assistant {
  content-visibility: visible;
  contain: none;
}
.chat-turn {
  display: flex;
  flex-direction: column;
  gap: inherit;
  width: 100%;
  min-width: 0;
}
.chat-turn:has(.chat-turn-head) {
  --turn-head-height: 40px;
  --chat-sticky-fade: 12px;
  --code-block-sticky-top: calc(
    var(--chat-sticky-top, 0px) + var(--turn-head-height) + var(--chat-sticky-fade)
  );
}
.chat-turn-sticky-sentinel {
  height: 1px;
  margin-bottom: -1px;
  pointer-events: none;
  visibility: hidden;
}
.chat-turn-head {
  position: sticky;
  top: var(--chat-sticky-top, 2px);
  z-index: 4;
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
  padding-bottom: 0;
  overflow: visible;
  background: transparent;
  box-shadow: none;
  pointer-events: none;
}
.chat-turn-head.is-stuck::before {
  content: "";
  position: absolute;
  right: var(--chat-reply-inset, var(--peek-radius-composer, 16px));
  bottom: 100%;
  left: var(--chat-reply-inset, var(--peek-radius-composer, 16px));
  height: calc(-1 * min(0px, var(--chat-sticky-top, 0px)) + 8px);
  min-height: 8px;
  background: var(--peek-list-bg);
  pointer-events: none;
}
.chat-turn-head.is-stuck::after {
  content: "";
  position: absolute;
  right: var(--chat-reply-inset, var(--peek-radius-composer, 16px));
  top: 100%;
  left: var(--chat-reply-inset, var(--peek-radius-composer, 16px));
  height: var(--chat-sticky-fade, 12px);
  pointer-events: none;
  background: linear-gradient(
    to bottom,
    var(--peek-list-bg) 0%,
    color-mix(in srgb, var(--peek-list-bg) 42%, transparent) 55%,
    transparent 100%
  );
}
.chat-turn.is-first-turn .chat-turn-head.is-stuck::before {
  content: none;
}
.chat-turn-head > * {
  pointer-events: auto;
}
.chat-turn-head .turn-head-build {
  width: calc(100% - 2 * var(--chat-reply-inset, var(--peek-radius-composer, 16px)));
  margin: 0 0 0 var(--chat-reply-inset, var(--peek-radius-composer, 16px));
}
.chat-turn-head .message-item.user {
  content-visibility: visible;
  contain: none;
}
.user-turn {
  width: 100%;
  max-width: 100%;
}
.message-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  min-height: var(--peek-control-icon, 28px);
}
.assistant-message-actions {
  justify-content: flex-start;
}
.turn-mascot {
  flex: none;
  display: block;
  width: 20px;
  height: 20px;
  margin-right: 6px;
  opacity: 0.9;
}
.turn-state {
  margin-right: 8px;
  color: var(--peek-muted);
  font-size: 11px;
  line-height: 20px;
  white-space: nowrap;
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
  transition:
    opacity var(--motion-fast, 110ms) ease,
    background-color var(--motion-fast, 110ms) ease,
    color var(--motion-fast, 110ms) ease,
    transform var(--motion-instant, 80ms) ease;
}
.message-action-btn:hover:not(:disabled) {
  opacity: 1;
  color: var(--peek-accent);
  background: color-mix(in srgb, var(--peek-accent) 12%, transparent);
}
.message-action-btn:active:not(:disabled) {
  transform: scale(0.94);
  background: color-mix(in srgb, var(--peek-accent) 18%, transparent);
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
.assistant-bubble {
  display: flex;
  flex-direction: column;
  gap: 6px;
  box-sizing: border-box;
  width: calc(100% - 2 * var(--chat-reply-inset, var(--peek-radius-composer, 16px)));
  max-width: 100%;
  min-width: 0;
  margin-left: var(--chat-reply-inset, var(--peek-radius-composer, 16px));
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
  font-size: var(--peek-agent-font-size, 13px);
  line-height: 1.7;
}
.assistant-bubble :deep(.agent-work),
.assistant-bubble :deep(.tool-activity-list),
.assistant-bubble :deep(.reasoning-block),
.assistant-bubble :deep(.image-analysis-card) {
  width: 100%;
  max-width: none;
  box-sizing: border-box;
}

.message-item.inject {
  display: flex;
  justify-content: flex-start;
  width: 100%;
}

.inject-card,
.message-item.inject :deep(.inject-card) {
  width: 100%;
  max-width: 100%;
}
</style>
