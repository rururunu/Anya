//! Agent-facing mirror of the frontend contract tables (SlotRegistry anchors,
//! AssetOverrideRegistry keys). The frontend TS files are the source of truth
//! for actual mounting behavior; keep this in sync when anchors/keys change
//! there (`src/composables/plugins/slotRegistry.ts`, `assetRegistry.ts`) so
//! `manage_plugin describe_contract` doesn't lie to the agent.

use serde::Serialize;

use super::capability::{capability_label, KNOWN_CAPABILITY_CATEGORIES};
use super::manifest::SUPPORTED_API_VERSIONS;
use super::permissions::{permission_label, KNOWN_PERMISSIONS};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorInfo {
    pub id: &'static str,
    pub surfaces: &'static [&'static str],
    pub multiplicity: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetKeyInfo {
    pub key: &'static str,
    pub allowed_kinds: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityInfo {
    pub id: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionInfo {
    pub id: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomePageInfo {
    /// Manifest field: relative path to a `.md`/`.html` file rendered as the
    /// home page's "详情/About" tab. Falls back to `description` when absent.
    pub about_manifest_field: &'static str,
    /// Workbench SDK call for the home page's "设置/Settings" tab: a DOM
    /// mount, exactly like `sidebar.addTab`'s `mount(el)`.
    pub settings_primitive: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginUiInfo {
    pub color_tokens: &'static [&'static str],
    pub note: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconInfo {
    pub file: &'static str,
    pub manifest_field: &'static str,
    pub tab: &'static str,
    pub note: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginLayoutInfo {
    pub user_plugins: &'static str,
    pub not_the_workspace: &'static str,
    pub write_ui: &'static str,
    pub generated_ui: &'static str,
    pub ui_resolve_order: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentServiceInfo {
    pub mount: &'static str,
    pub unmount: &'static str,
    pub send: &'static str,
    pub run: &'static str,
    pub session_id: &'static str,
    pub aliases: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceServiceInfo {
    pub open_in_terminal: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractDescription {
    pub supported_api_versions: &'static [&'static str],
    pub anchors: Vec<AnchorInfo>,
    pub asset_keys: Vec<AssetKeyInfo>,
    /// Where `sidebar.addTab` launchers may appear: nav, header, views.
    /// Pane: `"views"` (alone or with nav) = right review strip. `"nav"` without
    /// `"views"` = center main area (same as 插件). `"header"` without nav/views
    /// = chrome action (`onClick` only, no pane). Exclusive center on enable:
    /// `ctx.workbench.setView`. Plugin settings stay on the home page (see `home`).
    pub tab_chrome: &'static [&'static str],
    pub tab_chrome_note: &'static str,
    /// Frozen agent surface: embed the live conversation into a plugin node.
    pub agent: AgentServiceInfo,
    /// Open the OS terminal at a workspace root (header action plugins).
    pub workspace: WorkspaceServiceInfo,
    /// Every installed plugin gets a home page (icon/name/enable toggle +
    /// 详情/设置 tabs) from the installed-plugins list — not from Anya's chrome.
    pub home: HomePageInfo,
    pub permissions: Vec<PermissionInfo>,
    pub capabilities: Vec<CapabilityInfo>,
    /// How plugins obtain local files. Permission names are not APIs —
    /// this is the only file-pick call surface.
    pub files: FileApiInfo,
    /// Theme tokens and native-control constraints for `mount(el)` UI.
    pub ui: PluginUiInfo,
    /// Shipped plugin mark (list + nav). Not a Lucide name.
    pub icon: IconInfo,
    /// Where plugin files live vs Anya source, and which UI file to edit.
    pub layout: PluginLayoutInfo,
    /// `plugin.json` `"role"`: ui (chrome), service (host/OS), agent (chat tools, no UI).
    pub plugin_roles: &'static [&'static str],
    pub plugin_roles_note: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileApiInfo {
    pub permission: &'static str,
    pub workbench: &'static str,
    pub isolated_window: &'static str,
    pub workspace_host: &'static str,
    pub not_provided: &'static [&'static str],
}

const ANCHORS: &[AnchorInfo] = &[
    AnchorInfo {
        id: "sidebar.tabs",
        surfaces: &["workbench", "peek"],
        multiplicity: "stack",
    },
    AnchorInfo {
        id: "workbench.main",
        surfaces: &["workbench"],
        multiplicity: "exclusive",
    },
    AnchorInfo {
        id: "composer.accessory",
        surfaces: &["workbench"],
        multiplicity: "stack",
    },
    AnchorInfo {
        id: "conversation.materials",
        surfaces: &["workbench"],
        multiplicity: "stack",
    },
];

const ASSET_KEYS: &[AssetKeyInfo] = &[
    AssetKeyInfo {
        key: "mascot.idle",
        allowed_kinds: &["image", "video", "lottie"],
    },
    AssetKeyInfo {
        key: "tray.icon",
        allowed_kinds: &["image"],
    },
    AssetKeyInfo {
        key: "workbench.backdrop",
        allowed_kinds: &["image", "video", "lottie"],
    },
    AssetKeyInfo {
        key: "pet.stage.skin",
        allowed_kinds: &["image", "video", "lottie"],
    },
];

pub fn describe_contract() -> ContractDescription {
    ContractDescription {
        supported_api_versions: SUPPORTED_API_VERSIONS,
        anchors: ANCHORS.to_vec(),
        asset_keys: ASSET_KEYS.to_vec(),
        tab_chrome: &["nav", "header", "views"],
        tab_chrome_note: "Launchers only. Own page = nav without views; put 资料/成品 columns in that page. Custom agent UI = ctx.agent.run (isolated plugin session). Embed Anya's live chat chrome = ctx.agent.mount(el). Do not request new shell anchors — compose workbench.main. views (with or without nav) = right review strip. header without nav/views = chrome action (addTab onClick, no pane). Exclusive center on Enable: ctx.workbench.setView + contributes.view. Never patch Anya src/ or src-tauri for a new plugin.",
        agent: AgentServiceInfo {
            mount: "ctx.agent.mount(el) — Teleport Anya's live MessageList + composer into a plugin node; returns cleanup",
            unmount: "ctx.agent.unmount()",
            send: "ctx.agent.send(text) — inject into the current workbench conversation (same path as the composer)",
            run: "ctx.agent.run(prompt, { sessionId?, cwd? }) — same Rust agent loop on a plugin-owned session (plugin:<pluginId>:…), not the open chat. Omit sessionId to mint one; pass a stable id to keep history. cwd binds tools to that folder without switching the workbench workspace. Read ctx.stores.chat.sessions[sessionId].",
            session_id: "ctx.agent.sessionId() — current workbench session id (for mount/send, not run)",
            aliases: "ctx.conversation.mount/unmount/send/run/sessionId are aliases. conversation.addMaterials is compat-only; do not use it for new plugins.",
        },
        workspace: WorkspaceServiceInfo {
            open_in_terminal: "ctx.workspace.openInTerminal(id?) — same as workbench「在终端中打开」: Windows wt.exe -d <cwd>, else cmd. Omit id to use the current conversation workspace, then get_current_workspace.",
        },
        home: HomePageInfo {
            about_manifest_field: "about",
            settings_primitive: "ctx.home.setSettingsView(mount)",
        },
        permissions: KNOWN_PERMISSIONS
            .iter()
            .map(|id| PermissionInfo { id, label: permission_label(id) })
            .collect(),
        capabilities: KNOWN_CAPABILITY_CATEGORIES
            .iter()
            .map(|id| CapabilityInfo { id, label: capability_label(id) })
            .collect(),
        files: FileApiInfo {
            permission: "fs.pick",
            workbench: "ctx.fs.pick({ multiple?, directory?, filters?: [{ name, extensions }] }) → Promise<{ path, name, url }[] | null>",
            isolated_window: "AnyaPlugin.pick(same options) → Promise<{ path, name, url }[] | null>",
            workspace_host: "fs.workspace grants Deno --allow-read/--allow-write on the current workspace in host/main.ts only — not a ctx method. Deno.readTextFile / writeTextFile against that root.",
            not_provided: &[
                "ctx.fs.read / ctx.fs.write / ctx.fs.stat",
                "native filesystem path from <input type=file>",
                "hidden file SDK in Anya src/ — do not grep it",
            ],
        },
        ui: PluginUiInfo {
            color_tokens: &[
                "--peek-text",
                "--peek-muted",
                "--peek-surface",
                "--peek-bg",
                "--peek-border",
                "--peek-accent",
            ],
            note: "Anya keeps color-scheme: only light (WebView2). Native <select> option lists are OS-white; `color:inherit` / `background:transparent` makes options unreadable. Use a row of <button>s for choices. Always append built nodes to mount(el); teardown must not call home.clearSettingsView.",
        },
        icon: IconInfo {
            file: "ui/icon.svg — required in create files. Distinctive 48×48 SVG (reads at 16px). Do not leave the letter-circle scaffold. Do not use Lucide names like sparkles as the only mark.",
            manifest_field: "plugin.json \"icon\": \"ui/icon.svg\"",
            tab: "addTab.icon may be omitted (host uses the manifest file) or `anya-plugin://localhost/${ctx.pluginId}/ui/icon.svg`",
            note: "Plugins list, home, and nav all show this file. Puzzle/terminal glyphs are leftovers, not a finished plugin.",
        },
        layout: PluginLayoutInfo {
            user_plugins: "%APPDATA%/Anya/plugins/<id>/ (debug build: Anya Debug/plugins/<id>/). Read/write only via manage_plugin.",
            not_the_workspace: "The open workspace (src/, src-tauri/) is Anya product source, not plugin files. Do not grep loaders for surfaces or UI entry.",
            write_ui: "ui/src/activate.js (or .ts). Never edit ui/.anya/activate.js.",
            generated_ui: "ui/.anya/activate.js — Enable/reload bundle the workbench actually loads.",
            ui_resolve_order: &[
                "ui/src/activate.ts",
                "ui/src/activate.js",
                "ui/src/main.ts",
                "ui/src/main.js",
                "plugin.json ui.entry",
            ],
        },
        plugin_roles: &["ui", "service", "agent"],
        plugin_roles_note: "plugin.json \"role\". ui = workbench chrome (activate.js + ui.workbench). service = host/OS capability (CLI, daemon); UI optional. agent = chat tools only — no sidebar/window; host describe() tools; after Enable the agent is told to call them. Official: terminal=ui, computer-use=agent.",
    }
}

#[cfg(test)]
mod tests {
    use super::describe_contract;

    #[test]
    fn describe_contract_documents_fs_pick() {
        let json = serde_json::to_value(describe_contract()).unwrap();
        assert_eq!(json["files"]["permission"], "fs.pick");
        let workbench = json["files"]["workbench"].as_str().unwrap();
        assert!(workbench.contains("ctx.fs.pick"));
        let missing = json["files"]["notProvided"].as_array().unwrap();
        assert!(missing
            .iter()
            .any(|v| v.as_str().unwrap().contains("ctx.fs.read")));
    }

    #[test]
    fn describe_contract_documents_ui_tokens() {
        let json = serde_json::to_value(describe_contract()).unwrap();
        let tokens = json["ui"]["colorTokens"].as_array().unwrap();
        assert!(tokens.iter().any(|v| v.as_str() == Some("--peek-text")));
        let note = json["ui"]["note"].as_str().unwrap();
        assert!(note.contains("select"));
        assert!(note.contains("append"));
    }

    #[test]
    fn describe_contract_documents_layout() {
        let json = serde_json::to_value(describe_contract()).unwrap();
        assert!(json["layout"]["writeUi"]
            .as_str()
            .unwrap()
            .contains("ui/src/activate"));
        assert!(json["layout"]["generatedUi"]
            .as_str()
            .unwrap()
            .contains("ui/.anya"));
        let order = json["layout"]["uiResolveOrder"].as_array().unwrap();
        assert!(order
            .iter()
            .any(|v| v.as_str() == Some("ui/src/activate.js")));
        assert!(json["layout"]["notTheWorkspace"]
            .as_str()
            .unwrap()
            .contains("src-tauri"));
        let note = json["tabChromeNote"].as_str().unwrap();
        assert!(note.contains("nav without views"));
        assert!(note.contains("workbench.setView"));
        assert!(note.contains("ctx.agent.mount"));
        assert!(note.contains("compose workbench.main"));
        assert!(note.contains("header without nav/views"));
        assert!(note.contains("onClick"));
        let workspace = json["workspace"]["openInTerminal"].as_str().unwrap();
        assert!(workspace.contains("ctx.workspace.openInTerminal"));
        let agent = &json["agent"];
        assert!(agent["mount"].as_str().unwrap().contains("ctx.agent.mount"));
        assert!(agent["send"].as_str().unwrap().contains("ctx.agent.send"));
        assert!(agent["run"].as_str().unwrap().contains("ctx.agent.run"));
        assert!(agent["sessionId"].as_str().unwrap().contains("sessionId"));
        assert!(agent["aliases"]
            .as_str()
            .unwrap()
            .contains("conversation.mount"));
        let ids: Vec<&str> = json["anchors"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v["id"].as_str())
            .collect();
        assert!(ids.contains(&"conversation.materials"));
        let perms: Vec<&str> = json["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v["id"].as_str())
            .collect();
        assert!(perms.contains(&"computer"));
        let icon = &json["icon"];
        assert!(icon["file"].as_str().unwrap().contains("ui/icon.svg"));
        assert!(icon["manifestField"].as_str().unwrap().contains("icon"));
        assert!(icon["note"].as_str().unwrap().contains("Puzzle"));
        let roles: Vec<&str> = json["pluginRoles"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(roles, vec!["ui", "service", "agent"]);
        assert!(json["pluginRolesNote"]
            .as_str()
            .unwrap()
            .contains("computer-use=agent"));
    }
}
