<template>
  <section class="connect-phone">
    <div class="connect-shell">
      <header class="connect-header">
        <h2>{{ copy.title }}</h2>
        <p>{{ copy.subtitle }}</p>
      </header>

      <div class="status-line" :data-running="status?.running ? '1' : '0'">
        <span class="status-dot" aria-hidden="true" />
        <span class="status-text">
          {{
            status?.running
              ? copy.portLabel.replace("{port}", String(status.port))
              : copy.gatewayOff
          }}
        </span>
        <button
          type="button"
          class="setting-toggle"
          :class="{ active: Boolean(status?.running) }"
          :aria-pressed="Boolean(status?.running)"
          :aria-label="status?.running ? copy.stop : copy.start"
          :title="status?.running ? copy.stop : copy.start"
          :disabled="busy"
          @click="toggleGateway"
        >
          <Loader2 v-if="busy" class="setting-toggle-spinner" />
          <span v-else class="setting-toggle-knob" />
        </button>
      </div>
      <p v-if="tunnelConnecting" class="tunnel-connecting-hint">{{ copy.tunnelConnecting }}</p>

      <div class="tabs" role="tablist" :aria-label="copy.connectMethods">
        <button
          type="button"
          role="tab"
          class="tab"
          :class="{ active: tab === 'qr' }"
          :aria-selected="tab === 'qr'"
          @click="tab = 'qr'"
        >
          {{ copy.tabQr }}
        </button>
        <button
          type="button"
          role="tab"
          class="tab"
          :class="{ active: tab === 'code' }"
          :aria-selected="tab === 'code'"
          @click="tab = 'code'"
        >
          {{ copy.tabCode }}
        </button>
      </div>

      <div class="public-tunnel-trigger">
        <Button
          variant="outline"
          size="sm"
          class="h-8 gap-1.5 public-tunnel-trigger-button"
          @click="tunnelConfigOpen = true"
        >
          {{ copy.publicTunnelConfigButton }}
        </Button>
      </div>

      <DialogRoot :open="tunnelConfigOpen" @update:open="tunnelConfigOpen = $event">
        <DialogPortal>
          <DialogOverlay class="public-tunnel-overlay" />
          <DialogContent class="public-tunnel-dialog" :aria-describedby="undefined">
            <DialogTitle class="public-tunnel-title">
              {{ copy.publicTunnelConfigTitle }}
            </DialogTitle>
            <DialogDescription class="public-tunnel-desc">
              {{ copy.publicTunnelConfigDesc }}
            </DialogDescription>

            <section class="public-tunnel-guide">
              <button
                type="button"
                class="public-tunnel-section-toggle"
                @click="tunnelGuideOpen = !tunnelGuideOpen"
              >
                {{ copy.tunnelGuideTitle }}
                <span aria-hidden="true">{{ tunnelGuideOpen ? "−" : "+" }}</span>
              </button>
              <div v-if="tunnelGuideOpen" class="public-tunnel-guide-body">
                <p class="public-tunnel-guide-text">{{ copy.tunnelGuideSummary }}</p>
                <div class="public-tunnel-cmd">
                  <code>{{ quickTunnelCommand }}</code>
                  <button
                    type="button"
                    class="public-tunnel-cmd-copy"
                    :title="commandCopied ? copy.copied : copy.copyCommand"
                    :aria-label="commandCopied ? copy.copied : copy.copyCommand"
                    @click="copyQuickTunnelCommand"
                  >
                    <Copy :size="13" :stroke-width="1.75" />
                  </button>
                </div>
              </div>
            </section>

            <label class="public-tunnel-switch">
              <span>{{ copy.tunnelEnabled }}</span>
              <input type="checkbox" v-model="cloudflaredEnabled" />
            </label>

            <div v-if="cloudflaredEnabled" class="public-tunnel-fields">
              <label class="public-tunnel-row">
                <input type="checkbox" v-model="useQuickTunnel" />
                <span>{{ copy.tunnelModeQuick }}</span>
              </label>

              <template v-if="!useQuickTunnel">
                <label class="public-tunnel-label" for="cloudflared-hostname">
                  {{ copy.tunnelHostname }}
                </label>
                <input
                  id="cloudflared-hostname"
                  v-model="cloudflaredHostname"
                  type="text"
                  class="public-tunnel-input"
                  :placeholder="copy.tunnelHostnamePlaceholder"
                  autocomplete="off"
                  spellcheck="false"
                />
              </template>
              <p v-else class="public-tunnel-guide-text">
                {{ copy.tunnelQuickAutoHint }}
              </p>

              <button
                type="button"
                class="public-tunnel-section-toggle"
                @click="tunnelAdvancedOpen = !tunnelAdvancedOpen"
              >
                {{ copy.advanced }}
                <span aria-hidden="true">{{ tunnelAdvancedOpen ? "−" : "+" }}</span>
              </button>

              <div v-if="tunnelAdvancedOpen" class="public-tunnel-advanced">
                <template v-if="!useQuickTunnel">
                  <label class="public-tunnel-label" for="cloudflared-token">
                    {{ copy.tunnelToken }}
                  </label>
                  <input
                    id="cloudflared-token"
                    v-model="cloudflaredToken"
                    type="text"
                    class="public-tunnel-input"
                    :placeholder="copy.optional"
                    autocomplete="off"
                    spellcheck="false"
                  />
                </template>

                <label class="public-tunnel-label" for="cloudflared-binary">
                  {{ copy.tunnelBinary }}
                </label>
                <input
                  id="cloudflared-binary"
                  v-model="cloudflaredBinary"
                  type="text"
                  class="public-tunnel-input"
                  autocomplete="off"
                  spellcheck="false"
                />

                <a
                  class="public-tunnel-link"
                  href="https://dash.cloudflare.com/"
                  target="_blank"
                  rel="noreferrer"
                >
                  Cloudflare Dashboard
                </a>
              </div>
            </div>

            <p v-if="tunnelLoadError" class="form-error">{{ tunnelLoadError }}</p>
            <p v-if="tunnelConnecting" class="tunnel-connecting-hint">
              {{ copy.tunnelConnecting }}
            </p>

            <div class="public-tunnel-actions">
              <Button
                variant="outline"
                size="sm"
                class="h-8"
                :disabled="busy"
                @click="tunnelConfigOpen = false"
              >
                {{ copy.cancel }}
              </Button>
              <Button size="sm" class="h-8 gap-1.5" :disabled="busy" @click="saveTunnelPrefs">
                <Loader2 v-if="busy" :size="13" :stroke-width="1.75" class="spin" />
                {{ copy.save }}
              </Button>
            </div>
          </DialogContent>
        </DialogPortal>
      </DialogRoot>

      <p v-if="error" class="form-error">{{ error }}</p>

      <div class="tab-panel" role="tabpanel">
        <template v-if="tab === 'qr'">
          <div class="hero">
            <div class="qr-frame">
              <img v-if="pairing?.qrDataUrl" :src="pairing.qrDataUrl" alt="" />
              <div v-else class="qr-placeholder">{{ copy.qrEmpty }}</div>
            </div>
            <p class="hint">{{ copy.qrHint }}</p>
            <p v-if="expiresLabel" class="expires">{{ expiresLabel }}</p>
            <Button size="sm" class="h-8 gap-1.5" :disabled="busy" @click="refreshPairing">
              <Loader2 v-if="busy" :size="13" :stroke-width="1.75" class="spin" />
              <RefreshCw v-else :size="13" :stroke-width="1.75" />
              {{ pairing ? copy.refreshCode : copy.generateCode }}
            </Button>
          </div>
        </template>

        <template v-else>
          <div class="hero">
            <p class="eyebrow">{{ copy.pairingCode }}</p>
            <div class="code-block">
              <code>{{ displayCode }}</code>
              <Button
                variant="ghost"
                size="icon"
                class="size-8 shrink-0"
                :disabled="!pairing"
                :title="copy.copy"
                :aria-label="copy.copy"
                @click="copyText(displayCode)"
              >
                <Copy :size="14" :stroke-width="1.75" />
              </Button>
            </div>
            <p class="hint">{{ copy.pairingCodeHint }}</p>
            <p v-if="connectionSummary" class="meta">{{ connectionSummary }}</p>
            <p v-if="expiresLabel" class="expires">{{ expiresLabel }}</p>
            <div class="hero-actions">
              <Button
                variant="outline"
                size="sm"
                class="h-8 gap-1.5"
                :disabled="!pairing"
                @click="copyText(pairing?.token || '')"
              >
                <Copy :size="13" :stroke-width="1.75" />
                {{ copy.copyToken }}
              </Button>
              <Button size="sm" class="h-8 gap-1.5" :disabled="busy" @click="refreshPairing">
                <Loader2 v-if="busy" :size="13" :stroke-width="1.75" class="spin" />
                <RefreshCw v-else :size="13" :stroke-width="1.75" />
                {{ pairing ? copy.refreshCode : copy.generateCode }}
              </Button>
            </div>
          </div>
        </template>
      </div>

      <section class="devices">
        <div class="devices-header">
          <h3>{{ copy.devices }}</h3>
          <span>
            {{ copy.connectedCount.replace("{n}", String(status?.connectedClients ?? 0)) }}
          </span>
        </div>
        <p v-if="!status?.devices?.length" class="empty">{{ copy.noDevices }}</p>
        <ul v-else class="device-list">
          <li v-for="device in status?.devices || []" :key="device.deviceId" class="device-item">
            <div class="device-info">
              <strong>{{ device.deviceName || device.deviceId.slice(0, 8) }}</strong>
              <span>{{ formatTime(device.lastSeenEpochMs) }}</span>
            </div>
            <button type="button" class="revoke" :disabled="busy" @click="revoke(device.deviceId)">
              {{ copy.revoke }}
            </button>
          </li>
        </ul>
      </section>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Copy, Loader2, RefreshCw } from "@lucide/vue";
import {
  remoteCreatePairing,
  remoteGatewayStart,
  remoteGatewayStatus,
  remoteGatewayStop,
  remoteRevokeDevice,
  remoteGetTunnelPrefs,
  remoteSetTunnelPrefs,
  type GatewayStatus,
  type PairingSessionInfo,
  type TunnelPrefs,
} from "@/commands/remote";
import { Button } from "@/components/ui/button";
import {
  DialogContent,
  DialogDescription,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
} from "reka-ui";
import { useSettingStore } from "@/stores/setting";

type ConnectTab = "qr" | "code";

const settingStore = useSettingStore();
const status = ref<GatewayStatus | null>(null);
const pairing = ref<PairingSessionInfo | null>(null);
const busy = ref(false);
const tunnelConnecting = ref(false);
const error = ref("");
const now = ref(Date.now());
const tab = ref<ConnectTab>("qr");
let unlisten: UnlistenFn | null = null;
let timer: number | undefined;

// Cloudflare tunnel (optional): pairing will use wss public host.
const cloudflaredEnabled = ref(false);
const cloudflaredToken = ref("");
const cloudflaredHostname = ref("");
const cloudflaredBinary = ref("cloudflared");
const useQuickTunnel = ref(true);
const tunnelLoadError = ref("");
const tunnelConfigOpen = ref(false);
const tunnelGuideOpen = ref(true);
const tunnelAdvancedOpen = ref(false);
const commandCopied = ref(false);
let commandCopiedTimer: number | undefined;

const isChinese = computed(() => settingStore.language === "zh-CN");

const copy = computed(() =>
  isChinese.value
    ? {
        title: "连接手机",
        subtitle: "用 Companion 扫码或输入配对码，远程查看会话与处理审批。",
        connectMethods: "连接方式",
        gatewayOn: "远程网关已开启",
        gatewayOff: "网关未开启",
        portLabel: "已开启 · 端口 {port}",
        start: "开启",
        stop: "停止",
        tabQr: "扫码",
        tabCode: "配对码",
        qrHint: "打开手机端扫一扫即可配对",
        qrEmpty: "点击下方生成二维码",
        generateCode: "生成",
        refreshCode: "刷新",
        pairingCode: "配对码",
        pairingCodeHint: "在手机端「配对令牌」中填写此短码",
        copy: "复制",
        copyToken: "复制完整令牌",
        devices: "已配对设备",
        connectedCount: "在线 {n}",
        noDevices: "还没有手机连上",
        revoke: "解除",
        expiresIn: "{m} 分钟后失效",
        expired: "已过期，请刷新",
        tunnelEnabled: "启用公网连接",
        tunnelToken: "Token",
        tunnelHostname: "公网 Hostname",
        tunnelHostnamePlaceholder: "xxxx.trycloudflare.com",
        tunnelBinary: "cloudflared 路径",
        tunnelModeQuick: "Quick Tunnel（自动解析域名）",
        tunnelQuickAutoHint:
          "保存并刷新后，配对地址会自动变成日志里的 wss://*.trycloudflare.com/remote/v1。",
        tunnelGuideTitle: "使用指南",
        tunnelGuideSummary:
          "开启网关后，用 App 自动拉起隧道；也可手动运行命令。Quick Tunnel 会自动读日志域名。",
        copyCommand: "复制命令",
        copied: "已复制",
        optional: "可选",
        advanced: "高级选项",
        save: "保存",
        cancel: "取消",
        publicTunnelConfigButton: "公网连接设置",
        publicTunnelConfigTitle: "公网连接",
        publicTunnelConfigDesc: "让手机在任意网络下连上这台电脑。",
        tunnelConnecting: "正在建立公网隧道，可能需要几秒到 25 秒，请稍候…",
      }
    : {
        title: "Connect phone",
        subtitle: "Scan or enter a code in Companion to view sessions and approvals.",
        connectMethods: "Connection methods",
        gatewayOn: "Remote gateway on",
        gatewayOff: "Gateway off",
        portLabel: "On · port {port}",
        start: "Start",
        stop: "Stop",
        tabQr: "Scan",
        tabCode: "Code",
        qrHint: "Open Companion and scan to pair",
        qrEmpty: "Generate a QR code below",
        generateCode: "Generate",
        refreshCode: "Refresh",
        pairingCode: "Pairing code",
        pairingCodeHint: "Enter this short code as the pairing token",
        copy: "Copy",
        copyToken: "Copy full token",
        devices: "Paired devices",
        connectedCount: "{n} online",
        noDevices: "No phones paired yet",
        revoke: "Revoke",
        expiresIn: "Expires in {m} min",
        expired: "Expired — refresh",
        tunnelEnabled: "Enable public access",
        tunnelToken: "Token",
        tunnelHostname: "Public hostname",
        tunnelHostnamePlaceholder: "xxxx.trycloudflare.com",
        tunnelBinary: "cloudflared path",
        tunnelModeQuick: "Quick Tunnel (auto hostname)",
        tunnelQuickAutoHint:
          "After save & refresh, pairing shows wss://*.trycloudflare.com/remote/v1 from cloudflared logs.",
        tunnelGuideTitle: "Guide",
        tunnelGuideSummary:
          "Start the gateway, then let the app launch the tunnel (or run the command). Quick Tunnel reads the hostname from logs.",
        copyCommand: "Copy command",
        copied: "Copied",
        optional: "Optional",
        advanced: "Advanced",
        save: "Save",
        cancel: "Cancel",
        publicTunnelConfigButton: "Public tunnel settings",
        publicTunnelConfigTitle: "Public access",
        publicTunnelConfigDesc: "Let your phone connect from any network.",
        tunnelConnecting: "Setting up the public tunnel — this can take up to 25s…",
      },
);

const quickTunnelCommand = computed(() => {
  const port = status.value?.port || 8787;
  return `cloudflared tunnel --url http://127.0.0.1:${port}`;
});

const displayCode = computed(() => {
  const code = pairing.value?.pairingCode || "";
  if (code.length === 8) return `${code.slice(0, 4)}-${code.slice(4)}`;
  return code || "————";
});

const connectionSummary = computed(() => {
  const p = pairing.value;
  if (!p?.host || !p.port) return "";
  // Gateway WebSocket path is fixed in remote gateway.
  const path = "/remote/v1";
  if (p.scheme === "wss") {
    // Default TLS port can be omitted for readability.
    return p.port === 443 ? `wss://${p.host}${path}` : `wss://${p.host}:${p.port}${path}`;
  }
  return `ws://${p.host}:${p.port}${path}`;
});

const expiresLabel = computed(() => {
  if (!pairing.value) return "";
  const remain = pairing.value.expiresAtEpochMs - now.value;
  if (remain <= 0) return copy.value.expired;
  const minutes = Math.max(1, Math.ceil(remain / 60000));
  return copy.value.expiresIn.replace("{m}", String(minutes));
});

onMounted(async () => {
  await refreshStatus();
  try {
    const prefs: TunnelPrefs = await remoteGetTunnelPrefs();
    cloudflaredEnabled.value = Boolean(prefs.cloudflaredEnabled);
    cloudflaredToken.value = prefs.cloudflaredToken ?? "";
    cloudflaredHostname.value = prefs.cloudflaredHostname ?? "";
    cloudflaredBinary.value = prefs.cloudflaredBinary ?? "cloudflared";
    useQuickTunnel.value = Boolean(prefs.useQuickTunnel);
  } catch (e) {
    tunnelLoadError.value = String(e);
  }
  unlisten = await listen<GatewayStatus>("remote-gateway-status", (event) => {
    status.value = event.payload;
    if (event.payload.pairing) pairing.value = event.payload.pairing;
  });
  timer = window.setInterval(() => {
    now.value = Date.now();
  }, 15000);
});

onUnmounted(() => {
  unlisten?.();
  if (timer) window.clearInterval(timer);
  if (commandCopiedTimer) window.clearTimeout(commandCopiedTimer);
});

async function refreshStatus() {
  status.value = await remoteGatewayStatus();
  if (status.value.pairing) pairing.value = status.value.pairing;
}

async function startGateway() {
  busy.value = true;
  error.value = "";
  try {
    status.value = await remoteGatewayStart();
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function stopGateway() {
  busy.value = true;
  error.value = "";
  try {
    status.value = await remoteGatewayStop();
    pairing.value = null;
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function toggleGateway() {
  if (status.value?.running) {
    await stopGateway();
  } else {
    await startGateway();
  }
}

async function refreshPairing() {
  busy.value = true;
  error.value = "";
  // Establishing the public tunnel (first time, or after it dropped) blocks
  // on cloudflared for up to ~25s with no incremental progress to report —
  // without this, the UI looks stuck rather than working.
  tunnelConnecting.value = cloudflaredEnabled.value;
  try {
    pairing.value = await remoteCreatePairing();
    await refreshStatus();
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
    tunnelConnecting.value = false;
  }
}

async function saveTunnelPrefs() {
  busy.value = true;
  error.value = "";
  tunnelLoadError.value = "";
  try {
    await remoteSetTunnelPrefs({
      cloudflaredEnabled: cloudflaredEnabled.value,
      cloudflaredToken:
        !useQuickTunnel.value && cloudflaredToken.value.trim()
          ? cloudflaredToken.value.trim()
          : null,
      // Quick Tunnel ignores saved hostname and reads *.trycloudflare.com from logs.
      cloudflaredHostname:
        !useQuickTunnel.value && cloudflaredHostname.value.trim()
          ? cloudflaredHostname.value.trim()
          : null,
      cloudflaredBinary: cloudflaredBinary.value.trim() || "cloudflared",
      useQuickTunnel: useQuickTunnel.value,
    });
    tunnelConfigOpen.value = false;
    if (status.value?.running) {
      // refreshPairing drives its own busy/tunnelConnecting indicators.
      await refreshPairing();
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
    tunnelConnecting.value = false;
  }
}

async function revoke(deviceId: string) {
  busy.value = true;
  error.value = "";
  try {
    status.value = await remoteRevokeDevice(deviceId);
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function copyText(value: string) {
  if (!value) return;
  try {
    await navigator.clipboard.writeText(value);
  } catch {
    error.value = isChinese.value ? "复制失败" : "Copy failed";
  }
}

async function copyQuickTunnelCommand() {
  await copyText(quickTunnelCommand.value);
  commandCopied.value = true;
  if (commandCopiedTimer) window.clearTimeout(commandCopiedTimer);
  commandCopiedTimer = window.setTimeout(() => {
    commandCopied.value = false;
  }, 1500);
}

function formatTime(epochMs: number) {
  if (!epochMs) return "";
  return new Date(epochMs).toLocaleString();
}
</script>

<style scoped>
.connect-phone {
  display: flex;
  justify-content: center;
  width: 100%;
  min-height: 100%;
  padding: 28px 20px 24px;
  box-sizing: border-box;
}

.connect-shell {
  width: min(100%, 420px);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 18px;
  text-align: center;
}

.connect-header h2 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  letter-spacing: -0.02em;
}

.connect-header p {
  margin: 8px 0 0;
  color: var(--muted-foreground);
  font-size: 13px;
  line-height: 1.5;
}

.status-line {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  max-width: 100%;
  padding: 6px 8px 6px 12px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--muted-foreground) 8%, transparent);
  font-size: 12px;
  color: var(--muted-foreground);
}

.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--muted-foreground);
  flex: none;
}

.status-line[data-running="1"] .status-dot {
  background: #1f9d55;
}

.status-text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.setting-toggle {
  position: relative;
  width: 36px;
  height: 20px;
  border: 0;
  border-radius: 999px;
  background: color-mix(in srgb, var(--muted-foreground) 28%, transparent);
  cursor: pointer;
  padding: 0;
  flex: none;
}

.setting-toggle:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.setting-toggle.active {
  background: #1f9d55;
}

.setting-toggle-knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  border-radius: 999px;
  background: white;
  transition: transform 140ms ease;
  box-shadow: 0 1px 2px color-mix(in srgb, #000 18%, transparent);
}

.setting-toggle.active .setting-toggle-knob {
  transform: translateX(16px);
}

.setting-toggle-spinner {
  position: absolute;
  top: 2px;
  left: 10px;
  width: 16px;
  height: 16px;
  color: white;
  animation: peek-connect-spin 0.8s linear infinite;
}

.tunnel-connecting-hint {
  margin: 0;
  color: var(--muted-foreground);
  font-size: 11px;
  line-height: 1.5;
}

.spin {
  animation: peek-connect-spin 0.8s linear infinite;
}

@keyframes peek-connect-spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.tabs {
  display: flex;
  gap: 4px;
  padding: 3px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--muted-foreground) 10%, transparent);
}

.tab {
  border: 0;
  background: transparent;
  color: var(--muted-foreground);
  font-size: 12px;
  padding: 6px 16px;
  border-radius: 8px;
  cursor: pointer;
}

.tab.active {
  background: var(--background);
  color: var(--foreground);
}

.form-error,
.hint,
.expires,
.meta,
.empty {
  margin: 0;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.5;
}

.form-error {
  color: #ef4444;
}

.tab-panel {
  width: 100%;
}

.hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.qr-frame {
  width: 220px;
  height: 220px;
  display: grid;
  place-items: center;
  background: #fff;
  border: 1px solid color-mix(in srgb, var(--border, var(--peek-border)) 85%, transparent);
  border-radius: 16px;
  overflow: hidden;
  box-shadow: 0 10px 28px color-mix(in srgb, #000 6%, transparent);
}

.qr-frame img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.qr-placeholder {
  color: var(--muted-foreground);
  font-size: 12px;
  padding: 20px;
}

.eyebrow {
  margin: 0;
  font-size: 12px;
  font-weight: 600;
  color: var(--muted-foreground);
}

.code-block {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 10px 10px 10px 18px;
  border-radius: 14px;
  background: color-mix(in srgb, var(--muted-foreground) 8%, transparent);
}

.code-block code {
  font-size: 32px;
  font-weight: 650;
  letter-spacing: 0.16em;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  line-height: 1;
}

.hero-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 8px;
}

.devices {
  width: 100%;
  margin-top: 8px;
  padding-top: 18px;
  border-top: 1px solid color-mix(in srgb, var(--border, var(--peek-border)) 80%, transparent);
  text-align: left;
}

.devices-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 12px;
  margin-bottom: 10px;
}

.devices-header h3 {
  margin: 0;
  font-size: 12px;
  font-weight: 600;
}

.devices-header span {
  font-size: 11px;
  color: var(--muted-foreground);
}

.device-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.device-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  padding: 10px 0;
}

.device-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.device-info strong {
  font-size: 13px;
  font-weight: 600;
}

.device-info span {
  font-size: 11px;
  color: var(--muted-foreground);
}

.revoke {
  border: 0;
  background: transparent;
  color: #c23b22;
  font: inherit;
  font-size: 12px;
  cursor: pointer;
  padding: 4px 2px;
}

.revoke:disabled {
  opacity: 0.5;
  cursor: default;
}

.tunnel-config {
  width: 100%;
  padding-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 0;
  align-items: center;
}

.tunnel-config-button {
  max-width: 560px;
  margin: 0 auto;
  align-self: center;
  width: fit-content;
}

.public-tunnel-trigger {
  width: 100%;
  display: flex;
  justify-content: center;
  margin-top: 8px;
}

.public-tunnel-trigger-button {
  max-width: 560px;
}

.public-tunnel-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  background: color-mix(in srgb, #000 48%, transparent);
  backdrop-filter: blur(2px);
}

.public-tunnel-dialog {
  position: fixed;
  top: 50%;
  left: 50%;
  z-index: 51;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 14px;
  width: min(380px, calc(100vw - 32px));
  padding: 18px 16px 14px;
  border: 1px solid var(--peek-border, rgba(255, 255, 255, 0.14));
  border-radius: 14px;
  background: var(--peek-dialog-bg, var(--peek-surface, #252526));
  color: var(--peek-text, #f3f4f6);
  box-shadow: 0 18px 48px var(--peek-shadow, rgb(0 0 0 / 28%));
  transform: translate(-50%, -50%);
  outline: none;
}

.public-tunnel-title {
  margin: 0;
  font-size: 15px;
  font-weight: 650;
  line-height: 1.3;
  letter-spacing: -0.02em;
}

.public-tunnel-desc {
  margin: -4px 0 0;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.45;
}

.public-tunnel-guide {
  display: flex;
  flex-direction: column;
  gap: 8px;
  text-align: left;
}

.public-tunnel-guide-body {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.public-tunnel-guide-text {
  margin: 0;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.45;
}

.public-tunnel-cmd {
  display: flex;
  align-items: stretch;
  gap: 0;
  min-width: 0;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--muted-foreground) 6%, transparent);
}

.public-tunnel-cmd code {
  flex: 1;
  min-width: 0;
  padding: 10px 12px;
  color: var(--foreground);
  font-size: 11px;
  line-height: 1.4;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  word-break: break-all;
  text-align: left;
}

.public-tunnel-cmd-copy {
  display: grid;
  place-items: center;
  width: 36px;
  border: 0;
  border-left: 1px solid var(--border);
  background: transparent;
  color: var(--muted-foreground);
  cursor: pointer;
}

.public-tunnel-cmd-copy:hover {
  color: var(--foreground);
  background: color-mix(in srgb, var(--muted-foreground) 8%, transparent);
}

.public-tunnel-switch {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  font-size: 13px;
  font-weight: 550;
  color: var(--foreground);
}

.public-tunnel-switch input {
  width: 16px;
  height: 16px;
  accent-color: var(--primary);
}

.public-tunnel-fields {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.public-tunnel-label {
  margin: 0;
  font-size: 12px;
  font-weight: 550;
  color: var(--muted-foreground);
  text-align: left;
}

.public-tunnel-input {
  width: 100%;
  height: 34px;
  box-sizing: border-box;
  border-radius: 9px;
  border: 1px solid var(--border);
  background: transparent;
  padding: 0 11px;
  font-size: 12px;
  color: var(--foreground);
}

.public-tunnel-input:focus {
  outline: none;
  border-color: color-mix(in srgb, var(--primary) 55%, var(--border));
}

.public-tunnel-section-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--muted-foreground);
  font: inherit;
  font-size: 12px;
  font-weight: 550;
  cursor: pointer;
  text-align: left;
}

.public-tunnel-section-toggle:hover {
  color: var(--foreground);
}

.public-tunnel-advanced {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-top: 2px;
}

.public-tunnel-row {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 12px;
  color: var(--muted-foreground);
}

.public-tunnel-link {
  color: var(--primary);
  font-size: 12px;
  text-decoration: none;
  text-align: left;
}

.public-tunnel-link:hover {
  text-decoration: underline;
  text-underline-offset: 2px;
}

.public-tunnel-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 2px;
}

.tunnel-toggle {
  appearance: none;
  border: 1px solid color-mix(in srgb, var(--border, var(--peek-border)) 70%, transparent);
  background: transparent;
  width: 100%;
  max-width: 560px;
  cursor: pointer;
  padding: 10px 14px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  color: var(--foreground);
  border-radius: 12px;
}

.tunnel-toggle-main {
  display: inline-flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  font-weight: 600;
  align-items: center;
}

.tunnel-toggle-sub {
  font-size: 11px;
  font-weight: 400;
  color: var(--muted-foreground);
  text-align: center;
}

.tunnel-toggle-icon {
  font-size: 14px;
  line-height: 1;
  color: var(--muted-foreground);
  width: 22px;
  text-align: center;
  border-radius: 8px;
  background: color-mix(in srgb, var(--muted-foreground) 6%, transparent);
  padding: 5px 0;
}

.tunnel-expanded {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 10px;
  padding: 0;
  border-radius: 0;
  border: 0;
  background: transparent;
  width: 100%;
  max-width: 560px;
  align-items: center;
}

.tunnel-sub {
  font-size: 11px;
  color: var(--muted-foreground);
}

.tunnel-row {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: 12px;
  font-size: 12px;
  color: var(--muted-foreground);
  width: 100%;
}

.tunnel-fields {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.tunnel-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
}

.tunnel-label {
  font-size: 12px;
  color: var(--muted-foreground);
  text-align: left;
}

.tunnel-input {
  width: 100%;
  border: 1px solid color-mix(in srgb, var(--border, var(--peek-border)) 85%, transparent);
  border-radius: 10px;
  background: transparent;
  padding: 8px 10px;
  font-size: 12px;
  color: var(--foreground);
  outline: none;
  max-width: 420px;
  margin-left: auto;
  margin-right: auto;
}

.tunnel-note {
  margin: 0;
  font-size: 11px;
  color: var(--muted-foreground);
}

.tunnel-save {
  margin: 0 auto;
}
</style>
