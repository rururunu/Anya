<template>
  <article class="catalog-item-card">
    <div class="card-head">
      <div
        class="card-icon"
        :class="{ 'has-image': Boolean(displayIconUrl) && !iconBroken }"
        aria-hidden="true"
      >
        <img
          v-if="displayIconUrl && !iconBroken"
          :src="displayIconUrl"
          alt=""
          referrerpolicy="no-referrer"
          @error="iconBroken = true"
        />
        <span v-else class="icon-fallback">{{ fallbackLetter }}</span>
      </div>
      <div class="card-identity">
        <div class="title-row">
          <strong>{{ title }}</strong>
          <span
            v-if="verified"
            class="verified-mark"
            :title="verifiedLabel"
            :aria-label="verifiedLabel"
          >
            <svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">
              <circle cx="8" cy="8" r="8" fill="#ea580c" />
              <path
                d="M5.2 8.1a2.8 2.8 0 0 1 5.6 0 2.8 2.8 0 0 1-5.6 0Zm1.15 0a1.65 1.65 0 1 0 3.3 0 1.65 1.65 0 0 0-3.3 0Z"
                fill="#fff"
              />
              <path
                d="M8 3.4v1.2M8 11.4v1.2M3.4 8h1.2M11.4 8h1.2M4.7 4.7l.85.85M10.45 10.45l.85.85M4.7 11.3l.85-.85M10.45 5.55l.85-.85"
                stroke="#fff"
                stroke-width="1.1"
                stroke-linecap="round"
              />
            </svg>
          </span>
          <span v-for="(pill, index) in pills" :key="`${pill}-${index}`" class="added-pill">
            {{ pill }}
          </span>
        </div>
        <p v-if="vendor" class="card-vendor" :title="vendor">{{ vendor }}</p>
        <p v-if="meta" class="card-meta">{{ meta }}</p>
        <!-- Never gate slots on $slots.* — that can infinite-loop with conditional parent slots. -->
        <div class="below-meta">
          <slot name="below-meta" />
        </div>
      </div>
      <div class="card-action">
        <slot name="action" />
      </div>
    </div>
    <CollapsibleText
      v-if="description"
      :text="description"
      :expand-label="expandLabel"
      :collapse-label="collapseLabel"
    />
    <div class="card-footer">
      <slot name="footer" />
    </div>
  </article>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import CollapsibleText from "@/components/settings/CollapsibleText.vue";
import { lookupInstallIcon, peekInstallIcon, type IconInstallKind } from "@/services/iconCache";

const props = withDefaults(
  defineProps<{
    title: string;
    /** Vendor / registry identity, e.g. adamamer20/paper-search-mcp-openai or gmail. */
    vendor?: string;
    /** Secondary line under the title (id, uses…). */
    meta?: string;
    description?: string;
    iconUrl?: string | null;
    /**
     * When set with iconCacheKey, resolve the install-scoped local icon first.
     * Browse/catalog cards omit these so they never write the disk cache.
     */
    iconCacheKind?: IconInstallKind | null;
    iconCacheKey?: string | null;
    /** Letter shown when icon is missing/broken. Defaults to first char of title. */
    iconFallback?: string;
    verified?: boolean;
    verifiedLabel?: string;
    /** Small pills next to the title (e.g. Added, Built-in, Connected). */
    pills?: string[];
    expandLabel?: string;
    collapseLabel?: string;
  }>(),
  {
    vendor: "",
    meta: "",
    description: "",
    iconUrl: null,
    iconCacheKind: null,
    iconCacheKey: null,
    iconFallback: "",
    verified: false,
    verifiedLabel: "Verified",
    pills: () => [],
    expandLabel: "More",
    collapseLabel: "Less",
  },
);

const iconBroken = ref(false);
/** Display src (local install cache, or remote browse URL). */
const displayIconUrl = ref("");

watch(
  () => [props.iconUrl, props.iconCacheKind, props.iconCacheKey] as const,
  ([url, kind, cacheKey], _prev, onCleanup) => {
    let cancelled = false;
    onCleanup(() => {
      cancelled = true;
    });
    iconBroken.value = false;

    const raw = url?.trim() || "";
    const key = cacheKey?.trim() || "";

    // Installed items: show remote immediately, upgrade to local disk cache when ready.
    if (kind && key) {
      const peeked = peekInstallIcon(kind, key);
      displayIconUrl.value = peeked || raw;
      void lookupInstallIcon(kind, key).then((local) => {
        if (cancelled) return;
        if (local) displayIconUrl.value = local;
        else if (!displayIconUrl.value) displayIconUrl.value = raw;
      });
      return;
    }

    // Browse / manual: use URL as-is (http remote or already-local). Never cache here.
    displayIconUrl.value = raw;
  },
  { immediate: true },
);

const fallbackLetter = computed(() => {
  const raw = (props.iconFallback || props.vendor || props.title || "?").trim();
  return raw ? raw.slice(0, 1).toUpperCase() : "?";
});
</script>

<style scoped>
.catalog-item-card {
  padding: 14px 16px;
  border: 1px solid color-mix(in srgb, var(--border) 88%, transparent);
  border-radius: 14px;
  background: color-mix(in srgb, var(--muted-foreground) 4.5%, var(--background));
}

.card-head {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.card-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  overflow: hidden;
  flex-shrink: 0;
  background: transparent;
  display: grid;
  place-items: center;
}

.card-icon:not(.has-image) {
  background: color-mix(in srgb, var(--muted-foreground) 10%, transparent);
}

.card-icon.has-image {
  background: transparent;
  border: none;
}

.card-icon img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
  background: transparent;
}

.icon-fallback {
  font-size: 15px;
  font-weight: 650;
  color: var(--muted-foreground);
}

.card-identity {
  min-width: 0;
  flex: 1;
  padding-top: 1px;
}

.title-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}

.title-row strong {
  font-size: 14px;
  font-weight: 650;
  letter-spacing: -0.01em;
  line-height: 1.2;
}

.verified-mark {
  display: inline-flex;
  line-height: 0;
}

.added-pill {
  font-size: 10px;
  padding: 1px 7px;
  border-radius: 999px;
  border: 1px solid var(--border);
  color: var(--muted-foreground);
}

.card-vendor {
  margin: 3px 0 0;
  font-family: var(--font-mono, ui-monospace, Consolas, monospace);
  font-size: 12px;
  font-weight: 550;
  color: color-mix(in srgb, var(--foreground) 78%, var(--muted-foreground));
  line-height: 1.35;
  word-break: break-all;
}

.card-meta {
  margin: 3px 0 0;
  font-size: 12px;
  color: var(--muted-foreground);
  line-height: 1.35;
}

.below-meta:empty {
  display: none;
}

.below-meta > :deep(*) {
  margin-top: 6px;
}

.card-action {
  display: flex;
  align-items: flex-start;
  gap: 4px;
  flex-shrink: 0;
  flex-wrap: wrap;
  justify-content: flex-end;
  max-width: min(100%, 260px);
}

.card-action:empty {
  display: none;
}

.card-footer:empty {
  display: none;
}

.card-footer:not(:empty) {
  margin-top: 8px;
}
</style>
