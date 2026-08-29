<template>
  <button
    type="button"
    class="setting-toggle"
    :class="{ active: modelValue, 'is-compact': compact }"
    :aria-pressed="modelValue"
    :disabled="disabled"
    @click="onClick"
  >
    <span class="setting-toggle-knob" />
  </button>
</template>

<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    modelValue: boolean;
    disabled?: boolean;
    compact?: boolean;
  }>(),
  {
    disabled: false,
    compact: false,
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
  click: [event: MouseEvent];
}>();

function onClick(event: MouseEvent) {
  if (props.disabled) return;
  emit("click", event);
  if (event.defaultPrevented) return;
  emit("update:modelValue", !props.modelValue);
}
</script>
