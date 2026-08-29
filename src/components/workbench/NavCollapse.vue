<template>
  <div class="nav-collapse" :class="{ 'is-collapsed': collapsed }">
    <div class="nav-collapse-inner" :aria-hidden="collapsed || undefined">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  collapsed: boolean;
}>();
</script>

<style scoped>
.nav-collapse {
  display: grid;
  grid-template-rows: 1fr;
  min-width: 0;
  max-width: 100%;
  opacity: 1;
  transition:
    grid-template-rows 280ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1)),
    opacity 220ms var(--motion-ease-out, cubic-bezier(0.16, 1, 0.3, 1));
}

.nav-collapse.is-collapsed {
  grid-template-rows: 0fr;
  opacity: 0;
  pointer-events: none;
}

.nav-collapse-inner {
  overflow: hidden;
  min-width: 0;
  min-height: 0;
}

@media (prefers-reduced-motion: reduce) {
  .nav-collapse {
    transition-duration: 1ms;
  }
}
</style>
