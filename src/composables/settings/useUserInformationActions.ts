import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import { getAppSettings, setAppSettings } from "@/services/ipc";
import { useSettingStore } from "@/stores/setting";
import { useChatModelStore } from "@/stores/chatModel";
import {
  exportUserInformation,
  parseUserInformation,
  userInformationKeys,
  informationStorage,
  applyInformationStorage,
  type UserInformation,
} from "@/services/settings/userInformation";

export type InformationDeletionScope = "information-only" | "everything";

/** Owns backup, transactional import and confirmed reset; profile presentation stays separate. */
export function useUserInformationActions(onImported: (data: UserInformation) => void) {
  const settingStore = useSettingStore();
  const zh = computed(() => settingStore.language === "zh-CN");
  const transferBusy = ref(false);
  const transferMessage = ref("");
  const transferError = ref(false);
  const informationInputRef = ref<HTMLInputElement | null>(null);
  const resetConfirmRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
  const deletionOptionsOpen = ref(false);
  async function deleteInformation(scope: InformationDeletionScope) {
    if (transferBusy.value) return;
    const preserveSettings = scope === "information-only";
    deletionOptionsOpen.value = false;
    const confirmed = await resetConfirmRef.value?.ask({
      title: preserveSettings
        ? zh.value
          ? "仅删除信息，保留设置？"
          : "Delete information and keep settings?"
        : zh.value
          ? "删除所有信息并恢复初始状态？"
          : "Delete all information and reset the app?",
      description: preserveSettings
        ? zh.value
          ? "此操作不可恢复。将删除本地对话、工作区登记、Token 用量统计、归档、本地记忆、头像和个人资料。API Key、供应商、模型及其他应用设置会保留。应用将关闭并自动重启，工作区中的实际项目文件不会被删除。"
          : "This cannot be undone. Conversations, workspace registrations, token history, archives, local memory and your profile will be deleted. API keys, providers, models and app settings are kept. The app will restart. Workspace project files are preserved."
        : zh.value
          ? "此操作不可恢复。将永久清空本地 SQLite 中所有数据，包括对话、工作区登记、Token 用量统计、归档与本地记忆；头像、个人资料、API Key、模型配置、安装插件和其他设置也将清除。建议先导出用户信息。确认后应用会关闭并自动重启，回到首次安装状态。工作区中的实际项目文件不会被删除。"
          : "This cannot be undone. All local SQLite data, conversations, workspace registrations, token history, archives and local memory will be deleted, together with your profile, avatar, API keys, model configuration, installed plugins and settings. Export your information first if needed. The app will close and restart in its first-install state. Project files in your workspaces are preserved.",
      confirmLabel: zh.value ? "永久删除并重置" : "Delete permanently and reset",
      cancelLabel: zh.value ? "取消" : "Cancel",
      tone: "danger",
      confirmOnEnter: false,
    });
    if (!confirmed) return;
    transferBusy.value = true;
    transferError.value = false;
    transferMessage.value = zh.value
      ? "正在退出并清理本地数据，应用将自动重新启动…"
      : "Closing and clearing local data. The app will restart…";
    try {
      await invoke("delete_all_user_information", {
        confirmation: "DELETE_ALL_USER_INFORMATION",
        preserveSettings,
      });
    } catch (cause) {
      transferError.value = true;
      transferMessage.value = String(cause);
      transferBusy.value = false;
    }
  }
  async function exportInformation() {
    transferBusy.value = true;
    transferMessage.value = "";
    transferError.value = false;
    try {
      const path = await save({
        defaultPath: `Anya-user-information-${new Date().toISOString().slice(0, 10)}.json`,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path) return;
      const data = await exportUserInformation(await getAppSettings());
      await writeFile(path, new TextEncoder().encode(JSON.stringify(data, null, 2)));
      transferMessage.value = zh.value
        ? "用户信息已导出，文件包含 API Key，请妥善保存。"
        : "Exported. The file contains API keys; keep it private.";
    } catch (cause) {
      transferError.value = true;
      transferMessage.value = String(cause);
    } finally {
      transferBusy.value = false;
    }
  }
  async function importInformation(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    transferBusy.value = true;
    transferMessage.value = "";
    transferError.value = false;
    try {
      if (file.size > 50 * 1024 * 1024)
        throw new Error("User information file is too large (max 50 MB)");
      const previous = await getAppSettings();
      const data = parseUserInformation(await file.text(), previous);
      const oldStorage = userInformationKeys.map((key) => localStorage.getItem(key));
      const values = informationStorage(data);
      const stageKey = "anya.user-information-import-stage";
      try {
        localStorage.setItem(stageKey, JSON.stringify(values));
      } finally {
        localStorage.removeItem(stageKey);
      }
      const imported = await setAppSettings(data.settings);
      try {
        applyInformationStorage(values);
      } catch (cause) {
        try {
          applyInformationStorage(oldStorage);
        } finally {
          const restored = await setAppSettings({
            ...previous,
            customBackground: previous.customBackground ?? null,
          });
          settingStore.applySettings(restored);
        }
        throw cause;
      }
      settingStore.applySettings(imported);
      void useChatModelStore().refresh();
      onImported(data);
      transferMessage.value = zh.value
        ? "用户信息已导入。硬件加速等启动项需重启后生效。"
        : "Imported. Startup settings such as hardware acceleration require a restart.";
    } catch (cause) {
      transferError.value = true;
      transferMessage.value = String(cause);
    } finally {
      transferBusy.value = false;
    }
  }
  return {
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
  };
}
