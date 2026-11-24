<template>
  <div class="extension-toolbar">
    <div class="extension-toolbar-navigation">
      <slot />
    </div>
    <div class="extension-toolbar-actions">
      <slot name="actions" />
    </div>
  </div>
</template>

<style scoped>
.extension-toolbar {
  display: flex;
  flex: none;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 8px 12px;
  min-height: 36px;
  margin-bottom: 16px;
}

.extension-toolbar-navigation,
.extension-toolbar-actions {
  min-width: 0;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}

.extension-toolbar-actions {
  margin-inline-start: auto;
}

.extension-toolbar-navigation :deep(.settings-tabs) {
  margin: 0;
}
</style>
