<script setup lang="ts">
import type { ScrollAreaScrollbarProps } from "reka-ui";
import type { HTMLAttributes } from "vue";
import { reactiveOmit } from "@vueuse/core";
import { ScrollAreaScrollbar, ScrollAreaThumb } from "reka-ui";
import { cn } from "@/lib/utils";

const props = withDefaults(
  defineProps<ScrollAreaScrollbarProps & { class?: HTMLAttributes["class"] }>(),
  {
    orientation: "vertical",
  },
);

const delegatedProps = reactiveOmit(props, "class");
</script>

<template>
  <ScrollAreaScrollbar
    data-slot="scroll-area-scrollbar"
    :data-orientation="orientation"
    v-bind="delegatedProps"
    :class="
      cn(
        'flex touch-none select-none p-0 transition-colors',
        'data-[orientation=vertical]:h-full data-[orientation=vertical]:w-1 data-[orientation=vertical]:border-l-0',
        'data-[orientation=horizontal]:h-1 data-[orientation=horizontal]:w-full data-[orientation=horizontal]:flex-col data-[orientation=horizontal]:border-t-0',
        props.class,
      )
    "
  >
    <ScrollAreaThumb data-slot="scroll-area-thumb" class="relative flex-1 rounded-full" />
  </ScrollAreaScrollbar>
</template>
