<template>
  <div class="catalog-panel">
    <SettingsSearchField
      v-model="query"
      :placeholder="copy.catalogSearch"
      :loading="loading"
      :submit-label="copy.search"
      @submit="$emit('search')"
    />
    <p v-if="runtimeHint" class="catalog-hint">{{ runtimeHint }}</p>
    <SettingsFormError :message="error" />

    <div v-if="showCurated" class="catalog-section">
      <h3>{{ copy.curatedTitle }}</h3>
      <div class="server-list">
        <CatalogItemCard
          v-for="entry in curatedEntries"
          :key="`curated-${entry.name}`"
          :title="entry.title"
          :meta="metaLine(entry)"
          :description="entry.description"
          :icon-fallback="entry.title"
          :pills="curatedPills(entry)"
          :expand-label="copy.expand"
          :collapse-label="copy.collapse"
        >
          <template #action>
            <CatalogRoundAction
              :done="isInstalled(entry.install.id)"
              :disabled="saving"
              :label="isInstalled(entry.install.id) ? copy.added : copy.install"
              @click="$emit('install', entry)"
            />
          </template>
          <template v-if="entry.requiredEnv.length" #footer>
            <p class="env-line">
              {{ copy.needsEnv(entry.requiredEnv.map((item) => item.name).join(", ")) }}
            </p>
          </template>
        </CatalogItemCard>
      </div>
    </div>

    <div class="catalog-section">
      <div class="section-head">
        <h3>{{ copy.registryTitle }}</h3>
        <span v-if="registryMeta" class="section-meta">{{ registryMeta }}</span>
      </div>
      <p v-if="!loading && registryEntries.length === 0" class="settings-empty">
        {{ copy.catalogEmpty }}
      </p>
      <div class="server-list">
        <CatalogItemCard
          v-for="entry in registryEntries"
          :key="entry.name"
          :title="entry.title"
          :meta="metaLine(entry)"
          :description="entry.description"
          :icon-fallback="entry.title"
          :pills="registryPills(entry)"
          :expand-label="copy.expand"
          :collapse-label="copy.collapse"
        >
          <template #action>
            <CatalogRoundAction
              :done="isInstalled(entry.install.id)"
              :disabled="saving"
              :label="isInstalled(entry.install.id) ? copy.added : copy.install"
              @click="$emit('install', entry)"
            />
          </template>
          <template v-if="entry.requiredEnv.length" #footer>
            <p class="env-line">
              {{ copy.needsEnv(entry.requiredEnv.map((item) => item.name).join(", ")) }}
            </p>
          </template>
        </CatalogItemCard>
      </div>
      <InfiniteScrollSentinel
        v-if="nextCursor || loading"
        :has-more="Boolean(nextCursor)"
        :loading="loading"
        @load="$emit('load-more')"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import SettingsSearchField from "@/components/settings/SettingsSearchField.vue";
import SettingsFormError from "@/components/settings/SettingsFormError.vue";
import InfiniteScrollSentinel from "@/components/settings/InfiniteScrollSentinel.vue";
import CatalogItemCard from "@/components/settings/CatalogItemCard.vue";
import CatalogRoundAction from "@/components/settings/CatalogRoundAction.vue";
import type { CatalogEntry } from "@/services/mcp/registry";
import type { McpServerConfig } from "@/types/setting";

const props = defineProps<{
  query: string;
  loading: boolean;
  error: string;
  runtimeHint: string;
  showCurated: boolean;
  curatedEntries: CatalogEntry[];
  registryEntries: CatalogEntry[];
  registryMeta: string;
  nextCursor: string | undefined;
  saving: boolean;
  isInstalled: (id: string) => boolean;
  copy: {
    catalogSearch: string;
    search: string;
    searching: string;
    curatedTitle: string;
    curatedBadge: string;
    registryTitle: string;
    catalogEmpty: string;
    install: string;
    added: string;
    needsEnv: (names: string) => string;
    expand: string;
    collapse: string;
  };
}>();

const emit = defineEmits<{
  search: [];
  install: [entry: CatalogEntry];
  "load-more": [];
  "update:query": [value: string];
}>();

const query = computed({
  get: () => props.query,
  set: (value: string) => emit("update:query", value),
});

function formatCommand(server: McpServerConfig) {
  return [server.command, ...(server.args ?? [])].filter(Boolean).join(" ");
}

function metaLine(entry: CatalogEntry) {
  const cmd = formatCommand(entry.install);
  const type = entry.package.registryType?.trim();
  return type ? `${type} · ${cmd}` : cmd;
}

function curatedPills(entry: CatalogEntry) {
  const pills = [props.copy.curatedBadge];
  if (props.isInstalled(entry.install.id)) pills.push(props.copy.added);
  return pills;
}

function registryPills(entry: CatalogEntry) {
  return props.isInstalled(entry.install.id) ? [props.copy.added] : [];
}
</script>

<style scoped>
.catalog-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
}

.catalog-hint {
  margin: 0;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.5;
}

.catalog-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.catalog-section h3 {
  margin: 0;
  font-size: 11px;
  font-weight: 650;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--muted-foreground);
}

.section-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
}

.section-meta {
  color: var(--muted-foreground);
  font-size: 11px;
}

.server-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.env-line {
  margin: 0;
  font-size: 11px;
  color: var(--muted-foreground);
}
</style>
