import { computed, reactive } from "vue";
import type { ConfirmDialogOptions } from "@/components/ui/confirm-dialog";
import { displayPluginIcon } from "@/composables/plugins/pluginIcon";
import { afterPluginDisabled, syncEnabledPluginUi } from "@/composables/plugins/sdk";
import {
  closeAllUserPluginWindows,
  closeUserPluginWindow,
  deleteUserPlugin,
  disableUserPlugin,
  enableUserPlugin,
  exportUserPlugin,
  importUserPlugin,
  openUserPlugin,
  reloadUserPlugin,
  type UserPluginSummary,
} from "@/services/plugins/ipc";
import { usePluginsStore } from "@/stores/plugins";
import { useSettingStore } from "@/stores/setting";
import { open, save } from "@tauri-apps/plugin-dialog";

type AskConfirm = (options: ConfirmDialogOptions) => Promise<boolean | undefined>;

/** What the plugin is for: workbench chrome, a host/OS service, or chat-agent tools. */
export function pluginRole(plugin: UserPluginSummary): "ui" | "service" | "agent" {
  const role = plugin.role?.trim();
  if (role === "ui" || role === "service" || role === "agent") return role;
  const contributes = plugin.contributes || {};
  const chrome = Boolean(
    contributes.sidebar || contributes.view || contributes.composer || contributes.window,
  );
  if (contributes.agent?.tools && !chrome) return "agent";
  if (chrome) return "ui";
  return "service";
}

/** Plugins that can drive the OS desktop need an explicit enable consent. */
export function requiresComputerConsent(plugin: UserPluginSummary): boolean {
  return plugin.id === "computer-use" || plugin.permissions.includes("computer");
}

/** OpenCLI can control the user's logged-in Chrome via Browser Bridge. */
export function requiresOpenCliConsent(plugin: UserPluginSummary): boolean {
  return plugin.id === "opencli" || plugin.permissions.includes("opencli");
}

/**
 * Enable/disable/reload/uninstall + permission-grant selection for a user
 * plugin, shared between the installed-plugins grid card and its home page
 * so the two surfaces never drift.
 */
export function usePluginActions() {
  const pluginsStore = usePluginsStore();
  const settingStore = useSettingStore();
  const pending = reactive<Record<string, string[]>>({});

  const copy = computed(() =>
    settingStore.language === "zh-CN"
      ? {
          noDesc: "无简介",
          perms: "声明的权限",
          grant: "启用时授权",
          enable: "启用",
          disable: "停用",
          reload: "重载",
          window: "窗口",
          closeWindow: "关闭窗口",
          remove: "卸载",
          more: "更多",
          importZip: "导入",
          importFolder: "导入文件夹",
          exportPack: "导出",
          overwrite: "已有同名插件。覆盖安装？（不会自动启用）",
          apiUnsupported: "此插件的契约版本高于/不同于当前支持版本，行为可能异常",
          typeUi: "界面",
          typeService: "功能",
          typeAgent: "Agent",
          computerConsentTitle: "启用电脑操控？",
          computerConsentDesc:
            "启用后，聊天里的 Agent 可在本机查看窗口、点击、输入、滚动，并操控已打开的应用与浏览器（后台优先）。请只在你本人控制的电脑上启用；可随时停用。敏感操作仍会走工具审批，HUD「结束」可立即停止。",
          computerConsentConfirm: "我了解，启用",
          computerConsentCancel: "取消",
          opencliConsentTitle: "启用 OpenCLI？",
          opencliConsentDesc:
            "启用后，Agent 可通过本机 OpenCLI 调用站点适配器，并经 Browser Bridge 操控你已登录的 Chrome。请先安装 opencli 与扩展，并确认 doctor 通过。请只在本人控制的电脑上启用；可随时停用。",
          opencliConsentConfirm: "我了解，启用",
          opencliConsentCancel: "取消",
        }
      : {
          noDesc: "No description",
          perms: "Declared permissions",
          grant: "Grant on enable",
          enable: "Enable",
          disable: "Disable",
          reload: "Reload",
          window: "Window",
          closeWindow: "Close window",
          remove: "Uninstall",
          more: "More",
          importZip: "Import",
          importFolder: "Import folder",
          exportPack: "Export",
          overwrite: "A plugin with this id exists. Replace it? It will not be enabled.",
          apiUnsupported:
            "This plugin targets a contract version Anya doesn't fully support; behavior may be off.",
          typeUi: "UI",
          typeService: "Service",
          typeAgent: "Agent",
          computerConsentTitle: "Enable Computer Use?",
          computerConsentDesc:
            "Once enabled, the chat agent can view windows, click, type, scroll, and control open apps and browsers on this PC (background-preferred). Only enable on a machine you control. You can disable anytime. Sensitive actions still go through tool approval; HUD Stop ends the session immediately.",
          computerConsentConfirm: "I understand — Enable",
          computerConsentCancel: "Cancel",
          opencliConsentTitle: "Enable OpenCLI?",
          opencliConsentDesc:
            "Once enabled, the agent can run OpenCLI site adapters and drive your logged-in Chrome via Browser Bridge. Install opencli + the extension and confirm doctor is green first. Only enable on a machine you control; you can disable anytime.",
          opencliConsentConfirm: "I understand — Enable",
          opencliConsentCancel: "Cancel",
        },
  );

  function ensurePending(plugin: UserPluginSummary) {
    if (!pending[plugin.id]) pending[plugin.id] = [...plugin.permissions];
  }

  function iconSrc(plugin: UserPluginSummary): string | undefined {
    const tab = pluginsStore.sidebarTabs.find((entry) => entry.pluginId === plugin.id);
    return displayPluginIcon(plugin, tab?.icon);
  }

  function typeLabel(plugin: UserPluginSummary): string {
    switch (pluginRole(plugin)) {
      case "agent":
        return copy.value.typeAgent;
      case "service":
        return copy.value.typeService;
      default:
        return copy.value.typeUi;
    }
  }

  function computerConsentOptions(): ConfirmDialogOptions {
    return {
      title: copy.value.computerConsentTitle,
      description: copy.value.computerConsentDesc,
      confirmLabel: copy.value.computerConsentConfirm,
      cancelLabel: copy.value.computerConsentCancel,
      tone: "danger",
    };
  }

  function opencliConsentOptions(): ConfirmDialogOptions {
    return {
      title: copy.value.opencliConsentTitle,
      description: copy.value.opencliConsentDesc,
      confirmLabel: copy.value.opencliConsentConfirm,
      cancelLabel: copy.value.opencliConsentCancel,
      tone: "danger",
    };
  }

  async function enable(plugin: UserPluginSummary, askConfirm?: AskConfirm) {
    if (requiresComputerConsent(plugin)) {
      const confirmed = askConfirm
        ? await askConfirm(computerConsentOptions())
        : window.confirm(`${copy.value.computerConsentTitle}\n\n${copy.value.computerConsentDesc}`);
      if (!confirmed) return false;
    } else if (requiresOpenCliConsent(plugin)) {
      const confirmed = askConfirm
        ? await askConfirm(opencliConsentOptions())
        : window.confirm(`${copy.value.opencliConsentTitle}\n\n${copy.value.opencliConsentDesc}`);
      if (!confirmed) return false;
    }
    const selected = pending[plugin.id]?.length ? pending[plugin.id] : plugin.permissions;
    await enableUserPlugin(plugin.id, selected);
    await syncEnabledPluginUi();
    await pluginsStore.refresh();
    return true;
  }

  async function disable(id: string) {
    await disableUserPlugin(id);
    await afterPluginDisabled(id);
  }

  async function toggle(plugin: UserPluginSummary, askConfirm?: AskConfirm) {
    if (plugin.enabled) {
      await disable(plugin.id);
      return true;
    }
    return enable(plugin, askConfirm);
  }

  async function reload(id: string) {
    await reloadUserPlugin(id);
    await syncEnabledPluginUi();
  }

  /** Disable the occupant of an exclusive slot/asset, then remount the rejected plugin. */
  async function takeOver(ownerPluginId: string, rejectedPluginId: string) {
    try {
      await disable(ownerPluginId);
      await reload(rejectedPluginId);
      await pluginsStore.refresh();
    } catch (error) {
      window.alert(String(error));
    }
  }

  async function openWindow(id: string) {
    await openUserPlugin(id);
  }

  async function closeWindow(id: string) {
    await closeUserPluginWindow(id);
  }

  async function closeAllWindows() {
    await closeAllUserPluginWindows();
  }

  async function importFrom(path: string) {
    try {
      try {
        await importUserPlugin(path, false);
      } catch (error) {
        const message = String(error);
        if (message.includes("already exists") && window.confirm(copy.value.overwrite)) {
          await importUserPlugin(path, true);
        } else {
          throw error;
        }
      }
      await pluginsStore.refresh();
    } catch (error) {
      window.alert(String(error));
    }
  }

  async function importZip() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Anya plugin", extensions: ["zip"] }],
    });
    if (!selected || Array.isArray(selected)) return;
    await importFrom(selected);
  }

  async function importFolder() {
    const selected = await open({ multiple: false, directory: true });
    if (!selected || Array.isArray(selected)) return;
    await importFrom(selected);
  }

  async function exportPack(plugin: UserPluginSummary) {
    const dest = await save({
      defaultPath: `${plugin.id}-${plugin.version}.anya-plugin.zip`,
      filters: [{ name: "Anya plugin", extensions: ["zip"] }],
    });
    if (!dest) return;
    await exportUserPlugin(plugin.id, dest);
  }

  async function remove(id: string) {
    await deleteUserPlugin(id);
    await afterPluginDisabled(id);
  }

  return {
    pending,
    copy,
    ensurePending,
    iconSrc,
    typeLabel,
    enable,
    disable,
    toggle,
    reload,
    takeOver,
    openWindow,
    closeWindow,
    closeAllWindows,
    importZip,
    importFolder,
    exportPack,
    remove,
    computerConsentOptions,
  };
}
