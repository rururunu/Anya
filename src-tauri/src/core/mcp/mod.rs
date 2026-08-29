//! Minimal MCP stdio JSON-RPC client (tools/list + tools/call).

mod command;
mod manager;
mod process;
mod remote_auth;
mod runtime;

pub use remote_auth::{
    clear_saved_credentials, init_mcp_remote_config_dir, normalize_mcp_servers, uses_mcp_remote,
    McpServerRuntimeStatus,
};
pub use manager::shared_mcp_manager;
pub use runtime::{runtime_support, McpRuntimeSupport};

#[cfg(test)]
mod tests {
    use crate::models::settings::McpServerConfig;
    use super::command::build_mcp_command;
    use super::runtime::{file_exists, find_node_exe, find_npm_js_cli};

    #[test]
    fn finds_node_and_npx_cli() {
        let node = find_node_exe().expect("node.exe should be discoverable");
        assert!(file_exists(&node), "{node:?}");
        let cli = find_npm_js_cli("npx-cli.js").expect("npx-cli.js next to node");
        assert!(file_exists(&cli), "{cli:?}");
    }

    #[test]
    fn builds_npx_through_node_cli() {
        let config = McpServerConfig {
            id: "test".into(),
            command: "npx".into(),
            args: vec!["--version".into()],
            enabled: true,
            ..Default::default()
        };
        let (mut cmd, summary) = build_mcp_command(&config).expect("build npx command");
        assert!(
            summary.to_ascii_lowercase().contains("npx-cli.js")
                || summary.to_ascii_lowercase().contains("npx.cmd"),
            "unexpected launcher: {summary}"
        );
        let output = cmd.output().expect("spawn npx --version");
        assert!(
            output.status.success(),
            "npx --version failed: status={:?} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
