<template>
  <ReasoningBlock
    v-if="segment.type === 'reasoning'"
    :reasoning="segment.content"
    :streaming="streaming"
    :follow="follow"
    :language="language"
    :show-summary="showReasoningSummary"
    embedded
  />

  <ReasoningBlock
    v-else-if="segment.type === 'narration'"
    :reasoning="segment.content"
    :streaming="false"
    :language="language"
    :show-summary="showNarrationSummary"
    summary-key="executionDetails"
    embedded
  />

  <Markdown
    v-else-if="segment.type === 'content'"
    :content="segment.content"
    class="agent-work-content"
    @preview-image="emit('previewImage', $event)"
  />

  <ToolActivityList
    v-else-if="segment.type === 'inline'"
    :activities="segment.activities"
    :all-activities="allActivities"
    :operations="segment.operations"
    :cards-collapsed="false"
    @inspect-subagent="emit('inspectSubagent', $event)"
    @preview-image="emit('previewImage', $event)"
    @edit-from-image="emit('editFromImage', $event)"
  />

  <ToolActivityList
    v-else-if="segment.type === 'process' && !collapsible"
    :activities="segment.activities"
    :all-activities="allActivities"
    :operations="segment.operations"
    flat
    @inspect-subagent="emit('inspectSubagent', $event)"
    @preview-image="emit('previewImage', $event)"
    @edit-from-image="emit('editFromImage', $event)"
  />

  <section v-else-if="segment.type === 'process'" class="agent-work-details">
    <button
      type="button"
      class="agent-work-toggle"
      :aria-expanded="processOpen"
      @click="emit('toggleProcess', segment.id)"
    >
      <ChevronRight class="agent-work-chevron" :class="{ open: processOpen }" :size="12" />
      <span class="agent-work-label">{{ headline }}</span>
    </button>
    <div v-if="processOpen" class="agent-work-body">
      <ToolActivityList
        :activities="segment.activities"
        :all-activities="allActivities"
        :operations="segment.operations"
        :cards-collapsed="cardsCollapsed"
        flat
        @inspect-subagent="emit('inspectSubagent', $event)"
        @preview-image="emit('previewImage', $event)"
        @edit-from-image="emit('editFromImage', $event)"
      />
    </div>
  </section>
</template>

<script setup lang="ts">
import { ChevronRight } from "@lucide/vue";
import Markdown from "@/components/chat/Markdown.vue";
import ReasoningBlock from "@/components/chat/ReasoningBlock.vue";
import ToolActivityList from "@/components/chat/ToolActivityList.vue";
import type { ToolActivity } from "@/types/chat";
import type { AppLanguage } from "@/types/setting";

export type WorkSegment =
  | { type: "reasoning"; id: string; content: string }
  | { type: "narration"; id: string; content: string }
  | { type: "content"; id: string; content: string }
  | { type: "inline"; id: string; activities: ToolActivity[]; operations: boolean }
  | { type: "process"; id: string; activities: ToolActivity[]; operations: boolean };

defineProps<{
  segment: WorkSegment;
  streaming: boolean;
  follow: boolean;
  language?: AppLanguage;
  /** Hide nested "思考过程" chrome when already inside a completed fold. */
  showReasoningSummary?: boolean;
  showNarrationSummary?: boolean;
  allActivities: ToolActivity[];
  collapsible: boolean;
  processOpen: boolean;
  headline: string;
  cardsCollapsed: boolean;
}>();

const emit = defineEmits<{
  inspectSubagent: [activityId: string];
  previewImage: [source: string];
  editFromImage: [payload: import("@/services/chat/imageEditReference").ImageEditReferencePayload];
  toggleProcess: [id: string];
}>();
</script>

<style scoped>
.agent-work-details {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
}

.agent-work-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  min-height: 24px;
  padding: 0 2px;
  border: 0;
  border-radius: 0;
  background: transparent;
  color: var(--peek-muted);
  font-size: var(--peek-font-sm, 12px);
  font-weight: 400;
  line-height: 24px;
  cursor: pointer;
  text-align: left;
}

.agent-work-toggle:hover {
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 4%, transparent);
}

.agent-work-chevron {
  flex: none;
  color: var(--peek-faint);
  transition: transform 160ms ease;
}

.agent-work-chevron.open {
  transform: rotate(90deg);
}

.agent-work-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.agent-work-body {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 2px 0 2px 2px;
  border-left: 1.5px solid color-mix(in srgb, var(--peek-border) 70%, transparent);
}

.agent-work-content :deep(> *:first-child) {
  margin-top: 0;
}

.agent-work-content :deep(> *:last-child) {
  margin-bottom: 0;
}
</style>
