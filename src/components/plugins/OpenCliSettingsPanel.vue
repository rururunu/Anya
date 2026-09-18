<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Download, RefreshCw, Stethoscope, ExternalLink } from "@lucide/vue";
import { useSettingStore } from "@/stores/setting";

export type OpenCliSetupStatus = {
  nodeOk: boolean;
  nodeVersion?: string | null;
  npmOk: boolean;
  npmVersion?: string | null;
  cliInstalled: boolean;
  cliBin: string;
  cliVersion?: string | null;
  doctorOk?: boolean | null;
  doctorOutput?: string | null;
  hint: string;
  extensionUrl: string;
  docsUrl: string;
};

export type OpenCliInstallResult = {
  ok: boolean;
  output: string;
  status: OpenCliSetupStatus;
};

const settingStore = useSettingStore();
const status = ref<OpenCliSetupStatus | null>(null);
const loading = ref(false);
const installing = ref(false);
const doctoring = ref(false);
const error = ref("");
const log = ref("");

const copy = computed(() =>
  settingStore.language === "zh-CN"
    ? {
        title: "OpenCLI 安装",
        hint: "一键安装官方 CLI（npm i -g @jackwener/opencli）。还需安装 Chrome 扩展 Browser Bridge。",
        refresh: "刷新状态",
        install: "一键安装 CLI",
        installing: "正在安装…",
        reinstall: "重新安装 / 更新",
        doctor: "运行 doctor",
        doctoring: "检查中…",
        extension: "安装 Chrome 扩展",
        docs: "OpenCLI 文档",
        node: "Node.js",
        npm: "npm",
        cli: "opencli",
        bridge: "Browser Bridge",
        ok: "正常",
        missing: "未检测到",
        unknown: "未检查",
        fail: "异常",
        ready: "可以启用插件并在聊天里使用。",
      }
    : {
        title: "OpenCLI setup",
        hint: "One-click install of the official CLI (npm i -g @jackwener/opencli). You still need the Chrome Browser Bridge extension.",
        refresh: "Refresh",
        install: "Install CLI",
        installing: "Installing…",
        reinstall: "Reinstall / update",
        doctor: "Run doctor",
        doctoring: "Checking…",
        extension: "Chrome extension",
        docs: "OpenCLI docs",
        node: "Node.js",
        npm: "npm",
        cli: "opencli",
        bridge: "Browser Bridge",
        ok: "OK",
        missing: "Missing",
        unknown: "Not checked",
        fail: "Failed",
        ready: "You can enable the plugin and use it in chat.",
      },
);

function badge(ok: boolean | null | undefined, presentLabel: string) {
  if (ok === true) return { cls: "is-ok", text: copy.value.ok };
  if (ok === false) return { cls: "is-bad", text: presentLabel };
  return { cls: "is-muted", text: copy.value.unknown };
}

async function load(quick = true) {
  loading.value = true;
  error.value = "";
  try {
    status.value = await invoke<OpenCliSetupStatus>("get_opencli_setup_status", {
      includeDoctor: !quick,
    });
    if (!quick && status.value.doctorOutput) {
      log.value = status.value.doctorOutput;
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function onInstall() {
  installing.value = true;
  error.value = "";
  log.value = "";
  try {
    const result = await invoke<OpenCliInstallResult>("install_opencli_cli");
    status.value = result.status;
    log.value = result.output;
    if (!result.ok) {
      error.value =
        settingStore.language === "zh-CN"
          ? "安装未完全成功，请查看日志。"
          : "Install did not fully succeed — see log.";
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    installing.value = false;
  }
}

async function onDoctor() {
  doctoring.value = true;
  error.value = "";
  try {
    status.value = await invoke<OpenCliSetupStatus>("run_opencli_doctor");
    log.value = status.value.doctorOutput || "";
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    doctoring.value = false;
  }
}

async function openExternal(url: string) {
  try {
    await openUrl(url);
  } catch {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}

onMounted(() => load(true));
</script>

<template>
  <div class="oc-setup">
    <div class="oc-head">
      <div>
        <h3>{{ copy.title }}</h3>
        <p>{{ copy.hint }}</p>
      </div>
      <button
        type="button"
        class="oc-icon-btn"
        :title="copy.refresh"
        :disabled="loading || installing || doctoring"
        @click="load(true)"
      >
        <RefreshCw :size="15" :stroke-width="1.75" :class="{ spin: loading }" />
      </button>
    </div>

    <p v-if="status" class="oc-hint">{{ status.hint }}</p>
    <p v-if="error" class="oc-error">{{ error }}</p>

    <ul v-if="status" class="oc-checks">
      <li>
        <span>{{ copy.node }}</span>
        <span class="oc-badge" :class="badge(status.nodeOk, copy.missing).cls">
          {{ status.nodeOk ? status.nodeVersion || copy.ok : copy.missing }}
        </span>
      </li>
      <li>
        <span>{{ copy.npm }}</span>
        <span class="oc-badge" :class="badge(status.npmOk, copy.missing).cls">
          {{ status.npmOk ? status.npmVersion || copy.ok : copy.missing }}
        </span>
      </li>
      <li>
        <span>{{ copy.cli }}</span>
        <span class="oc-badge" :class="badge(status.cliInstalled, copy.missing).cls">
          {{ status.cliInstalled ? status.cliVersion || status.cliBin || copy.ok : copy.missing }}
        </span>
      </li>
      <li>
        <span>{{ copy.bridge }}</span>
        <span class="oc-badge" :class="badge(status.doctorOk ?? null, copy.fail).cls">
          {{
            status.doctorOk === true
              ? copy.ok
              : status.doctorOk === false
                ? copy.fail
                : copy.unknown
          }}
        </span>
      </li>
    </ul>

    <div class="oc-actions">
      <button
        type="button"
        class="oc-primary"
        :disabled="installing || doctoring || (status !== null && !status.npmOk)"
        @click="onInstall"
      >
        <Download :size="15" :stroke-width="1.75" />
        {{ installing ? copy.installing : status?.cliInstalled ? copy.reinstall : copy.install }}
      </button>
      <button
        type="button"
        class="oc-secondary"
        :disabled="installing || doctoring || !status?.cliInstalled"
        @click="onDoctor"
      >
        <Stethoscope :size="15" :stroke-width="1.75" />
        {{ doctoring ? copy.doctoring : copy.doctor }}
      </button>
    </div>

    <div v-if="status" class="oc-links">
      <button type="button" class="oc-link" @click="openExternal(status.extensionUrl)">
        <ExternalLink :size="14" />
        {{ copy.extension }}
      </button>
      <button type="button" class="oc-link" @click="openExternal(status.docsUrl)">
        <ExternalLink :size="14" />
        {{ copy.docs }}
      </button>
    </div>

    <p v-if="status?.cliInstalled && status.doctorOk === true" class="oc-ready">
      {{ copy.ready }}
    </p>

    <pre v-if="log" class="oc-log">{{ log }}</pre>
  </div>
</template>

<style scoped>
.oc-setup {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 4px 2px 12px;
}

.oc-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.oc-head h3 {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
}

.oc-head p,
.oc-hint {
  margin: 4px 0 0;
  font-size: 12.5px;
  line-height: 1.45;
  color: var(--text-muted, #6b7280);
}

.oc-error {
  margin: 0;
  font-size: 12.5px;
  color: var(--danger, #dc2626);
}

.oc-ready {
  margin: 0;
  font-size: 12.5px;
  color: var(--success, #059669);
}

.oc-checks {
  list-style: none;
  margin: 0;
  padding: 0;
  border: 1px solid var(--border, #e5e7eb);
  border-radius: 10px;
  overflow: hidden;
}

.oc-checks li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  font-size: 13px;
  border-bottom: 1px solid var(--border, #e5e7eb);
}

.oc-checks li:last-child {
  border-bottom: none;
}

.oc-badge {
  font-size: 12px;
  max-width: 55%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--surface-2, #f3f4f6);
}

.oc-badge.is-ok {
  color: #047857;
  background: #d1fae5;
}

.oc-badge.is-bad {
  color: #b91c1c;
  background: #fee2e2;
}

.oc-badge.is-muted {
  color: #6b7280;
}

.oc-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.oc-primary,
.oc-secondary {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: none;
  border-radius: 8px;
  padding: 8px 12px;
  font-size: 13px;
  cursor: pointer;
}

.oc-primary {
  background: var(--accent, #0f172a);
  color: #fff;
}

.oc-secondary {
  background: var(--surface-2, #f3f4f6);
  color: var(--text, #111827);
}

.oc-primary:disabled,
.oc-secondary:disabled,
.oc-icon-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.oc-links {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.oc-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  border: none;
  background: transparent;
  color: var(--accent, #2563eb);
  font-size: 12.5px;
  cursor: pointer;
  padding: 0;
}

.oc-icon-btn {
  border: none;
  background: transparent;
  color: var(--text-muted, #6b7280);
  cursor: pointer;
  padding: 4px;
  border-radius: 6px;
}

.oc-log {
  margin: 0;
  max-height: 220px;
  overflow: auto;
  padding: 10px 12px;
  border-radius: 8px;
  background: var(--surface-2, #0b1220);
  color: #e5e7eb;
  font-size: 11.5px;
  line-height: 1.4;
  white-space: pre-wrap;
  word-break: break-word;
}

.spin {
  animation: oc-spin 0.9s linear infinite;
}

@keyframes oc-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
