<template>
  <span class="chat-avatar-frame" aria-hidden="true"><slot /></span>
</template>

<style scoped>
.chat-avatar-frame {
  box-sizing: border-box;
  display: flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 28px;
  min-width: 28px;
  height: 28px;
  border: 1px solid var(--peek-border, rgba(0, 0, 0, 0.18));
  border-radius: 50%;
  background: var(--peek-surface, #fff);
  overflow: hidden;
  color: var(--peek-muted);
}
.chat-avatar-frame :deep(svg) {
  width: 18px;
  height: 18px;
}
</style>
