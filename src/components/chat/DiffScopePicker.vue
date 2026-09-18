<template>
  <PopoverRoot v-model:open="open">
    <PopoverTrigger as-child>
      <button
        type="button"
        class="diff-scope-trigger"
        :class="{ open }"
        :aria-label="tr(settingStore.language, 'diffScope')"
        :aria-expanded="open"
        aria-haspopup="listbox"
      >
        <span>{{ currentLabel }}</span>
        <ChevronDown :size="12" :stroke-width="2.25" class="chevron" aria-hidden="true" />
      </button>
    </PopoverTrigger>
    <PopoverPortal>
      <PopoverContent
        class="diff-scope-menu"
        side="bottom"
        align="start"
        :side-offset="6"
        :collision-padding="8"
      >
        <OptionPicker
          compact
          :options="options"
          :selected-id="modelValue"
          :selected-index="hoverIndex"
          :ariaLabel="tr(settingStore.language, 'diffScope')"
          @hover="hoverIndex = $event"
          @select="onSelect"
        />
      </PopoverContent>
    </PopoverPortal>
  </PopoverRoot>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ChevronDown, GitBranch, History, MessagesSquare } from "@lucide/vue";
import { PopoverContent, PopoverPortal, PopoverRoot, PopoverTrigger } from "reka-ui";
import OptionPicker from "@/components/chat/input/OptionPicker.vue";
import type { DiffScope } from "@/composables/chat/useDiffScope";
import { tr } from "@/services/i18n";
import { useSettingStore } from "@/stores/setting";

const props = defineProps<{
  modelValue: DiffScope;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: DiffScope];
}>();

const settingStore = useSettingStore();
const open = ref(false);
const hoverIndex = ref(0);

const options = computed(() => [
  {
    id: "round",
    label: tr(settingStore.language, "diffScopeRound"),
    icon: History,
  },
  {
    id: "uncommitted",
    label: tr(settingStore.language, "diffScopeUncommitted"),
    icon: GitBranch,
  },
  {
    id: "session",
    label: tr(settingStore.language, "diffScopeSession"),
    icon: MessagesSquare,
  },
]);

const currentLabel = computed(
  () => options.value.find((item) => item.id === props.modelValue)?.label ?? options.value[0].label,
);

watch(open, (value) => {
  if (!value) return;
  const index = options.value.findIndex((item) => item.id === props.modelValue);
  hoverIndex.value = index >= 0 ? index : 0;
});

function onSelect(id: string) {
  if (id === "round" || id === "uncommitted" || id === "session") {
    emit("update:modelValue", id);
  }
  open.value = false;
}
</script>

<style scoped>
.diff-scope-trigger {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  height: 22px;
  padding: 0 4px 0 8px;
  border: 0;
  border-radius: 4px;
  background: color-mix(in srgb, var(--peek-text) 6%, transparent);
  color: var(--peek-text);
  font: 600 11px/1 var(--peek-font-sans);
  white-space: nowrap;
  cursor: pointer;
}
.diff-scope-trigger:hover,
.diff-scope-trigger.open,
.diff-scope-trigger:focus-visible {
  background: color-mix(in srgb, var(--peek-text) 8%, transparent);
  outline: none;
}
.diff-scope-trigger .chevron {
  opacity: 0.42;
  transition:
    transform 140ms ease,
    opacity 140ms ease;
}
.diff-scope-trigger.open .chevron {
  transform: rotate(180deg);
  opacity: 0.72;
}
</style>

<style>
.diff-scope-menu {
  z-index: 80;
  width: 168px;
  padding: 0;
  border: 0;
  background: transparent;
  outline: none;
}
.diff-scope-menu .command-list {
  border: 1px solid color-mix(in srgb, var(--peek-text) 12%, transparent);
  border-radius: 10px;
  background: var(--peek-surface, var(--peek-list-bg));
  box-shadow: 0 10px 28px color-mix(in srgb, #000 16%, transparent);
}
.diff-scope-menu .command-list.compact {
  --command-row-height: 30px;
}
</style>
