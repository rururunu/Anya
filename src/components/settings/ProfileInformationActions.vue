<script setup lang="ts">
import { Upload, Download, Trash2 } from "@lucide/vue";
import {
  DialogRoot,
  DialogPortal,
  DialogOverlay,
  DialogContent,
  DialogTitle,
  DialogDescription,
} from "reka-ui";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import { useUserInformationActions } from "@/composables/settings/useUserInformationActions";
import type { UserInformation } from "@/services/settings/userInformation";
const emit = defineEmits<{ imported: [data: UserInformation] }>();
const {
  zh,
  transferBusy,
  transferMessage,
  transferError,
  informationInputRef,
  resetConfirmRef,
  deletionOptionsOpen,
  deleteInformation,
  exportInformation,
  importInformation,
} = useUserInformationActions((data) => emit("imported", data));
</script>
<template>
  <AppConfirmDialog ref="resetConfirmRef" />
  <DialogRoot v-model:open="deletionOptionsOpen">
    <DialogPortal>
      <DialogOverlay class="deletion-overlay" />
      <DialogContent class="deletion-dialog" :aria-describedby="undefined">
        <DialogTitle>{{ zh ? "删除信息" : "Delete information" }}</DialogTitle>
        <DialogDescription>
          {{
            zh
              ? "选择删除范围，下一步需要确认。"
              : "Choose what to delete. Confirmation is required next."
          }}
        </DialogDescription>
        <button class="deletion-option" @click="deleteInformation('information-only')">
          <strong>{{ zh ? "仅删除信息" : "Delete information only" }}</strong>
          <span>
            {{
              zh
                ? "保留 API Key、供应商、模型与其他设置"
                : "Keep API keys, providers, models and settings"
            }}
          </span>
        </button>
        <button class="deletion-option deletion-danger" @click="deleteInformation('everything')">
          <strong>
            {{ zh ? "删除所有信息（包含 Key）" : "Delete everything (including keys)" }}
          </strong>
          <span>
            {{
              zh
                ? "清除所有本地数据与设置，恢复初始状态"
                : "Clear local data and settings, restore the initial state"
            }}
          </span>
        </button>
        <button @click="deletionOptionsOpen = false">{{ zh ? "取消" : "Cancel" }}</button>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
  <div class="profile-transfer-actions">
    <button
      type="button"
      :disabled="transferBusy"
      :title="
        zh ? '导入用户资料、配置及 API Key' : 'Import your profile, configuration and API keys'
      "
      @click="informationInputRef?.click()"
    >
      <Upload :size="14" :stroke-width="1.75" aria-hidden="true" />
      {{ zh ? "导入" : "Import" }}
    </button>
    <button
      type="button"
      :disabled="transferBusy"
      :title="
        zh
          ? '导出用户资料、配置及 API Key，请妥善保存文件'
          : 'Export your profile, configuration and API keys; keep the file private'
      "
      @click="exportInformation"
    >
      <Download :size="14" :stroke-width="1.75" aria-hidden="true" />
      {{ zh ? "导出" : "Export" }}
    </button>
    <button
      class="profile-reset-button"
      type="button"
      :disabled="transferBusy"
      :title="zh ? '选择删除信息的范围，确认后执行' : 'Choose what to delete, then confirm'"
      @click="deletionOptionsOpen = true"
    >
      <Trash2 :size="14" :stroke-width="1.75" aria-hidden="true" />
      {{ zh ? "删除" : "Delete" }}
    </button>
    <input
      ref="informationInputRef"
      type="file"
      accept=".json,application/json"
      hidden
      @change="importInformation"
    />
  </div>
  <p v-if="transferMessage" class="profile-status" :role="transferError ? 'alert' : 'status'">
    {{ transferMessage }}
  </p>
</template>
<style scoped>
.deletion-overlay {
  position: fixed;
  inset: 0;
  background: rgb(0 0 0 / 18%);
  z-index: 1000;
}
.deletion-dialog {
  position: fixed;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  width: min(420px, calc(100vw - 40px));
  padding: 24px;
  border-radius: 16px;
  background: var(--peek-input-bg);
  color: var(--peek-text);
  border: 1px solid var(--peek-border);
  box-shadow: 0 8px 28px rgb(0 0 0 / 10%);
  z-index: 1001;
  display: grid;
  gap: 14px;
}
.deletion-option {
  display: grid;
  gap: 6px;
  text-align: left;
  padding: 14px;
  border: 1px solid var(--peek-border);
  border-radius: 10px;
  background: transparent;
  cursor: pointer;
}
.deletion-option:hover {
  background: rgb(128 128 128 / 8%);
}
.deletion-option span {
  font-size: 12px;
  opacity: 0.65;
}
.deletion-danger strong {
  color: #dc4545;
}
.profile-transfer-actions .profile-reset-button {
  color: #df5c58;
}
.profile-transfer-actions {
  display: flex;
  flex-wrap: nowrap;
  align-items: center;
  justify-content: flex-end;
  gap: 16px;
  margin-bottom: 8px;
  font-size: 12px;
}
.profile-transfer-actions button {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  flex: none;
  white-space: nowrap;
  padding: 6px 0;
}
.profile-transfer-actions button svg {
  flex: none;
}
.profile-transfer-actions button:disabled {
  opacity: 0.5;
  cursor: default;
}
.profile-status {
  font-size: 12px;
  color: var(--peek-muted);
  text-align: right;
}
.profile-status[role="alert"] {
  color: #df5c58;
}
</style>
