<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { RefreshCw, Trash2, ChevronDown, ChevronRight } from "@lucide/vue";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import { useSettingStore } from "@/stores/setting";

export type ComputerPlaybookStep = {
  kind: string;
  action?: string | null;
  name?: string | null;
  role?: string | null;
  keys?: string | null;
  text?: string | null;
  window?: string | null;
  note?: string | null;
  precondition?: string | null;
};

export type ComputerPlaybook = {
  id: string;
  intent: string;
  app?: string | null;
  exe?: string | null;
  version_last_ok?: string | null;
  steps: ComputerPlaybookStep[];
  success_assert?: string | null;
  confidence: number;
  status: "active" | "quarantined";
  success_count: number;
  fail_count: number;
  created_at: number;
  updated_at: number;
};

const settingStore = useSettingStore();
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const items = ref<ComputerPlaybook[]>([]);
const loading = ref(false);
const error = ref("");
const expanded = ref<string | null>(null);
const query = ref("");

const copy = computed(() =>
  settingStore.language === "zh-CN"
    ? {
        title: "已保存的 Playbook",
        hint: "Agent 学会的操作流程会保存在本机。可在此查看或删除；删除后 lookup 不再命中。",
        empty: "还没有学习到的 playbook。成功完成新流程后，Agent 会用 playbook save 写入。",
        refresh: "刷新",
        delete: "删除",
        deleteTitle: "删除 playbook？",
        deleteDesc: "确定删除这条学习流程吗？此操作无法撤销。",
        cancel: "取消",
        search: "搜索意图 / 应用…",
        steps: "步骤",
        conf: "置信度",
        okFail: "成功 / 失败",
        statusActive: "可用",
        statusQuarantined: "已隔离",
        updated: "更新于",
      }
    : {
        title: "Saved playbooks",
        hint: "Procedures the agent learned are stored on this machine. Review or delete them here.",
        empty:
          "No learned playbooks yet. After a novel success, the agent saves via playbook save.",
        refresh: "Refresh",
        delete: "Delete",
        deleteTitle: "Delete playbook?",
        deleteDesc: "Delete this learned procedure? This cannot be undone.",
        cancel: "Cancel",
        search: "Search intent / app…",
        steps: "Steps",
        conf: "Confidence",
        okFail: "OK / fail",
        statusActive: "Active",
        statusQuarantined: "Quarantined",
        updated: "Updated",
      },
);

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return items.value;
  return items.value.filter((p) => {
    const hay = [p.intent, p.app ?? "", p.exe ?? "", p.id].join(" ").toLowerCase();
    return hay.includes(q);
  });
});

async function load() {
  loading.value = true;
  error.value = "";
  try {
    items.value = await invoke<ComputerPlaybook[]>("list_computer_use_playbooks");
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
    items.value = [];
  } finally {
    loading.value = false;
  }
}

function toggleExpand(id: string) {
  expanded.value = expanded.value === id ? null : id;
}

function statusLabel(p: ComputerPlaybook) {
  return p.status === "quarantined" ? copy.value.statusQuarantined : copy.value.statusActive;
}

function formatTime(secs: number) {
  if (!secs) return "—";
  try {
    return new Date(secs * 1000).toLocaleString();
  } catch {
    return String(secs);
  }
}

function stepLine(s: ComputerPlaybookStep) {
  const parts = [
    s.kind,
    s.action,
    s.name && `name=${s.name}`,
    s.role && `role=${s.role}`,
    s.keys && `keys=${s.keys}`,
    s.text && `text=${s.text}`,
    s.window && `window=${s.window}`,
    s.note,
  ].filter(Boolean);
  return parts.join(" · ");
}

async function onDelete(p: ComputerPlaybook) {
  const ok = await confirmDialogRef.value?.ask({
    title: copy.value.deleteTitle,
    description: `${copy.value.deleteDesc}\n\n${p.intent}`,
    confirmLabel: copy.value.delete,
    cancelLabel: copy.value.cancel,
    tone: "danger",
  });
  if (!ok) return;
  try {
    await invoke("delete_computer_use_playbook", { id: p.id });
    if (expanded.value === p.id) expanded.value = null;
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

onMounted(load);
watch(
  () => settingStore.language,
  () => {
    /* labels only */
  },
);
</script>

<template>
  <div class="cu-playbooks">
    <AppConfirmDialog ref="confirmDialogRef" />
    <div class="cu-head">
      <div>
        <h3>{{ copy.title }}</h3>
        <p>{{ copy.hint }}</p>
      </div>
      <button type="button" class="cu-icon-btn" :title="copy.refresh" @click="load">
        <RefreshCw :size="15" :stroke-width="1.75" :class="{ spin: loading }" />
      </button>
    </div>

    <input v-model="query" class="cu-search" type="search" :placeholder="copy.search" />

    <p v-if="error" class="cu-error">{{ error }}</p>
    <p v-else-if="!loading && !filtered.length" class="cu-empty">{{ copy.empty }}</p>

    <ul v-else class="cu-list">
      <li v-for="p in filtered" :key="p.id" class="cu-item" :class="{ open: expanded === p.id }">
        <button type="button" class="cu-row" @click="toggleExpand(p.id)">
          <span class="cu-chevron" aria-hidden="true">
            <ChevronDown v-if="expanded === p.id" :size="14" />
            <ChevronRight v-else :size="14" />
          </span>
          <span class="cu-main">
            <strong>{{ p.intent }}</strong>
            <span class="cu-meta">
              {{ p.app || p.exe || "—" }}
              · {{ statusLabel(p) }} · {{ copy.conf }} {{ (p.confidence * 100).toFixed(0) }}% ·
              {{ p.steps.length }} {{ copy.steps }}
            </span>
          </span>
        </button>
        <button
          type="button"
          class="cu-icon-btn danger"
          :title="copy.delete"
          @click.stop="onDelete(p)"
        >
          <Trash2 :size="14" :stroke-width="1.75" />
        </button>

        <div v-if="expanded === p.id" class="cu-detail">
          <div class="cu-kv">
            <span>id</span>
            <code>{{ p.id }}</code>
            <span>{{ copy.okFail }}</span>
            <span>{{ p.success_count }} / {{ p.fail_count }}</span>
            <span>{{ copy.updated }}</span>
            <span>{{ formatTime(p.updated_at) }}</span>
            <span v-if="p.success_assert">assert</span>
            <span v-if="p.success_assert">{{ p.success_assert }}</span>
          </div>
          <ol class="cu-steps">
            <li v-for="(s, i) in p.steps" :key="i">{{ stepLine(s) }}</li>
          </ol>
        </div>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.cu-playbooks {
  display: flex;
  flex-direction: column;
  gap: 10px;
  max-width: 42rem;
}
.cu-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 8px;
}
.cu-head h3 {
  margin: 0;
  font-size: 13px;
  font-weight: 650;
}
.cu-head p {
  margin: 4px 0 0;
  font-size: 12.5px;
  line-height: 1.5;
  color: var(--peek-muted, #8b939e);
}
.cu-search {
  font: inherit;
  font-size: 12.5px;
  padding: 7px 10px;
  border-radius: 8px;
  border: 1px solid var(--peek-border, rgba(255, 255, 255, 0.12));
  background: var(--peek-input-bg, transparent);
  color: inherit;
}
.cu-empty,
.cu-error {
  margin: 0;
  font-size: 12.5px;
  color: var(--peek-muted, #8b939e);
}
.cu-error {
  color: var(--peek-danger, #c42b1c);
}
.cu-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.cu-item {
  display: grid;
  grid-template-columns: 1fr auto;
  grid-template-rows: auto auto;
  gap: 0 4px;
  border: 1px solid var(--peek-border, rgba(255, 255, 255, 0.1));
  border-radius: 10px;
  padding: 4px;
  background: color-mix(in srgb, var(--peek-text, #fff) 3%, transparent);
}
.cu-row {
  grid-column: 1;
  display: flex;
  align-items: flex-start;
  gap: 6px;
  text-align: left;
  border: none;
  background: transparent;
  color: inherit;
  font: inherit;
  padding: 6px 8px;
  cursor: pointer;
  border-radius: 8px;
  min-width: 0;
}
.cu-row:hover {
  background: color-mix(in srgb, var(--peek-text, #fff) 5%, transparent);
}
.cu-chevron {
  flex: none;
  margin-top: 2px;
  color: var(--peek-muted, #8b939e);
}
.cu-main {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.cu-main strong {
  font-size: 13px;
  font-weight: 600;
}
.cu-meta {
  font-size: 11.5px;
  color: var(--peek-muted, #8b939e);
}
.cu-icon-btn {
  grid-column: 2;
  grid-row: 1;
  align-self: start;
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  margin: 4px 4px 0 0;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--peek-muted, #8b939e);
  cursor: pointer;
}
.cu-icon-btn:hover {
  color: inherit;
  background: color-mix(in srgb, var(--peek-text, #fff) 8%, transparent);
}
.cu-icon-btn.danger:hover {
  color: #f87171;
  background: rgba(248, 113, 113, 0.12);
}
.cu-icon-btn .spin {
  animation: cu-spin 0.8s linear infinite;
}
@keyframes cu-spin {
  to {
    transform: rotate(360deg);
  }
}
.cu-detail {
  grid-column: 1 / -1;
  padding: 4px 10px 10px 28px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.cu-kv {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 4px 10px;
  font-size: 11.5px;
  color: var(--peek-muted, #8b939e);
}
.cu-kv code {
  font-size: 11px;
  word-break: break-all;
  color: inherit;
}
.cu-steps {
  margin: 0;
  padding-left: 1.1rem;
  font-size: 12px;
  line-height: 1.55;
}
.cu-steps li + li {
  margin-top: 4px;
}
</style>
