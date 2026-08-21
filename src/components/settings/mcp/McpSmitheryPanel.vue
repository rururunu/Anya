<template>
  <div class="catalog-panel">
    <SettingsSearchField
      v-model="query"
      :placeholder="labels.smitherySearch"
      :loading="loading"
      :submit-label="labels.search"
      @submit="$emit('search')"
    />
    <p v-if="error" class="form-error">{{ error }}</p>

    <div class="catalog-section">
      <p v-if="!loading && servers.length === 0 && loaded" class="empty">
        {{ labels.smitheryEmpty }}
      </p>
      <p v-else-if="loading && servers.length === 0" class="empty">{{ labels.searching }}</p>
      <div class="server-list">
        <CatalogItemCard
          v-for="server in servers"
          :key="server.id"
          :title="server.displayName || server.qualifiedName"
          :vendor="server.qualifiedName"
          :meta="metaLine(server)"
          :description="server.description"
          :icon-url="server.iconUrl"
          :icon-fallback="server.qualifiedName || server.displayName"
          :verified="Boolean(server.verified)"
          :verified-label="labels.verified"
          :pills="isInstalled(server) ? [labels.added] : []"
          :expand-label="labels.expand"
          :collapse-label="labels.collapse"
        >
          <template #action>
            <CatalogRoundAction
              :done="isInstalled(server)"
              :busy="installingId === server.id"
              :disabled="saving"
              :label="
                isInstalled(server)
                  ? labels.added
                  : installingId === server.id
                    ? labels.installing
                    : labels.install
              "
              @click="$emit('install', server)"
            />
          </template>
        </CatalogItemCard>
      </div>
      <InfiniteScrollSentinel
        v-if="loaded && (hasMore || loading)"
        :has-more="hasMore"
        :loading="loading"
        @load="$emit('load-more')"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import SettingsSearchField from "@/components/settings/SettingsSearchField.vue";
import InfiniteScrollSentinel from "@/components/settings/InfiniteScrollSentinel.vue";
import CatalogItemCard from "@/components/settings/CatalogItemCard.vue";
import CatalogRoundAction from "@/components/settings/CatalogRoundAction.vue";
import { formatSmitheryUses, type SmitheryMcpServerSummary } from "@/services/mcp/smithery";

export type McpSmitheryLabels = {
  smitherySearch: string;
  smitheryEmpty: string;
  search: string;
  searching: string;
  install: string;
  installing: string;
  added: string;
  verified: string;
  expand: string;
  collapse: string;
};

const props = withDefaults(
  defineProps<{
    loading?: boolean;
    loaded?: boolean;
    error?: string;
    servers?: SmitheryMcpServerSummary[];
    hasMore?: boolean;
    saving?: boolean;
    installingId?: string;
    isInstalled: (server: SmitheryMcpServerSummary) => boolean;
    labels?: McpSmitheryLabels;
  }>(),
  {
    loading: false,
    loaded: false,
    error: "",
    servers: () => [],
    hasMore: false,
    saving: false,
    installingId: "",
    labels: () => ({
      smitherySearch: "",
      smitheryEmpty: "",
      search: "",
      searching: "",
      install: "",
      installing: "",
      added: "",
      verified: "",
      expand: "",
      collapse: "",
    }),
  },
);

defineEmits<{
  search: [];
  "load-more": [];
  install: [server: SmitheryMcpServerSummary];
}>();

const query = defineModel<string>("query", { required: true });

/** Local alias so template never reads an unbound `copy` identifier. */
const labels = computed(() => props.labels);

function metaLine(server: SmitheryMcpServerSummary) {
  const parts: string[] = [];
  if (server.useCount != null) parts.push(formatSmitheryUses(server.useCount));
  return parts.join(" · ");
}
</script>

<style scoped>
.catalog-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
}

.empty {
  margin: 0;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.5;
}
.form-error {
  margin: 0;
  color: #ef4444;
  font-size: 12px;
  line-height: 1.5;
}
.catalog-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.server-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
</style>
