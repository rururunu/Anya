<script setup lang="ts">
import type { HTMLAttributes } from "vue";
import { ref } from "vue";
import { Eye, EyeOff } from "@lucide/vue";
import { cn } from "@/lib/utils";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";

const props = defineProps<{
  modelValue?: string;
  class?: HTMLAttributes["class"];
  placeholder?: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
  blur: [];
}>();

const visible = ref(false);
const settingStore = useSettingStore();

function onInput(event: Event) {
  emit("update:modelValue", (event.target as HTMLInputElement).value);
}
</script>

<template>
  <div class="relative w-full">
    <input
      :value="modelValue"
      type="text"
      :placeholder="placeholder"
      spellcheck="false"
      autocomplete="off"
      :class="
        cn(
          'border-input focus-visible:border-foreground/25 h-8 w-full min-w-0 rounded-lg border bg-transparent py-1 pl-2.5 pr-8 font-mono text-xs outline-none transition-colors focus-visible:ring-0 placeholder:text-muted-foreground',
          !visible && 'secret-masked',
          props.class,
        )
      "
      @input="onInput"
      @blur="emit('blur')"
    />
    <button
      type="button"
      class="absolute top-1/2 right-1 flex size-6 -translate-y-1/2 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-sidebar-accent hover:text-foreground"
      :aria-label="tr(settingStore.language, visible ? 'hideSecret' : 'showSecret')"
      tabindex="-1"
      @mousedown.prevent
      @click="visible = !visible"
    >
      <EyeOff v-if="visible" class="size-3.5 shrink-0 opacity-80" />
      <Eye v-else class="size-3.5 shrink-0 opacity-80" />
    </button>
  </div>
</template>

<style scoped>
.secret-masked {
  -webkit-text-security: disc;
}
</style>
