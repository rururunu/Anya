<template>
  <section class="settings-page is-wide mcp-settings">
    <AppConfirmDialog ref="confirmDialogRef" />

    <DialogRoot :open="smitheryConfigOpen" @update:open="onSmitheryConfigOpenChange">
      <DialogPortal>
        <DialogOverlay class="smithery-config-overlay" />
        <DialogContent class="smithery-config-dialog" :aria-describedby="undefined">
          <DialogTitle class="smithery-config-title">
            {{ smitheryConfigRequired ? copy.smitheryApiKeyMissingTitle : copy.smitheryConfig }}
          </DialogTitle>
          <DialogDescription class="smithery-config-desc">
            {{ smitheryConfigRequired ? copy.smitheryApiKeyMissingDesc : copy.smitheryApiKeyHint }}
          </DialogDescription>
          <label class="smithery-config-label" for="smithery-api-key">
            {{ copy.smitheryApiKey }}
          </label>
          <input
            id="smithery-api-key"
            v-model="smitheryApiKeyDraft"
            class="smithery-key-input"
            type="password"
            autocomplete="off"
            spellcheck="false"
            :placeholder="copy.smitheryApiKeyPlaceholder"
            @keydown.enter.prevent="confirmSmitheryConfig"
          />
          <button type="button" class="smithery-config-link" @click="openSmitheryApiKeysPage">
            {{ copy.smitheryGetApiKey }}
            <ExternalLink class="size-3" />
          </button>
          <div class="smithery-config-actions">
            <Button variant="outline" size="sm" class="h-8" @click="closeSmitheryConfig">
              {{ copy.cancel }}
            </Button>
            <Button size="sm" class="h-8" :disabled="saving" @click="confirmSmitheryConfig">
              {{ copy.save }}
            </Button>
          </div>
        </DialogContent>
      </DialogPortal>
    </DialogRoot>

    <SettingsPageHeader :title="copy.title" :hide-title="embedded">
      <template #actions>
        <Button
          variant="outline"
          size="sm"
          class="h-8 gap-1.5"
          :title="copy.smitheryConfig"
          :aria-label="copy.smitheryConfig"
          @click="openSmitheryConfig()"
        >
          <Settings class="size-3.5" />
          {{ copy.smitheryConfig }}
        </Button>
        <Button
          v-if="tab === 'installed'"
          size="sm"
          class="h-8 gap-1.5"
          :disabled="Boolean(editor)"
          @click="startCreate"
        >
          <Plus class="size-3.5" />
          {{ copy.add }}
        </Button>
      </template>
    </SettingsPageHeader>

    <div class="settings-tabs" role="tablist">
      <button
        type="button"
        role="tab"
        class="settings-tab"
        :class="{ on: tab === 'installed' }"
        :aria-selected="tab === 'installed'"
        @click="tab = 'installed'"
      >
        {{ copy.tabInstalled }}
      </button>
      <button
        type="button"
        role="tab"
        class="settings-tab"
        :class="{ on: tab === 'catalog' }"
        :aria-selected="tab === 'catalog'"
        @click="openCatalog"
      >
        {{ copy.tabBuiltin }}
      </button>
      <button
        type="button"
        role="tab"
        class="settings-tab"
        :class="{ on: tab === 'smithery' }"
        :aria-selected="tab === 'smithery'"
        @click="openSmithery"
      >
        {{ copy.tabSmithery }}
      </button>
    </div>

    <SettingsFormError :message="error" />

    <McpServerEditor
      v-if="editor"
      :editor="editor"
      :saving="saving"
      :copy="copy"
      :meta-labels="metaLabels"
      @update:editor="(value) => (editor = value)"
      @cancel="cancelEdit"
      @save="saveEditor"
    />

    <McpInstalledList
      v-if="tab === 'installed'"
      :servers="filtered"
      :statuses="statusMap"
      :busy-id="authBusyId"
      :disabled-actions="Boolean(editor)"
      :copy="copy"
      @toggle="toggleEnabled"
      @edit="startEdit"
      @remove="remove"
      @connect="connectServer"
      @reauthenticate="reauthenticateServer"
    />

    <McpCatalogPanel
      v-else-if="tab === 'catalog'"
      v-model:query="catalogQuery"
      :loading="catalogLoading"
      :error="catalogError"
      :runtime-hint="runtimeHint"
      :show-curated="showCurated"
      :curated-entries="curatedEntries"
      :registry-entries="visibleRegistryEntries"
      :registry-meta="registryMeta"
      :next-cursor="catalogNextCursor"
      :saving="saving"
      :is-installed="isInstalled"
      :copy="copy"
      @search="runCatalogSearch"
      @install="addFromCatalog"
      @load-more="loadMoreCatalog"
    />

    <McpSmitheryPanel
      v-else-if="tab === 'smithery'"
      v-model:query="smitheryQuery"
      :loading="smitheryLoading"
      :loaded="smitheryLoaded"
      :error="smitheryError"
      :servers="smitheryServers"
      :has-more="smitheryHasMore"
      :saving="saving"
      :installing-id="smitheryInstallingId"
      :is-installed="isSmitheryInstalled"
      :labels="smitheryLabels"
      @search="runSmitherySearch"
      @load-more="loadMoreSmithery"
      @install="installFromSmithery"
    />
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { ExternalLink, Plus, Settings } from "@lucide/vue";
import { Button } from "@/components/ui/button";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import SettingsPageHeader from "@/components/settings/SettingsPageHeader.vue";
import SettingsFormError from "@/components/settings/SettingsFormError.vue";
import {
  DialogContent,
  DialogDescription,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
} from "reka-ui";
import { invoke } from "@tauri-apps/api/core";
import {
  type CatalogEntry,
  type McpRuntimeSupport,
  filterCurated,
  filterInstallable,
  searchMcpRegistry,
} from "@/services/mcp/registry";
import {
  isMcpRemoteServer,
  isSameMcpInstall,
  mcpRemoteServerUrl,
  withoutApiKeyParam,
  withPinnedMcpRemote,
  type McpConnectResult,
  type McpServerRuntimeStatus,
} from "@/services/mcp/remote";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  buildSmitheryMcpInstall,
  deleteSmitheryConnection,
  getSmitheryMcpServer,
  isSmitheryConnectProxyUrl,
  isSmitheryHostedServer,
  mcpInstallId,
  resolveSmitheryDeploymentUrl,
  searchSmitheryMcpServers,
  sortSmitheryMcpByDownloads,
  upsertSmitheryConnection,
  withSmitheryConnectProxyArgs,
  type SmitheryMcpServerSummary,
} from "@/services/mcp/smithery";
import { cacheInstallIcon, clearInstallIcon, warmInstallIcons } from "@/services/iconCache";
import { sortByResourceUsage } from "@/services/usage/resourceUsage";
import { tr } from "@/services/i18n";
import type { McpI18nKey } from "@/services/locales/mcp";
import { useSettingStore } from "@/stores/setting";
import type { McpServerConfig } from "@/types/setting";
import McpCatalogPanel from "./mcp/McpCatalogPanel.vue";
import McpInstalledList from "./mcp/McpInstalledList.vue";
import McpServerEditor from "./mcp/McpServerEditor.vue";
import McpSmitheryPanel from "./mcp/McpSmitheryPanel.vue";

const props = defineProps<{ query?: string; embedded?: boolean }>();
const settingStore = useSettingStore();
const saving = ref(false);
const error = ref("");
const tab = ref<"installed" | "catalog" | "smithery">("installed");
const catalogQuery = ref("");
const catalogLoading = ref(false);
const catalogError = ref("");
const registryEntries = ref<CatalogEntry[]>([]);
const catalogNextCursor = ref<string | undefined>();
const catalogLoaded = ref(false);
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const runtimeSupport = ref<McpRuntimeSupport>({ npm: true, pypi: true });

const smitheryQuery = ref("");
const smitheryServers = ref<SmitheryMcpServerSummary[]>([]);
const smitheryLoading = ref(false);
const smitheryError = ref("");
const smitheryLoaded = ref(false);
const smitheryPage = ref(1);
const smitheryTotalPages = ref(1);
const smitheryInstallingId = ref("");
const smitheryApiKeyDraft = ref("");
const smitheryConfigOpen = ref(false);
const smitheryConfigRequired = ref(false);
const smitheryConfigContinue = ref<"none" | "smithery-tab">("none");
const statusMap = ref<Record<string, McpServerRuntimeStatus>>({});
const authBusyId = ref("");
/** Full Smithery install kept while the user fills required env vars. */
const pendingSmitheryInstall = ref<McpServerConfig | null>(null);

const SMITHERY_API_KEYS_URL = "https://smithery.ai/account/api-keys";
let statusTimer: ReturnType<typeof setInterval> | null = null;

type EditorState = {
  mode: "create" | "edit";
  id: string;
  title: string;
  description: string;
  command: string;
  argsText: string;
  envText: string;
  enabled: boolean;
};

const editor = ref<EditorState | null>(null);

const lang = computed(() => settingStore.language);
const t = (key: McpI18nKey, values?: Record<string, string | number>) =>
  tr(lang.value, key, values ?? {});

const metaLabels = computed(() => ({
  displayName: t("mcp.displayName"),
  displayNamePlaceholder: t("mcp.displayNamePlaceholder"),
  blurb: t("mcp.blurb"),
  blurbPlaceholder: t("mcp.blurbPlaceholder"),
}));

const copy = computed(() => ({
  title: t("mcp.title"),
  add: t("mcp.add"),
  empty: t("mcp.empty"),
  id: t("mcp.id"),
  idPlaceholder: t("mcp.idPlaceholder"),
  command: t("mcp.command"),
  commandPlaceholder: t("mcp.commandPlaceholder"),
  args: t("mcp.args"),
  argsPlaceholder: t("mcp.argsPlaceholder"),
  env: t("mcp.env"),
  envPlaceholder: t("mcp.envPlaceholder"),
  envCount: (count: number) => t("mcp.envCount", { count }),
  enabled: t("mcp.enabled"),
  disabled: t("mcp.disabled"),
  edit: t("mcp.edit"),
  remove: t("mcp.remove"),
  cancel: t("mcp.cancel"),
  save: t("mcp.save"),
  deleteTitle: t("mcp.deleteTitle"),
  deleteDesc: (name: string) => t("mcp.deleteDesc", { name }),
  deleteConfirm: t("mcp.deleteConfirm"),
  idRequired: t("mcp.idRequired"),
  idExists: t("mcp.idExists"),
  commandRequired: t("mcp.commandRequired"),
  tabInstalled: t("mcp.tabInstalled"),
  tabBuiltin: t("mcp.tabBuiltin"),
  tabSmithery: t("mcp.tabSmithery"),
  catalogSearch: t("mcp.catalogSearch"),
  search: t("mcp.search"),
  searching: t("mcp.searching"),
  curatedTitle: t("mcp.curatedTitle"),
  curatedBadge: t("mcp.curatedBadge"),
  registryTitle: t("mcp.registryTitle"),
  catalogEmpty: t("mcp.catalogEmpty"),
  install: t("mcp.install"),
  installing: t("mcp.installing"),
  added: t("mcp.added"),
  needsEnv: (names: string) => t("mcp.needsEnv", { names }),
  loadMore: t("mcp.loadMore"),
  resultCount: (count: number) => t("mcp.resultCount", { count }),
  smitherySearch: t("mcp.smitherySearch"),
  smitheryEmpty: t("mcp.smitheryEmpty"),
  smitheryNoRemote: t("mcp.smitheryNoRemote"),
  verified: t("mcp.verified"),
  expand: t("mcp.expand"),
  collapse: t("mcp.collapse"),
  authConnected: t("mcp.authConnected"),
  authSaved: t("mcp.authSaved"),
  authNeeded: t("mcp.authNeeded"),
  authLocal: t("mcp.authLocal"),
  authDisabled: t("mcp.authDisabled"),
  reauthenticate: t("mcp.reauthenticate"),
  connectNow: t("mcp.connectNow"),
  connecting: t("mcp.connecting"),
  smitheryConfig: t("mcp.smitheryConfig"),
  smitheryApiKey: t("mcp.smitheryApiKey"),
  smitheryApiKeyPlaceholder: t("mcp.smitheryApiKeyPlaceholder"),
  smitheryApiKeyHint: t("mcp.smitheryApiKeyHint"),
  smitheryGetApiKey: t("mcp.smitheryGetApiKey"),
  smitheryApiKeyRequired: t("mcp.smitheryApiKeyRequired"),
  smitheryApiKeyMissingTitle: t("mcp.smitheryApiKeyMissingTitle"),
  smitheryApiKeyMissingDesc: t("mcp.smitheryApiKeyMissingDesc"),
  smitheryAuthPending: t("mcp.smitheryAuthPending"),
  smitheryOpenConnections: t("mcp.smitheryOpenConnections"),
}));

const smitheryLabels = computed(() => ({
  smitherySearch: copy.value.smitherySearch,
  smitheryEmpty: copy.value.smitheryEmpty,
  search: copy.value.search,
  searching: copy.value.searching,
  install: copy.value.install,
  installing: copy.value.installing,
  added: copy.value.added,
  verified: copy.value.verified,
  expand: copy.value.expand,
  collapse: copy.value.collapse,
}));

const smitheryHasMore = computed(
  () => smitheryLoaded.value && smitheryPage.value < smitheryTotalPages.value,
);

const servers = computed(() => settingStore.mcpServers ?? []);
const curatedEntries = computed(() =>
  filterInstallable(filterCurated(catalogQuery.value || props.query || ""), runtimeSupport.value),
);
const visibleRegistryEntries = computed(() =>
  filterInstallable(registryEntries.value, runtimeSupport.value),
);
const showCurated = computed(() => curatedEntries.value.length > 0);
const registryMeta = computed(() => {
  if (!catalogLoaded.value || catalogLoading.value) return "";
  if (!visibleRegistryEntries.value.length) return "";
  return copy.value.resultCount(visibleRegistryEntries.value.length);
});
const runtimeHint = computed(() => {
  const support = runtimeSupport.value;
  if (support.npm && support.pypi) return "";
  const zh = settingStore.language.startsWith("zh");
  if (!support.npm && !support.pypi) {
    return zh
      ? "未检测到 Node/npx 或 uvx，目录暂无可安装项。请先安装 Node.js，或在「已安装」中手动添加。"
      : "No Node/npx or uvx detected — catalog is empty. Install Node.js, or add a server manually.";
  }
  if (!support.npm) {
    return zh
      ? "未检测到 Node/npx，已隐藏 npm 类 MCP。"
      : "Node/npx not found — npm MCP packages are hidden.";
  }
  return zh
    ? "未检测到 uvx，已隐藏 PyPI 类 MCP。"
    : "uvx not found — PyPI MCP packages are hidden.";
});

async function refreshRuntimeSupport() {
  try {
    runtimeSupport.value = await invoke<McpRuntimeSupport>("get_mcp_runtime_support");
  } catch {
    // Fail open so catalog still works if IPC unavailable during HMR.
    runtimeSupport.value = { npm: true, pypi: true };
  }
}

async function refreshServerStatuses() {
  try {
    const list = await invoke<McpServerRuntimeStatus[]>("list_mcp_server_statuses");
    const next: Record<string, McpServerRuntimeStatus> = {};
    for (const item of list) next[item.id] = item;
    statusMap.value = next;
  } catch {
    // Status is best-effort; keep last known map on IPC hiccups.
  }
}

function startStatusPolling() {
  stopStatusPolling();
  void refreshServerStatuses();
  statusTimer = setInterval(() => {
    void refreshServerStatuses();
  }, 2000);
}

function stopStatusPolling() {
  if (statusTimer) {
    clearInterval(statusTimer);
    statusTimer = null;
  }
}

/** After install/save: wait for background register, then ensure this server connects (OAuth if needed). */
async function ensureConnectedAfterInstall(server: McpServerConfig) {
  if (server.enabled === false) return;
  tab.value = "installed";
  error.value = "";
  // Smithery hosted servers need an API key + website account link first.
  if (isSmitheryHostedServer(server)) {
    if (!hasSmitheryApiKey()) {
      openSmitheryConfig({ required: true });
      return;
    }
    authBusyId.value = server.id;
    try {
      if (!(await ensureSmitheryConnectionAuthorized(server))) return;
    } finally {
      authBusyId.value = "";
    }
  }
  authBusyId.value = server.id;
  try {
    await new Promise((resolve) => setTimeout(resolve, 400));
    const result = await invoke<McpConnectResult>("connect_mcp_server", {
      serverId: server.id,
    });
    statusMap.value = { ...statusMap.value, [server.id]: result.status };
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
    await refreshServerStatuses();
  } finally {
    authBusyId.value = "";
  }
}

async function openSmitheryPage(server: McpServerConfig) {
  const url =
    server.homepage?.trim() ||
    (server.qualifiedName
      ? `https://smithery.ai/servers/${server.qualifiedName}`
      : SMITHERY_API_KEYS_URL);
  try {
    await openUrl(url);
  } catch (err) {
    console.error("open smithery page failed:", err);
  }
}

/**
 * Ask Smithery's Connections API for this server's real status before spawning mcp-remote.
 * When connected, rewrite mcp-remote to the Smithery Connect proxy so tool calls use the
 * vaulted Google/OAuth credentials (instead of hitting Arcade upstream and landing on
 * example.com/?flow_id=...).
 * https://smithery.ai/docs/use/connect
 */
async function resolveSmitheryUpstreamUrl(server: McpServerConfig): Promise<string | null> {
  const current = mcpRemoteServerUrl(server);
  if (current && !isSmitheryConnectProxyUrl(current)) {
    return withoutApiKeyParam(current);
  }
  const qn = server.qualifiedName?.trim();
  if (!qn) return null;
  try {
    const detail = await getSmitheryMcpServer(qn);
    return resolveSmitheryDeploymentUrl(detail);
  } catch (err) {
    console.error("resolve smithery upstream failed:", err);
    return null;
  }
}

async function routeServerThroughSmitheryConnect(
  server: McpServerConfig,
  namespace: string,
  connectionId: string,
): Promise<McpServerConfig> {
  const proxy = `https://api.smithery.ai/connect/${encodeURIComponent(namespace)}/${encodeURIComponent(connectionId)}/mcp`;
  const current = mcpRemoteServerUrl(server);
  if (current && withoutApiKeyParam(current) === proxy) return server;
  const nextArgs = withPinnedMcpRemote(
    withSmitheryConnectProxyArgs(server.args ?? [], namespace, connectionId),
  );
  const updated: McpServerConfig = { ...server, args: nextArgs };
  const nextServers = servers.value.map((item) => (item.id === server.id ? updated : item));
  await persist(nextServers);
  return updated;
}

async function ensureSmitheryConnectionAuthorized(
  server: McpServerConfig,
  options?: { forceReauth?: boolean },
): Promise<boolean> {
  const apiKey = settingStore.smitheryApiKey.trim();
  if (!apiKey) return true;
  const upstream = await resolveSmitheryUpstreamUrl(server);
  if (!upstream) return true;
  try {
    if (options?.forceReauth) {
      // Drop the vaulted connection so the next upsert returns auth_required + setupUrl.
      await deleteSmitheryConnection(server, apiKey);
    }
    const status = await upsertSmitheryConnection(server, upstream, apiKey);
    if (status.state === "connected" && !options?.forceReauth) {
      await routeServerThroughSmitheryConnect(server, status.namespace, status.connectionId);
      return true;
    }
    if (status.setupUrl) {
      await openUrl(status.setupUrl);
    } else {
      await openSmitheryPage(server);
    }
    if (status.state === "connected" && options?.forceReauth) {
      // Already connected again somehow — still open setup/homepage above for re-link.
      await routeServerThroughSmitheryConnect(server, status.namespace, status.connectionId);
    }
    error.value = status.message || copy.value.smitheryAuthPending;
    return false;
  } catch (err) {
    console.error("smithery connection check failed:", err);
    if (options?.forceReauth) {
      error.value = err instanceof Error ? err.message : String(err);
      return false;
    }
    return true;
  }
}

async function openSmitheryApiKeysPage() {
  try {
    await openUrl(SMITHERY_API_KEYS_URL);
  } catch (err) {
    console.error("open smithery api keys page failed:", err);
  }
}

function hasSmitheryApiKey() {
  return Boolean(settingStore.smitheryApiKey.trim());
}

function openSmitheryConfig(options?: {
  required?: boolean;
  continueTo?: "none" | "smithery-tab";
}) {
  smitheryApiKeyDraft.value = settingStore.smitheryApiKey;
  smitheryConfigRequired.value = options?.required ?? false;
  smitheryConfigContinue.value = options?.continueTo ?? "none";
  smitheryConfigOpen.value = true;
}

function closeSmitheryConfig() {
  smitheryConfigOpen.value = false;
  smitheryConfigRequired.value = false;
  smitheryConfigContinue.value = "none";
  smitheryApiKeyDraft.value = settingStore.smitheryApiKey;
}

function onSmitheryConfigOpenChange(nextOpen: boolean) {
  if (!nextOpen) {
    closeSmitheryConfig();
    return;
  }
  smitheryConfigOpen.value = true;
}

async function confirmSmitheryConfig() {
  error.value = "";
  await saveSmitheryApiKey();
  if (smitheryConfigRequired.value && !hasSmitheryApiKey()) {
    error.value = copy.value.smitheryApiKeyRequired;
    return;
  }
  const continueTo = smitheryConfigContinue.value;
  closeSmitheryConfig();
  if (continueTo === "smithery-tab" && hasSmitheryApiKey()) {
    await enterSmitheryTab();
  }
}

async function connectServer(server: McpServerConfig) {
  error.value = "";
  if (isSmitheryHostedServer(server)) {
    if (!hasSmitheryApiKey()) {
      openSmitheryConfig({ required: true });
      return;
    }
    authBusyId.value = server.id;
    try {
      if (!(await ensureSmitheryConnectionAuthorized(server))) return;
    } finally {
      authBusyId.value = "";
    }
  }
  authBusyId.value = server.id;
  try {
    const result = await invoke<McpConnectResult>("connect_mcp_server", {
      serverId: server.id,
    });
    statusMap.value = { ...statusMap.value, [server.id]: result.status };
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
    await refreshServerStatuses();
  } finally {
    authBusyId.value = "";
  }
}

async function reauthenticateServer(server: McpServerConfig) {
  error.value = "";
  if (isSmitheryHostedServer(server)) {
    if (!hasSmitheryApiKey()) {
      openSmitheryConfig({ required: true });
      return;
    }
    authBusyId.value = server.id;
    try {
      // Force Smithery hosted setupUrl even when Connection status is already connected.
      await ensureSmitheryConnectionAuthorized(server, { forceReauth: true });
      await refreshServerStatuses();
    } finally {
      authBusyId.value = "";
    }
    return;
  }
  authBusyId.value = server.id;
  try {
    const result = await invoke<McpConnectResult>("reauthenticate_mcp_server", {
      serverId: server.id,
    });
    statusMap.value = { ...statusMap.value, [server.id]: result.status };
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
    await refreshServerStatuses();
  } finally {
    authBusyId.value = "";
  }
}

async function saveSmitheryApiKey() {
  const next = smitheryApiKeyDraft.value.trim();
  if (next === settingStore.smitheryApiKey) return;
  try {
    await settingStore.update({ smitheryApiKey: next });
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  }
}

const filtered = computed(() => {
  const query = props.query?.trim().toLowerCase() ?? "";
  const list = !query
    ? servers.value
    : servers.value.filter((server) => {
        const haystack = [
          server.id,
          server.title ?? "",
          server.description ?? "",
          server.command,
          ...(server.args ?? []),
          ...(server.env ?? []).flatMap(([k, v]) => [k, v]),
        ]
          .join(" ")
          .toLowerCase();
        return haystack.includes(query);
      });
  return sortByResourceUsage(list, "mcp", (server) => server.id);
});

function parseArgs(text: string) {
  return text
    .trim()
    .split(/\s+/)
    .map((part) => part.trim())
    .filter(Boolean);
}

function parseEnv(text: string): Array<[string, string]> {
  const result: Array<[string, string]> = [];
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eq = trimmed.indexOf("=");
    if (eq <= 0) continue;
    result.push([trimmed.slice(0, eq).trim(), trimmed.slice(eq + 1)]);
  }
  return result;
}

function formatEnv(env?: Array<[string, string]>) {
  return (env ?? []).map(([k, v]) => `${k}=${v}`).join("\n");
}

function isInstalled(id: string) {
  return servers.value.some((server) => server.id === id);
}

function isSmitheryInstalled(server: SmitheryMcpServerSummary) {
  return servers.value.some((installed) =>
    isSameMcpInstall(installed, {
      id: server.id,
      qualifiedName: server.qualifiedName,
      installId: mcpInstallId(server),
    }),
  );
}

async function cacheIconForInstall(server: McpServerConfig) {
  const url = server.iconUrl?.trim();
  if (!url) return;
  await cacheInstallIcon("mcp", server.id, url);
}

function serverTitle(server: McpServerConfig) {
  return server.title?.trim() || server.id;
}

function startCreate() {
  error.value = "";
  editor.value = {
    mode: "create",
    id: "",
    title: "",
    description: "",
    command: "",
    argsText: "",
    envText: "",
    enabled: true,
  };
}

function startEdit(server: McpServerConfig) {
  error.value = "";
  editor.value = {
    mode: "edit",
    id: server.id,
    title: server.title ?? "",
    description: server.description ?? "",
    command: server.command,
    argsText: (server.args ?? []).join(" "),
    envText: formatEnv(server.env),
    enabled: server.enabled !== false,
  };
}

function cancelEdit() {
  editor.value = null;
  pendingSmitheryInstall.value = null;
  error.value = "";
}

async function persist(next: McpServerConfig[]) {
  saving.value = true;
  error.value = "";
  try {
    await settingStore.update({ mcpServers: next });
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
    throw err;
  } finally {
    saving.value = false;
  }
}

async function saveEditor() {
  const draft = editor.value;
  if (!draft) return;
  const id = draft.id.trim();
  const command = draft.command.trim();
  if (!id) {
    error.value = copy.value.idRequired;
    return;
  }
  if (!command) {
    error.value = copy.value.commandRequired;
    return;
  }
  if (draft.mode === "create" && servers.value.some((server) => server.id === id)) {
    error.value = copy.value.idExists;
    return;
  }

  const title = draft.title.trim();
  const description = draft.description.trim();
  const pending =
    draft.mode === "create" && pendingSmitheryInstall.value?.id === id
      ? pendingSmitheryInstall.value
      : null;
  const nextServer: McpServerConfig = {
    id,
    ...(title ? { title } : {}),
    ...(description ? { description } : {}),
    command,
    args: withPinnedMcpRemote(parseArgs(draft.argsText)),
    env: parseEnv(draft.envText),
    enabled: draft.enabled,
    ...(pending?.iconUrl ? { iconUrl: pending.iconUrl } : {}),
    ...(pending?.qualifiedName ? { qualifiedName: pending.qualifiedName } : {}),
    ...(pending?.registryId ? { registryId: pending.registryId } : {}),
    ...(pending?.homepage ? { homepage: pending.homepage } : {}),
    ...(pending?.source ? { source: pending.source } : {}),
  };
  const next =
    draft.mode === "create"
      ? [...servers.value, nextServer]
      : servers.value.map((server) => (server.id === id ? nextServer : server));
  await persist(next);
  editor.value = null;
  pendingSmitheryInstall.value = null;
  if (draft.mode === "create") {
    void cacheIconForInstall(nextServer);
    if (isMcpRemoteServer(nextServer)) {
      void ensureConnectedAfterInstall(nextServer);
    } else {
      void refreshServerStatuses();
    }
  } else {
    void refreshServerStatuses();
  }
}

async function toggleEnabled(server: McpServerConfig) {
  const next = servers.value.map((item) =>
    item.id === server.id ? { ...item, enabled: item.enabled === false } : item,
  );
  await persist(next);
  void refreshServerStatuses();
}

async function remove(server: McpServerConfig) {
  const confirmed = await confirmDialogRef.value?.ask({
    title: copy.value.deleteTitle,
    description: copy.value.deleteDesc(serverTitle(server)),
    confirmLabel: copy.value.deleteConfirm,
    cancelLabel: copy.value.cancel,
  });
  if (!confirmed) return;
  await persist(servers.value.filter((item) => item.id !== server.id));
  void clearInstallIcon("mcp", server.id);
  if (editor.value?.id === server.id) editor.value = null;
  void refreshServerStatuses();
}

function toPlainInstall(entry: CatalogEntry): McpServerConfig {
  const title = (entry.install.title ?? entry.title ?? "").trim();
  const description = (entry.install.description ?? entry.description ?? "").trim();
  return {
    id: String(entry.install.id ?? "").trim(),
    ...(title ? { title } : {}),
    ...(description ? { description } : {}),
    command: String(entry.install.command ?? "").trim(),
    args: withPinnedMcpRemote([...(entry.install.args ?? [])].map(String)),
    env: (entry.install.env ?? []).map(([k, v]) => [String(k), String(v)] as [string, string]),
    enabled: entry.source === "curated" ? false : entry.install.enabled !== false,
  };
}

async function addFromCatalog(entry: CatalogEntry) {
  if (isInstalled(entry.install.id)) return;
  const install = toPlainInstall(entry);
  if (!install.id || !install.command) {
    error.value = copy.value.commandRequired;
    return;
  }
  const requiredEnv = entry.requiredEnv ?? [];
  if (requiredEnv.length) {
    const envLines = [
      ...(install.env ?? []).map(([k, v]) => `${k}=${v}`),
      ...requiredEnv.map((item) => `${item.name}=`),
    ].filter(Boolean);
    editor.value = {
      mode: "create",
      id: install.id,
      title: install.title ?? "",
      description: install.description ?? "",
      command: install.command,
      argsText: (install.args ?? []).join(" "),
      envText: envLines.join("\n"),
      enabled: install.enabled !== false,
    };
    error.value = copy.value.needsEnv(requiredEnv.map((item) => item.name).join(", "));
    return;
  }
  try {
    await persist([...servers.value, install]);
    if (isMcpRemoteServer(install)) {
      void ensureConnectedAfterInstall(install);
    } else {
      void refreshServerStatuses();
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  }
}

async function runCatalogSearch() {
  catalogLoading.value = true;
  catalogError.value = "";
  catalogNextCursor.value = undefined;
  try {
    await refreshRuntimeSupport();
    const result = await searchMcpRegistry(catalogQuery.value || props.query || "", {
      desired: 60,
      maxPages: 12,
    });
    registryEntries.value = filterInstallable(result.entries, runtimeSupport.value);
    catalogNextCursor.value = result.nextCursor;
    catalogLoaded.value = true;
  } catch (err) {
    catalogError.value = err instanceof Error ? err.message : String(err);
    registryEntries.value = [];
    catalogNextCursor.value = undefined;
  } finally {
    catalogLoading.value = false;
  }
}

async function loadMoreCatalog() {
  if (!catalogNextCursor.value || catalogLoading.value) return;
  catalogLoading.value = true;
  catalogError.value = "";
  try {
    const result = await searchMcpRegistry(catalogQuery.value || props.query || "", {
      desired: 40,
      maxPages: 8,
      cursor: catalogNextCursor.value,
    });
    const seen = new Set(registryEntries.value.map((entry) => entry.name));
    for (const entry of filterInstallable(result.entries, runtimeSupport.value)) {
      if (seen.has(entry.name)) continue;
      seen.add(entry.name);
      registryEntries.value.push(entry);
    }
    catalogNextCursor.value = result.nextCursor;
  } catch (err) {
    catalogError.value = err instanceof Error ? err.message : String(err);
  } finally {
    catalogLoading.value = false;
  }
}

function openCatalog() {
  tab.value = "catalog";
  if (!catalogLoaded.value) void runCatalogSearch();
}

async function openSmithery() {
  if (!hasSmitheryApiKey()) {
    openSmitheryConfig({ required: true, continueTo: "smithery-tab" });
    return;
  }
  await enterSmitheryTab();
}

async function enterSmitheryTab() {
  tab.value = "smithery";
  if (!smitheryLoaded.value && !smitheryLoading.value) {
    await runSmitherySearch();
  }
}

async function runSmitherySearch() {
  smitheryLoading.value = true;
  smitheryError.value = "";
  smitheryPage.value = 1;
  try {
    const result = await searchSmitheryMcpServers(smitheryQuery.value, {
      page: 1,
      pageSize: 20,
    });
    smitheryServers.value = result.servers;
    smitheryTotalPages.value = Math.max(1, result.pagination.totalPages || 1);
    smitheryLoaded.value = true;
  } catch (err) {
    smitheryError.value = err instanceof Error ? err.message : String(err);
    smitheryServers.value = [];
  } finally {
    smitheryLoading.value = false;
  }
}

async function loadMoreSmithery() {
  if (!smitheryHasMore.value || smitheryLoading.value) return;
  smitheryLoading.value = true;
  smitheryError.value = "";
  try {
    const nextPage = smitheryPage.value + 1;
    const result = await searchSmitheryMcpServers(smitheryQuery.value, {
      page: nextPage,
      pageSize: 20,
    });
    const seen = new Set(smitheryServers.value.map((s) => s.id));
    const merged = [...smitheryServers.value];
    for (const server of result.servers) {
      if (seen.has(server.id)) continue;
      seen.add(server.id);
      merged.push(server);
    }
    smitheryServers.value = sortSmitheryMcpByDownloads(merged);
    smitheryPage.value = nextPage;
    smitheryTotalPages.value = Math.max(1, result.pagination.totalPages || 1);
  } catch (err) {
    smitheryError.value = err instanceof Error ? err.message : String(err);
  } finally {
    smitheryLoading.value = false;
  }
}

async function installFromSmithery(server: SmitheryMcpServerSummary) {
  if (isSmitheryInstalled(server)) return;
  smitheryInstallingId.value = server.id;
  smitheryError.value = "";
  error.value = "";
  try {
    const detail = await getSmitheryMcpServer(server.qualifiedName);
    const plan = buildSmitheryMcpInstall(
      {
        ...detail,
        iconUrl: detail.iconUrl || server.iconUrl,
        homepage: detail.homepage || server.homepage,
      },
      { apiKey: settingStore.smitheryApiKey },
    );
    if (!plan) {
      smitheryError.value = copy.value.smitheryNoRemote;
      return;
    }
    const { install, requiredEnv } = plan;
    if (requiredEnv.length) {
      // Stash full install payload so saveEditor can keep metadata + cache the icon.
      pendingSmitheryInstall.value = install;
      editor.value = {
        mode: "create",
        id: install.id,
        title: install.title ?? "",
        description: install.description ?? "",
        command: install.command,
        argsText: (install.args ?? []).join(" "),
        envText: requiredEnv.map((item) => `${item.name}=`).join("\n"),
        enabled: true,
      };
      tab.value = "installed";
      error.value = copy.value.needsEnv(requiredEnv.map((item) => item.name).join(", "));
      return;
    }
    await persist([...servers.value, install]);
    void cacheIconForInstall(install);
    void ensureConnectedAfterInstall(install);
  } catch (err) {
    smitheryError.value = err instanceof Error ? err.message : String(err);
  } finally {
    smitheryInstallingId.value = "";
  }
}

watch(
  () => props.query,
  (value) => {
    if (value == null) return;
    if (tab.value === "catalog") {
      catalogQuery.value = value;
      void runCatalogSearch();
    } else if (tab.value === "smithery") {
      smitheryQuery.value = value;
      void runSmitherySearch();
    }
  },
);

onMounted(() => {
  smitheryApiKeyDraft.value = settingStore.smitheryApiKey;
  if (props.query?.trim()) {
    catalogQuery.value = props.query;
    smitheryQuery.value = props.query;
  }
  void refreshRuntimeSupport();
  startStatusPolling();
  void warmInstallIcons(
    (settingStore.mcpServers ?? []).map((server) => ({
      kind: "mcp" as const,
      cacheKey: server.id,
      url: server.iconUrl,
    })),
  );
});

onUnmounted(() => {
  stopStatusPolling();
});

watch(tab, (value) => {
  if (value === "installed") void refreshServerStatuses();
});

watch(
  () => settingStore.smitheryApiKey,
  (value) => {
    if (document.activeElement?.id !== "smithery-api-key") {
      smitheryApiKeyDraft.value = value;
    }
  },
);
</script>

<style scoped>
.mcp-settings {
  display: flex;
  flex-direction: column;
  gap: 0;
}
</style>

<style>
.smithery-config-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  background: color-mix(in srgb, #000 48%, transparent);
  backdrop-filter: blur(2px);
}

.smithery-config-dialog {
  position: fixed;
  top: 50%;
  left: 50%;
  z-index: 51;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: min(380px, calc(100vw - 32px));
  padding: 16px;
  border: 1px solid var(--peek-border, rgba(255, 255, 255, 0.14));
  border-radius: 12px;
  background: var(--peek-dialog-bg, var(--peek-surface, #ffffff));
  color: var(--peek-text, #f3f4f6);
  box-shadow: 0 18px 48px var(--peek-shadow, rgb(0 0 0 / 28%));
  transform: translate(-50%, -50%);
  outline: none;
}

.smithery-config-title {
  margin: 0;
  font-size: 14px;
  font-weight: 650;
  line-height: 1.35;
}

.smithery-config-desc {
  margin: 0;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-line;
  word-break: break-all;
}

.smithery-config-label {
  margin: 4px 0 0;
  font-size: 12px;
  font-weight: 550;
}

.smithery-config-dialog .smithery-key-input {
  width: 100%;
  height: 32px;
  box-sizing: border-box;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: transparent;
  padding: 0 10px;
  font-size: 12px;
  color: var(--foreground);
}

.smithery-config-dialog .smithery-key-input:focus {
  outline: none;
  border-color: color-mix(in srgb, var(--primary) 55%, var(--border));
}

.smithery-config-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  align-self: flex-start;
  border: 0;
  background: transparent;
  padding: 0;
  color: var(--primary);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.smithery-config-link:hover {
  text-decoration: underline;
}

.smithery-config-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 6px;
}
</style>
