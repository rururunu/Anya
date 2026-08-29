<template>
  <div class="workbench-session-list" :class="`is-${variant}`">
    <template v-for="session in rootSessions" :key="session.sessionId">
      <div
        class="session-row"
        :class="{
          active: session.sessionId === activeSessionId,
          dragging: session.sessionId === draggedSessionId,
          'is-archiving': archiveVisual(session.sessionId) === 'shown',
          'is-archive-leaving': archiveVisual(session.sessionId) === 'leaving',
          'is-title-generating': isTitleGenerating(session.sessionId),
        }"
        role="button"
        tabindex="0"
        :title="draggedSessionId ? undefined : sessionHoverText(session)"
        @pointerdown="onRowPointerDown($event, session)"
        @click="onSelect(session.sessionId)"
        @keydown.enter.self="onSelect(session.sessionId)"
        @keydown.space.self.prevent="onSelect(session.sessionId)"
        @dragstart.prevent
      >
        <strong>
          <span
            class="session-preview"
            :class="{ 'is-generating': isTitleGenerating(session.sessionId) }"
          >
            {{ sessionPreviewText(session) }}
          </span>
          <span v-if="archiveVisual(session.sessionId)" class="session-archived-mark">
            {{ archivedLabel }}
          </span>
        </strong>
        <span
          class="session-status"
          role="status"
          :title="sessionStatusLabel(session.sessionId)"
          :aria-label="sessionStatusLabel(session.sessionId) || undefined"
        >
          <Archive
            v-if="archiveVisual(session.sessionId)"
            :size="12"
            class="archived-icon"
            aria-hidden="true"
          />
          <ShieldAlert
            v-else-if="attentionSessionIds.includes(session.sessionId)"
            :size="13"
            class="attention-icon"
          />
          <LoaderCircle
            v-else-if="isTitleGenerating(session.sessionId)"
            :size="13"
            class="running-icon title-generating-icon"
          />
          <LoaderCircle
            v-else-if="runningSessionIds.includes(session.sessionId)"
            :size="13"
            class="running-icon"
          />
          <span v-else-if="unreadSessionIds.includes(session.sessionId)" class="unread-dot" />
          <PenLine v-else-if="hasDraft(session.sessionId)" :size="12" class="draft-icon" />
        </span>
        <button
          v-if="!archiveVisual(session.sessionId)"
          type="button"
          class="session-action session-action-menu"
          :title="sessionMenuLabel"
          @pointerdown.stop
          @click.stop="openMenu($event, session)"
        >
          <Ellipsis :size="13" />
        </button>
        <button
          v-if="!archiveVisual(session.sessionId)"
          type="button"
          class="session-action"
          :title="archiveLabel"
          @pointerdown.stop
          @click.stop="emit('archive', session.sessionId)"
        >
          <Archive :size="13" />
        </button>
      </div>

      <div
        v-if="childSessions(session.sessionId).length"
        class="session-sublist"
        :class="{ open: shouldShowChildren(session.sessionId) }"
      >
        <div
          v-for="child in childSessions(session.sessionId)"
          :key="child.sessionId"
          class="session-row session-row-subagent"
          :class="{
            active: child.sessionId === activeSessionId,
            dragging: child.sessionId === draggedSessionId,
          }"
          role="button"
          tabindex="0"
          :title="draggedSessionId ? undefined : sessionHoverText(child)"
          @pointerdown="onRowPointerDown($event, child)"
          @click="onSelect(child.sessionId)"
          @keydown.enter.self="onSelect(child.sessionId)"
          @keydown.space.self.prevent="onSelect(child.sessionId)"
          @dragstart.prevent
        >
          <SubagentIcon :size="12" class="subagent-leading" />
          <strong>{{ displaySubagentPreview(child) }}</strong>
          <span
            class="session-status"
            role="status"
            :title="sessionStatusLabel(child.sessionId)"
            :aria-label="sessionStatusLabel(child.sessionId) || undefined"
          >
            <LoaderCircle
              v-if="runningSessionIds.includes(child.sessionId)"
              :size="12"
              class="running-icon"
            />
            <span v-else-if="unreadSessionIds.includes(child.sessionId)" class="unread-dot" />
          </span>
          <button
            type="button"
            class="session-action"
            :title="dismissSubagentLabel || archiveLabel"
            @pointerdown.stop
            @click.stop="emit('dismiss', child.sessionId)"
          >
            <X :size="12" />
          </button>
        </div>
      </div>
    </template>
    <Teleport to="body">
      <div
        v-if="menuSession"
        class="session-menu-flyout"
        :style="menuStyle"
        @pointerdown.stop
        @click.stop
      >
        <button type="button" @click="onRename(menuSession)">{{ renameLabel }}</button>
        <button
          type="button"
          :disabled="isTitleGenerating(menuSession.sessionId)"
          @click="onRegenerate(menuSession.sessionId)"
        >
          {{ regenerateTitleLabel }}
        </button>
        <button type="button" class="danger" @click="onDelete(menuSession.sessionId)">
          {{ deleteLabel }}
        </button>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { Archive, Ellipsis, LoaderCircle, PenLine, ShieldAlert, X } from "@lucide/vue";
import SubagentIcon from "@/components/chat/SubagentIcon.vue";
import type { ArchiveVisualState } from "@/composables/workbench/useWorkbenchSessions";
import { formatSessionPreview } from "@/services/chat/sessionPreview";
import {
  isSubagentSessionId,
  parentSessionId,
  rootSessionId,
} from "@/services/chat/subagentSession";
import { tr } from "@/services/i18n";
import type { ChatSessionSummary } from "@/types/chat";
import type { AppLanguage } from "@/types/setting";

const props = defineProps<{
  sessions: ChatSessionSummary[];
  allSessions?: ChatSessionSummary[];
  activeSessionId: string;
  language: AppLanguage;
  untitledLabel: string;
  archiveLabel: string;
  archivedLabel: string;
  renameLabel: string;
  regenerateTitleLabel: string;
  deleteLabel: string;
  generatingTitleLabel: string;
  sessionMenuLabel: string;
  archiveVisualState?: Record<string, ArchiveVisualState>;
  dismissSubagentLabel?: string;
  runningSessionIds: string[];
  titleGeneratingSessionIds?: string[];
  attentionSessionIds: string[];
  unreadSessionIds: string[];
  draftSessionIds?: string[];
  variant?: "workspace" | "quick";
  draggedSessionId?: string;
}>();

const emit = defineEmits<{
  select: [sessionId: string];
  archive: [sessionId: string];
  dismiss: [sessionId: string];
  rename: [session: ChatSessionSummary];
  regenerateTitle: [sessionId: string];
  delete: [sessionId: string];
  sessionPointerDown: [event: PointerEvent, session: ChatSessionSummary];
}>();

const menuSession = ref<ChatSessionSummary | null>(null);
const menuStyle = ref<Record<string, string>>({});

function closeMenu() {
  menuSession.value = null;
}

function openMenu(event: MouseEvent, session: ChatSessionSummary) {
  if (archiveVisual(session.sessionId)) return;
  const target = event.currentTarget as HTMLElement | null;
  const rect = target?.getBoundingClientRect();
  if (!rect) return;
  menuSession.value = session;
  menuStyle.value = {
    top: `${rect.bottom + 4}px`,
    left: `${Math.max(8, rect.right - 168)}px`,
  };
}

function onRename(session: ChatSessionSummary) {
  closeMenu();
  emit("rename", session);
}

function onRegenerate(sessionId: string) {
  if (isTitleGenerating(sessionId)) return;
  closeMenu();
  emit("regenerateTitle", sessionId);
}

function onDelete(sessionId: string) {
  closeMenu();
  emit("delete", sessionId);
}

function isTitleGenerating(sessionId: string) {
  return (props.titleGeneratingSessionIds ?? []).includes(sessionId);
}

function sessionPreviewText(session: ChatSessionSummary) {
  if (isTitleGenerating(session.sessionId)) {
    return props.generatingTitleLabel;
  }
  return displayPreview(session);
}

function onDocumentPointerDown(event: PointerEvent) {
  const target = event.target;
  if (target instanceof Element && target.closest(".session-menu-flyout")) {
    return;
  }
  closeMenu();
}

onMounted(() => {
  document.addEventListener("pointerdown", onDocumentPointerDown);
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", onDocumentPointerDown);
});

const sessionPool = computed(() => props.allSessions ?? props.sessions);

const rootSessions = computed(() =>
  props.sessions.filter((session) => !isSubagentSessionId(session.sessionId)),
);

function archiveVisual(sessionId: string) {
  return props.archiveVisualState?.[sessionId];
}

function childSessions(parentId: string) {
  return sessionPool.value.filter((session) => parentSessionId(session.sessionId) === parentId);
}

function shouldShowChildren(parentId: string) {
  return childSessions(parentId).length > 0;
}

function onRowPointerDown(event: PointerEvent, session: ChatSessionSummary) {
  if (archiveVisual(session.sessionId)) return;
  emit("sessionPointerDown", event, session);
}

function onSelect(sessionId: string) {
  if (props.draggedSessionId || archiveVisual(sessionId)) return;
  emit("select", sessionId);
}

function displayPreview(session: ChatSessionSummary) {
  return formatSessionPreview(session.preview || "") || props.untitledLabel;
}

function displaySubagentPreview(session: ChatSessionSummary) {
  const preview = formatSessionPreview(session.preview || "") || props.untitledLabel;
  const badge = tr(props.language, "subagent.badge");
  return preview.startsWith(badge) ? preview : `${badge} ${preview}`;
}

function hasDraft(sessionId: string) {
  return (props.draftSessionIds ?? []).includes(sessionId);
}

function formatSessionTime(timestamp: number) {
  return new Intl.DateTimeFormat(props.language, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(timestamp));
}

function formatTurnCount(count: number) {
  return props.language === "zh-CN"
    ? `${count} \u8f6e\u5bf9\u8bdd`
    : `${count} ${count === 1 ? "turn" : "turns"}`;
}

function sessionHoverText(session: ChatSessionSummary) {
  const root = rootSessionId(session.sessionId);
  const rootNote =
    isSubagentSessionId(session.sessionId) && root !== session.sessionId
      ? `\n${tr(props.language, "subagent.parentSession")}: ${root}`
      : "";
  return `${formatSessionTime(session.updatedAt)}\n${formatTurnCount(session.turnCount)}${rootNote}`;
}

function sessionStatusLabel(sessionId: string) {
  if (archiveVisual(sessionId)) {
    return props.archivedLabel;
  }
  if (props.attentionSessionIds.includes(sessionId)) {
    return props.language === "zh-CN" ? "需要处理请求" : "Action required";
  }
  if (props.runningSessionIds.includes(sessionId)) {
    return props.language === "zh-CN" ? "运行中" : "Running";
  }
  if (isTitleGenerating(sessionId)) {
    return props.generatingTitleLabel;
  }
  if (props.unreadSessionIds.includes(sessionId)) {
    return props.language === "zh-CN" ? "任务已完成" : "Task completed";
  }
  if (hasDraft(sessionId)) {
    return props.language === "zh-CN" ? "有未发送内容" : "Unsent draft";
  }
  return "";
}
</script>

<style scoped>
.workbench-session-list {
  box-sizing: border-box;
  width: 100%;
  min-width: 0;
  max-width: 100%;
}
.workbench-session-list.is-workspace {
  width: calc(100% - 14px);
  max-width: calc(100% - 14px);
  margin: 2px 0 4px 14px;
  padding: 2px 0 4px 10px;
  border-left: 1px solid color-mix(in srgb, var(--peek-border) 72%, transparent);
  user-select: none;
  -webkit-user-select: none;
}
.workbench-session-list.is-quick {
  padding: 2px 0 0;
  user-select: none;
  -webkit-user-select: none;
}
.session-sublist {
  display: none;
  margin: 0 0 2px 10px;
  padding-left: 8px;
  border-left: 1px solid color-mix(in srgb, var(--peek-border) 55%, transparent);
}
.session-sublist.open {
  display: block;
}
.session-row {
  position: relative;
  box-sizing: border-box;
  width: 100%;
  min-width: 0;
  max-width: 100%;
  height: var(--peek-control-row, 30px);
  display: flex;
  align-items: center;
  padding: 0 62px 0 8px;
  border-radius: var(--peek-radius-sm, 6px);
  background: transparent;
  color: var(--peek-text);
  cursor: pointer;
  user-select: none;
  -webkit-user-select: none;
  touch-action: none;
  text-align: left;
  transition:
    background-color var(--motion-fast, 110ms) var(--motion-ease-out, ease),
    color var(--motion-fast, 110ms) var(--motion-ease-out, ease),
    opacity var(--motion-normal, 160ms) var(--motion-ease-out, ease);
}
.session-row.is-archiving {
  opacity: 0.88;
  pointer-events: none;
  background: color-mix(in srgb, var(--peek-muted) 8%, transparent);
}
.session-row.is-archive-leaving {
  opacity: 0;
  pointer-events: none;
}
.session-row-subagent {
  height: 28px;
  padding-left: 6px;
}
.session-row:hover {
  background: var(--peek-row-hover);
}
.session-row.active {
  background: var(--peek-row-active);
}
.session-row.active > strong {
  font-weight: 600;
  color: var(--peek-text);
}
.session-row.is-archiving > strong,
.session-row.is-archive-leaving > strong {
  font-weight: 500;
  color: var(--peek-muted);
}
.session-row:focus-visible {
  outline: none;
  box-shadow: var(--peek-focus-ring);
}
.session-row.dragging {
  cursor: grabbing;
  opacity: 0.4;
}
.is-quick .session-row {
  padding: 0 62px 0 9px;
}
.subagent-leading {
  flex: none;
  margin-right: 2px;
  color: var(--peek-faint);
}
.session-row > strong {
  min-width: 0;
  display: flex;
  align-items: center;
  flex: 1;
  overflow: hidden;
  font-size: var(--peek-font-sm, 12px);
  font-weight: 500;
  color: color-mix(in srgb, var(--peek-text) 92%, var(--peek-muted));
  user-select: none;
  -webkit-user-select: none;
  pointer-events: none;
}
.session-preview {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.session-preview.is-generating {
  background: linear-gradient(
    90deg,
    var(--peek-muted) 0%,
    color-mix(in srgb, var(--peek-text) 72%, var(--peek-muted)) 45%,
    var(--peek-muted) 90%
  );
  background-size: 200% 100%;
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
  animation: session-title-shimmer 1.8s linear infinite;
}
.session-row.is-title-generating .session-action-menu {
  opacity: 0.45;
  pointer-events: none;
}
.title-generating-icon {
  color: color-mix(in srgb, var(--peek-accent) 82%, var(--peek-muted));
}
.session-archived-mark {
  flex: none;
  margin-left: 6px;
  padding: 1px 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--peek-muted) 12%, transparent);
  color: var(--peek-muted);
  font-size: 10px;
  font-weight: 600;
  line-height: 14px;
  letter-spacing: 0.01em;
}
.session-row-subagent > strong {
  display: block;
  font-size: 11px;
  color: var(--peek-muted);
}
.session-status {
  flex: none;
  width: 16px;
  height: 16px;
  display: inline-grid;
  place-items: center;
  color: var(--peek-accent);
}
.archived-icon {
  color: var(--peek-muted);
}
.running-icon {
  animation: session-running-spin 0.9s linear infinite;
}
.attention-icon {
  color: var(--peek-warning, #d97706);
}
.draft-icon {
  color: var(--peek-muted);
  opacity: 0.85;
}
.unread-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--peek-accent);
}
.session-action {
  position: absolute;
  top: 0;
  bottom: 0;
  right: 2px;
  margin-block: auto;
  flex: none;
  width: var(--peek-control-icon, 28px);
  height: var(--peek-control-icon, 28px);
  display: inline-grid;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: var(--peek-radius-sm, 6px);
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
  opacity: 0;
  transform: none;
  transform-origin: center;
  transition:
    opacity var(--motion-fast, 110ms) ease,
    background-color var(--motion-fast, 110ms) ease,
    color var(--motion-fast, 110ms) ease;
}
.session-row:hover .session-action,
.session-row:focus-within .session-action {
  opacity: 1;
}
.session-action:hover {
  color: var(--peek-text);
  background: var(--peek-hover-bg);
}
.session-action:active {
  transform: none;
  background: var(--peek-press-bg);
}
.session-action-menu {
  right: 30px;
}
.session-menu-flyout {
  position: fixed;
  z-index: 60;
  min-width: 168px;
  padding: 4px;
  border: 1px solid var(--peek-border);
  border-radius: var(--peek-radius-sm, 6px);
  background: var(--peek-dialog-bg, var(--peek-surface));
  box-shadow: var(--peek-elev-md);
}
.session-menu-flyout button {
  display: block;
  width: 100%;
  padding: 7px 10px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--peek-text);
  font: inherit;
  font-size: var(--peek-font-sm, 12px);
  text-align: left;
  cursor: pointer;
}
.session-menu-flyout button:hover {
  background: var(--peek-hover-bg);
}
.session-menu-flyout button:disabled {
  opacity: 0.45;
  cursor: default;
}
.session-menu-flyout button:disabled:hover {
  background: transparent;
}
.session-menu-flyout button.danger {
  color: var(--peek-danger);
}
.session-menu-flyout button.danger:hover {
  background: color-mix(in srgb, var(--peek-danger) 12%, transparent);
}
@keyframes session-running-spin {
  to {
    transform: rotate(360deg);
  }
}
@keyframes session-title-shimmer {
  0% {
    background-position: 100% 0;
  }
  100% {
    background-position: -100% 0;
  }
}
@media (prefers-reduced-motion: reduce) {
  .running-icon {
    animation-duration: 2.4s;
  }
  .session-preview.is-generating {
    animation: none;
    color: var(--peek-muted);
    background: none;
    -webkit-background-clip: initial;
    background-clip: initial;
  }
  .session-row.is-archive-leaving {
    transition: none;
  }
}
</style>
