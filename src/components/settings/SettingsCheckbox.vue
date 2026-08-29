<template>
  <input
    :id="id"
    type="checkbox"
    class="settings-checkbox"
    :class="sizeClass"
    :checked="checked"
    :value="value"
    :disabled="disabled"
    @change="onChange"
    @click.stop
  />
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    id?: string;
    checked?: boolean;
    value?: string | number;
    disabled?: boolean;
    size?: "sm" | "md";
    indeterminate?: boolean;
  }>(),
  {
    disabled: false,
    size: "sm",
  },
);

const emit = defineEmits<{
  change: [event: Event];
}>();

const sizeClass = computed(() =>
  props.size === "md" ? "settings-checkbox-md" : "settings-checkbox-sm",
);

function onChange(event: Event) {
  emit("change", event);
}
</script>
