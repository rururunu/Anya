<template>
  <div class="subagent-conversation">
    <header class="subagent-conversation-header">
      <div class="subagent-identity">
        <SubagentIcon :status="entry?.status ?? 'running'" :size="15" />
        <div class="subagent-identity-copy">
          <strong :title="entry?.title">{{ entry?.title }}</strong>
          <span v-if="entry?.model" class="subagent-model">{{ entry.model }}</span>
        </div>
        <span class="agent-status" :class="entry?.status ?? 'running'">
          {{ statusLabel }}
        </span>
      </div>
      <p class="subagent-readonly-hint">{{ tr(language, "subagent.readonlyHint") }}</p>
    </header>

    <div v-if="entry" class="subagent-conversation-body peek-scrollbar">
      <section class="subagent-task-block">
        <h3>{{ tr(language, "subagent.taskDetails") }}</h3>
        <Markdown :content="entry.task" />
      </section>

      <AgentWorkDetails
        v-if="viewMessage"
        :message="viewMessage"
        :language="language"
        :show-reasoning="showReasoning"
        :display-mode="displayMode"
      />

      <Markdown
        v-if="viewMessage?.content"
        class="subagent-completion"
        :content="viewMessage.content"
      />

      <p
        v-if="entry.status === 'running' && !viewMessage?.toolActivities?.length"
        class="subagent-waiting"
      >
        {{ tr(language, "subagent.waiting") }}
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, watchEffect } from "vue";
import AgentWorkDetails from "@/components/chat/AgentWorkDetails.vue";
import Markdown from "@/components/chat/Markdown.vue";
import SubagentIcon from "@/components/chat/SubagentIcon.vue";
import {
  buildSubagentViewMessage,
  entryIdFromPanelSession,
  findSubagentEntry,
} from "@/services/chat/subagentPanel";
import { tr } from "@/services/i18n";
import { useSubagentSessionStore } from "@/stores/subagentSessions";
import type { ChatMessage } from "@/types/chat";
import type { AgentWorkDisplay, AppLanguage } from "@/types/setting";

const props = defineProps<{
  sessionId: string;
  parentMessages: ChatMessage[];
  language: AppLanguage;
  showReasoning: boolean;
  displayMode: AgentWorkDisplay;
}>();

const subagentStore = useSubagentSessionStore();

const record = computed(() => subagentStore.records[props.sessionId]);

const entryId = computed(() => {
  return record.value?.entryId ?? entryIdFromPanelSession(props.sessionId) ?? "";
});

const parentActivities = computed(() =>
  props.parentMessages.flatMap((message) => message.toolActivities ?? []),
);

const entry = computed(() => {
  if (!entryId.value) return null;
  return findSubagentEntry(parentActivities.value, entryId.value, props.language);
});

const parentActivity = computed(() => {
  if (!entry.value) return null;
  return parentActivities.value.find((activity) => activity.id === entry.value?.parentActivityId);
});

const viewMessage = computed(() => {
  if (!entry.value || !parentActivity.value) return null;
  return buildSubagentViewMessage(entry.value, parentActivity.value, parentActivities.value);
});

const statusLabel = computed(() => {
  const status = entry.value?.status ?? "running";
  if (status === "running") return tr(props.language, "subagent.running");
  if (status === "error") return tr(props.language, "subagent.failed");
  return tr(props.language, "subagent.done");
});

// Keep registry preview in sync while the sub-agent runs.
const liveTitle = computed(() => entry.value?.title ?? "");

watchEffect(() => {
  const title = liveTitle.value;
  const current = subagentStore.records[props.sessionId];
  if (!current || !title || current.preview === title) return;
  subagentStore.upsert({ ...current, preview: title });
});
</script>

<style scoped>
.subagent-conversation {
  display: flex;
  flex-direction: column;
  min-height: 0;
  height: 100%;
}

.subagent-conversation-header {
  flex: none;
  padding: 12px 18px 10px;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-border) 55%, transparent);
}

.subagent-identity {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.subagent-identity-copy {
  min-width: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.subagent-identity-copy strong {
  overflow: hidden;
  font-size: 13px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.subagent-model {
  color: var(--peek-faint);
  font: 10px/1.3 var(--font-mono);
}

.agent-status {
  flex: none;
  font-size: 11px;
  color: var(--peek-muted);
}

.agent-status.running {
  color: var(--peek-accent);
}

.agent-status.error {
  color: var(--peek-danger);
}

.subagent-readonly-hint {
  margin: 8px 0 0;
  color: var(--peek-faint);
  font-size: 11px;
  line-height: 1.45;
}

.subagent-conversation-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 14px 18px 24px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.subagent-task-block h3 {
  margin: 0 0 6px;
  font-size: 11px;
  font-weight: 600;
  color: var(--peek-muted);
}

.subagent-waiting {
  margin: 0;
  color: var(--peek-muted);
  font-size: 12px;
}

.subagent-completion :deep(> *:first-child) {
  margin-top: 0;
}
</style>
