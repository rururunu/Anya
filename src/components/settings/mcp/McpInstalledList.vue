<template>
  <div class="server-list">
    <p v-if="servers.length === 0 && !disabledActions" class="settings-empty">{{ copy.empty }}</p>
    <CatalogItemCard
      v-for="server in servers"
      :key="server.id"
      :title="serverTitle(server)"
      :vendor="serverVendor(server)"
      :meta="metaLine(server)"
      :description="server.description"
      :icon-url="server.iconUrl"
      :icon-cache-kind="'mcp'"
      :icon-cache-key="server.id"
      :icon-fallback="serverVendor(server) || serverTitle(server)"
      :pills="pillsFor(server)"
      :expand-label="copy.expand"
      :collapse-label="copy.collapse"
    >
      <template #action>
        <CatalogRoundAction
          v-if="showConnect(server)"
          :busy="isBusy(server.id)"
          :disabled="disabledActions"
          :label="isBusy(server.id) ? copy.connecting : copy.connectNow"
          :icon="Link2"
          @click="$emit('connect', server)"
        />
        <CatalogRoundAction
          v-if="showReauth(server)"
          :busy="isBusy(server.id)"
          :disabled="disabledActions"
          :label="isBusy(server.id) ? copy.connecting : copy.reauthenticate"
          :icon="RefreshCw"
          @click="$emit('reauthenticate', server)"
        />
        <SettingsToggle
          :model-value="server.enabled !== false"
          :title="copy.enabled"
          @click.prevent="$emit('toggle', server)"
        />
        <CatalogRoundAction
          :disabled="disabledActions"
          :label="copy.edit"
          :icon="Pencil"
          :lock-when-done="false"
          @click="$emit('edit', server)"
        />
        <CatalogRoundAction
          :disabled="disabledActions"
          :label="copy.remove"
          :icon="Trash2"
          :lock-when-done="false"
          @click="$emit('remove', server)"
        />
      </template>
    </CatalogItemCard>
  </div>
</template>

<script setup lang="ts">
import { Link2, Pencil, RefreshCw, Trash2 } from "@lucide/vue";
import CatalogItemCard from "@/components/settings/CatalogItemCard.vue";
import CatalogRoundAction from "@/components/settings/CatalogRoundAction.vue";
import SettingsToggle from "@/components/settings/SettingsToggle.vue";
import { isMcpRemoteServer, type McpServerRuntimeStatus } from "@/services/mcp/remote";
import type { McpServerConfig } from "@/types/setting";

const props = defineProps<{
  servers: McpServerConfig[];
  statuses: Record<string, McpServerRuntimeStatus>;
  busyId: string;
  disabledActions: boolean;
  copy: {
    empty: string;
    enabled: string;
    disabled: string;
    edit: string;
    remove: string;
    envCount: (count: number) => string;
    authConnected: string;
    authSaved: string;
    authNeeded: string;
    authLocal: string;
    authDisabled: string;
    reauthenticate: string;
    connectNow: string;
    connecting: string;
    expand: string;
    collapse: string;
  };
}>();

defineEmits<{
  toggle: [server: McpServerConfig];
  edit: [server: McpServerConfig];
  remove: [server: McpServerConfig];
  connect: [server: McpServerConfig];
  reauthenticate: [server: McpServerConfig];
}>();

function formatCommand(server: McpServerConfig) {
  return [server.command, ...(server.args ?? [])].filter(Boolean).join(" ");
}

function serverTitle(server: McpServerConfig) {
  return server.title?.trim() || server.qualifiedName?.trim() || server.id;
}

function serverVendor(server: McpServerConfig) {
  return server.qualifiedName?.trim() || "";
}

function metaLine(server: McpServerConfig) {
  const parts: string[] = [];
  if (!server.qualifiedName?.trim() && server.title?.trim() && server.title.trim() !== server.id) {
    parts.push(server.id);
  }
  parts.push(formatCommand(server));
  if (server.env?.length) {
    parts.push(props.copy.envCount(server.env.length));
  }
  return parts.join(" · ");
}

function isRemote(server: McpServerConfig) {
  return isMcpRemoteServer(server);
}

function isBusy(id: string) {
  return props.busyId === id;
}

function statusFor(server: McpServerConfig): McpServerRuntimeStatus | undefined {
  return props.statuses[server.id];
}

function authLabel(server: McpServerConfig) {
  const state = statusFor(server)?.state;
  switch (state) {
    case "connected":
      return props.copy.authConnected;
    case "authenticated":
      return props.copy.authSaved;
    case "needs_auth":
      return props.copy.authNeeded;
    case "disabled":
      return props.copy.authDisabled;
    case "local":
      return props.copy.authLocal;
    default:
      if (server.enabled === false) return props.copy.authDisabled;
      return isRemote(server) ? props.copy.authNeeded : props.copy.authLocal;
  }
}

function pillsFor(server: McpServerConfig) {
  const pills = [
    server.enabled !== false ? props.copy.enabled : props.copy.disabled,
    authLabel(server),
  ];
  return pills;
}

function showConnect(server: McpServerConfig) {
  if (server.enabled === false || !isRemote(server)) return false;
  const state = statusFor(server)?.state;
  return state === "needs_auth" || state === "authenticated" || !state;
}

function showReauth(server: McpServerConfig) {
  if (server.enabled === false || !isRemote(server)) return false;
  // Smithery hosted: always offer reauth so a "connected" vault can still be relinked.
  if (server.source === "smithery") return true;
  const status = statusFor(server);
  if (!status) return false;
  return (
    status.state === "connected" || status.state === "authenticated" || status.hasSavedCredentials
  );
}
</script>

<style scoped>
.server-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
</style>
