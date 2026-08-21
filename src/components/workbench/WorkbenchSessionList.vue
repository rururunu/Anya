<template>
  <div class="workbench-session-list" :class="`is-${variant}`">
    <div
      v-for="session in sessions"
      :key="session.sessionId"
      class="session-row"
      :class="{ active: session.sessionId === activeSessionId }"
      role="button"
      tabindex="0"
      :title="sessionHoverText(session)"
      @click="emit('select', session.sessionId)"
      @keydown.enter.self="emit('select', session.sessionId)"
      @keydown.space.self.prevent="emit('select', session.sessionId)"
    >
      <strong>{{ displayPreview(session) }}</strong>
      <span
        class="session-status"
        role="status"
        :title="sessionStatusLabel(session.sessionId)"
        :aria-label="sessionStatusLabel(session.sessionId) || undefined"
      >
        <ShieldAlert
          v-if="attentionSessionIds.includes(session.sessionId)"
          :size="13"
          class="attention-icon"
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
        type="button"
        class="archive-session"
        :title="archiveLabel"
        @click.stop="emit('archive', session.sessionId)"
      >
        <Archive :size="13" />
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Archive, LoaderCircle, PenLine, ShieldAlert } from "@lucide/vue";
import { formatSessionPreview } from "@/services/chat/sessionPreview";
import type { ChatSessionSummary } from "@/types/chat";
import type { AppLanguage } from "@/types/setting";

const props = defineProps<{
  sessions: ChatSessionSummary[];
  activeSessionId: string;
  language: AppLanguage;
  untitledLabel: string;
  archiveLabel: string;
  runningSessionIds: string[];
  attentionSessionIds: string[];
  unreadSessionIds: string[];
  draftSessionIds?: string[];
  variant?: "workspace" | "quick";
}>();
const emit = defineEmits<{
  select: [sessionId: string];
  archive: [sessionId: string];
}>();

function displayPreview(session: ChatSessionSummary) {
  return formatSessionPreview(session.preview || "") || props.untitledLabel;
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
  return `${formatSessionTime(session.updatedAt)}\n${formatTurnCount(session.turnCount)}`;
}

function sessionStatusLabel(sessionId: string) {
  if (props.attentionSessionIds.includes(sessionId)) {
    return props.language === "zh-CN" ? "需要处理请求" : "Action required";
  }
  if (props.runningSessionIds.includes(sessionId)) {
    return props.language === "zh-CN" ? "运行中" : "Running";
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
.workbench-session-list.is-workspace {
  padding: 2px 0 2px 22px;
}
.workbench-session-list.is-quick {
  padding: 3px 0 0;
}
.session-row {
  position: relative;
  width: 100%;
  height: var(--peek-control-row, 30px);
  display: flex;
  align-items: center;
  padding: 0 5px 0 7px;
  border-radius: var(--peek-radius-sm, 6px);
  background: transparent;
  color: var(--peek-text);
  cursor: pointer;
  text-align: left;
}
.session-row:hover {
  background: var(--peek-row-hover);
}
.session-row.active {
  background: var(--peek-row-active);
}
.is-quick .session-row {
  padding-left: 9px;
}
.session-row > strong {
  min-width: 0;
  display: block;
  flex: 1;
  overflow: hidden;
  font-size: var(--peek-font-sm, 12px);
  font-weight: 500;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.session-status {
  flex: none;
  width: 16px;
  height: 16px;
  display: inline-grid;
  place-items: center;
  color: var(--peek-accent);
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
.archive-session {
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
}
.session-row:hover .archive-session,
.session-row:focus-within .archive-session {
  opacity: 1;
}
.archive-session:hover {
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 8%, transparent);
}
@keyframes session-running-spin {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .running-icon {
    animation-duration: 1.8s;
  }
}
</style>
