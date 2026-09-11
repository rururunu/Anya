//! User plugins: workbench UI (same JS heap), Deno host, and OS sidecar.

mod agent;
mod bundle;
mod bundled;
mod capability;
mod computer;
mod contract;
mod deno;
pub mod diagnostics;
mod fs;
mod grant;
mod job;
mod locate;
mod manifest;
mod package;
mod permissions;
mod prompt;
mod pty;
mod runtime;
mod safe;
mod scaffold;
mod sidecar;

pub mod host;
pub mod protocol;

pub use bundle::read_plugin_ui_source;
pub use bundled::ensure_bundled_plugins as bundled_ensure;
#[allow(unused_imports)]
pub use capability::{
    capability_label, is_known_capability, validate_capability_decl, CapabilityDecl,
    KNOWN_CAPABILITY_CATEGORIES,
};
pub use computer::requires_tool_approval;
pub use contract::describe_contract;
#[allow(unused_imports)]
pub use fs::{
    folder_as_picked, import_user_file, PickedFile, PluginFsPickFilter, PluginFsPickOptions,
};
#[allow(unused_imports)]
pub use grant::{get_grant, grant_has, PluginGrant};
pub use host::{
    close_all_plugin_windows, close_plugin_window, destroy_plugin_window_label, open_plugin_window,
};
#[allow(unused_imports)]
pub use manifest::{
    create_plugin, delete_plugin, list_plugin_files, list_plugins, load_manifest, plugin_dir,
    put_plugin_file, read_plugin_file, sanitize_plugin_id, PluginManifest, PluginSummary,
};
pub use package::{
    export_plugin_zip, import_plugin_from_path, is_official_plugin_id, peek_plugin_id,
};
#[allow(unused_imports)]
pub use permissions::{is_known_permission, permission_label, KNOWN_PERMISSIONS};
pub use protocol::handle_plugin_protocol;
#[allow(unused_imports)]
pub use runtime::{
    after_tool, before_tool, on_turn_end, rewrite_user_message, shared_runtime, PluginRuntime,
};
pub use safe::{is_plugin_safe_mode, plugin_safe_mode_reason};
pub use scaffold::ensure_plugin_icon;

pub const PLUGIN_SCHEME: &str = "anya-plugin";
pub const MAX_PLUGIN_FILE_BYTES: usize = 4 * 1024 * 1024;

/// Entry point for the `anya-plugin-host` sidecar binary.
pub fn run_plugin_host() {
    sidecar::run_stdio_host();
}
