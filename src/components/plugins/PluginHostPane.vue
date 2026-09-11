<template>
  <div
    ref="root"
    class="plugin-host-pane"
    :class="{ 'is-inactive': active === false, 'is-fit-content': fit === 'content' }"
  />
</template>

<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { usePluginsStore, type PluginMount } from "@/stores/plugins";
import { useSettingStore } from "@/stores/setting";

/** How long a mount gets before an empty view is flagged (async renders get a grace window). */
const EMPTY_CHECK_DELAY_MS = 600;

const props = withDefaults(
  defineProps<{
    mountFn: PluginMount | null;
    /** When false, hide without tearing down (PTY / long-running UI stays). Omit to mount immediately. */
    active?: boolean;
    /** Attributed to error records; omit for non-plugin usage. */
    pluginId?: string;
    /**
     * `fill` (default) stretches to a sized parent (sidebar / workbench).
     * `content` sizes to the plugin's DOM so a 0-height flex ancestor cannot clip it.
     */
    fit?: "fill" | "content";
  }>(),
  { active: true, fit: "fill" },
);

const root = ref<HTMLElement | null>(null);
let teardown: void | (() => void);
let mounted = false;
let mountInFlight = false;
let emptyCheckTimer: ReturnType<typeof setTimeout> | null = null;
let emptyPlaceholder: HTMLElement | null = null;
let emptyObserver: MutationObserver | null = null;

// `props.active` can flip between the two checks below (across the `await`);
// read it through a function each time so TS doesn't narrow it to a literal.
function isInactive(): boolean {
  return props.active === false;
}

async function ensureMounted() {
  if (isInactive() || mounted || mountInFlight) return;
  mountInFlight = true;
  await nextTick();
  try {
    if (isInactive() || mounted) return;
    if (props.mountFn && root.value) {
      try {
        teardown = props.mountFn(root.value) ?? undefined;
        mounted = true;
        if (props.pluginId) scheduleEmptyCheck();
      } catch (error) {
        if (props.pluginId) {
          usePluginsStore().recordError({
            pluginId: props.pluginId,
            phase: "mount",
            message: error instanceof Error ? error.message : String(error),
          });
        }
        console.warn("plugin mount failed", props.pluginId, error);
      }
    }
  } finally {
    mountInFlight = false;
  }
}

function isRootEmpty(): boolean {
  const el = root.value;
  return Boolean(el && el.childElementCount === 0 && !el.textContent?.trim());
}

function clearEmptyCheck() {
  if (emptyCheckTimer) clearTimeout(emptyCheckTimer);
  emptyCheckTimer = null;
  emptyObserver?.disconnect();
  emptyObserver = null;
  emptyPlaceholder?.remove();
  emptyPlaceholder = null;
}

/**
 * Flags a `mount()` that returned without rendering anything, instead of
 * leaving the user staring at a blank pane with no clue whether the plugin
 * is broken or just slow. Auto-clears if content shows up later (async render).
 */
function scheduleEmptyCheck() {
  clearEmptyCheck();
  emptyCheckTimer = setTimeout(() => {
    emptyCheckTimer = null;
    if (!mounted || !root.value || !isRootEmpty()) return;
    const isZh = useSettingStore().language === "zh-CN";
    const el = document.createElement("div");
    el.className = "plugin-host-empty";
    el.textContent = isZh
      ? "这个插件视图没有渲染任何内容，可能是插件本身的问题。"
      : "This plugin view rendered nothing — likely a bug in the plugin itself.";
    root.value.appendChild(el);
    emptyPlaceholder = el;
    if (props.pluginId) {
      usePluginsStore().recordError({
        pluginId: props.pluginId,
        phase: "mount",
        message: "mount() completed without rendering any content into the view",
      });
    }
    emptyObserver = new MutationObserver(() => {
      if (root.value && root.value.childElementCount > 1) clearEmptyCheck();
    });
    emptyObserver.observe(root.value, { childList: true });
  }, EMPTY_CHECK_DELAY_MS);
}

function dispose() {
  clearEmptyCheck();
  if (typeof teardown === "function") teardown();
  teardown = undefined;
  mounted = false;
  mountInFlight = false;
  if (root.value) root.value.innerHTML = "";
}

onMounted(ensureMounted);
watch(
  () => props.mountFn,
  () => {
    dispose();
    void ensureMounted();
  },
);
watch(
  () => props.active,
  () => {
    void ensureMounted();
  },
);
onBeforeUnmount(dispose);
</script>

<style scoped>
.plugin-host-pane {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent;
  color: var(--peek-text);
  font: inherit;
}

.plugin-host-pane.is-inactive {
  display: none;
}

.plugin-host-pane.is-fit-content {
  height: auto;
  min-height: 0;
  overflow: visible;
  flex: none;
  width: auto;
}

.plugin-host-pane :deep(.plugin-host-empty) {
  padding: 16px;
  color: var(--peek-faint);
  font-size: var(--peek-font-sm);
}

.plugin-host-pane :deep(select),
.plugin-host-pane :deep(textarea),
.plugin-host-pane
  :deep(
    input:not([type="checkbox"]):not([type="radio"]):not([type="range"]):not([type="file"]):not(
        [type="hidden"]
      )
  ) {
  font: inherit;
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 4%, var(--peek-surface));
  border: 1px solid var(--peek-border);
  border-radius: var(--peek-radius-sm, 6px);
}

.plugin-host-pane :deep(option) {
  background: #fff;
  color: #1a1a1a;
}

.plugin-host-pane :deep(input[type="range"]),
.plugin-host-pane :deep(input[type="checkbox"]),
.plugin-host-pane :deep(input[type="radio"]) {
  accent-color: var(--peek-accent);
}

/* i18n/a11y baseline: plugin-contributed content inherits reduced-motion by
   default unless the plugin explicitly opts its own elements out. */
@media (prefers-reduced-motion: reduce) {
  .plugin-host-pane :deep(*) {
    animation-duration: 0.001ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.001ms !important;
  }
}
</style>
