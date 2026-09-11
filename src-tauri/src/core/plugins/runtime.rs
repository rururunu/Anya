//! Enable / disable / host RPC / Deno lifecycle.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use super::agent::{register_plugin_tools, unregister_plugin_tools, PluginToolSpec};
use super::bundle;
use super::bundled::ensure_bundled_plugins;
use super::deno::DenoBridge;
use super::diagnostics::clear_plugin_errors;
use super::grant::{
    clear_grant, get_grant, grant_has, issue_token, revoke_token, save_grant, PluginGrant,
};
use super::manifest::{load_manifest, plugin_dir};
use super::prompt;
use super::pty::HostEventFn;
use super::safe;
use super::sidecar::{HostEngine, SidecarClient};
use crate::core::tools::error::ToolError;
use crate::core::tools::registry::ToolRegistry;

struct PluginSession {
    sidecar: Option<SidecarClient>,
    host: Option<HostEngine>,
    deno: Option<Arc<Mutex<DenoBridge>>>,
    hooks: Vec<String>,
    agent_tools: Vec<String>,
}

pub struct PluginRuntime {
    registry: Mutex<Option<Arc<ToolRegistry>>>,
    sessions: Mutex<HashMap<String, PluginSession>>,
    app: Mutex<Option<AppHandle>>,
}

impl PluginRuntime {
    fn new() -> Self {
        Self {
            registry: Mutex::new(None),
            sessions: Mutex::new(HashMap::new()),
            app: Mutex::new(None),
        }
    }

    pub fn attach(&self, app: AppHandle, registry: Arc<ToolRegistry>) {
        if let Ok(mut slot) = self.app.lock() {
            *slot = Some(app);
        }
        if let Ok(mut slot) = self.registry.lock() {
            *slot = Some(registry);
        }
        let _ = ensure_bundled_plugins();
        if safe::is_plugin_safe_mode() {
            tracing::warn!(
                reason = %safe::plugin_safe_mode_reason(),
                "plugin safe mode: skipping all plugins"
            );
            return;
        }
        let restore = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.restore_enabled();
        }));
        if restore.is_err() {
            let _ = safe::enter_plugin_safe_mode("panic while restoring plugins");
            self.stop_all_sessions();
        }
    }

    fn registry(&self) -> Option<Arc<ToolRegistry>> {
        self.registry.lock().ok().and_then(|g| g.clone())
    }

    fn emit_host_event(&self) -> HostEventFn {
        let app = self.app.lock().ok().and_then(|g| g.clone());
        Arc::new(move |value: Value| {
            if let Some(app) = &app {
                let _ = app.emit("plugin-host-event", value);
            }
        })
    }

    pub fn restore_enabled(&self) {
        let Ok(list) = super::list_plugins() else {
            return;
        };
        for item in list {
            if item.enabled {
                if let Err(err) = self.start_session(&item.id) {
                    tracing::warn!(plugin = %item.id, %err, "failed to restore plugin");
                }
            }
        }
    }

    pub fn enable(&self, id: &str, permissions: Vec<String>) -> Result<(), ToolError> {
        if safe::is_plugin_safe_mode() {
            return Err(ToolError::new(
                "plugin safe mode is on; clear it in Plugins before enabling",
            ));
        }
        let manifest = load_manifest(id)?;
        for perm in &permissions {
            if !manifest.permissions.iter().any(|p| p == perm) {
                return Err(ToolError::new(format!(
                    "permission `{perm}` is not declared by plugin `{id}`"
                )));
            }
        }
        bundle::bundle_plugin_ui(id, true)?;
        save_grant(
            id,
            PluginGrant {
                enabled: true,
                permissions,
                granted_at: now_secs(),
            },
        )?;
        clear_plugin_errors(Some(id));
        self.stop_session(id);
        self.start_session(id)?;
        self.notify_workbench_ui(id);
        Ok(())
    }

    pub fn disable(&self, id: &str) -> Result<(), ToolError> {
        let mut grant = get_grant(id);
        grant.enabled = false;
        save_grant(id, grant)?;
        self.close_window(id);
        self.stop_session(id);
        clear_plugin_errors(Some(id));
        self.notify_workbench_ui(id);
        Ok(())
    }

    pub fn uninstall_cleanup(&self, id: &str) -> Result<(), ToolError> {
        self.close_window(id);
        self.stop_session(id);
        clear_plugin_errors(Some(id));
        clear_grant(id)
    }

    fn close_window(&self, id: &str) {
        if let Some(app) = self.app.lock().ok().and_then(|g| g.clone()) {
            let _ = super::host::close_plugin_window(&app, id);
        }
    }

    pub fn reload(&self, id: &str) -> Result<(), ToolError> {
        if safe::is_plugin_safe_mode() {
            return Err(ToolError::new("plugin safe mode: reload disabled"));
        }
        if !get_grant(id).enabled {
            return Err(ToolError::new(format!("plugin `{id}` is not enabled")));
        }
        clear_plugin_errors(Some(id));
        self.stop_session(id);
        bundle::bundle_plugin_ui(id, true)?;
        self.start_session(id)?;
        self.notify_workbench_ui(id);
        Ok(())
    }

    fn notify_workbench_ui(&self, id: &str) {
        let Some(app) = self.app.lock().ok().and_then(|g| g.clone()) else {
            return;
        };
        let payload = json!({ "pluginId": id });
        let _ = app.emit("plugin-reload-request", &payload);
    }

    pub fn exit_safe_mode_and_restore(&self) -> Result<(), ToolError> {
        safe::clear_plugin_safe_mode()?;
        if safe::is_plugin_safe_mode() {
            return Err(ToolError::new(
                "safe mode is still on (ANYA_PLUGIN_SAFE_MODE / ANYA_SAFE_MODE env)",
            ));
        }
        self.restore_enabled();
        Ok(())
    }

    fn start_session(&self, id: &str) -> Result<(), ToolError> {
        if safe::is_plugin_safe_mode() {
            return Err(ToolError::new("plugin safe mode: hosts are not started"));
        }
        let manifest = load_manifest(id)?;
        if !get_grant(id).enabled {
            return Err(ToolError::new(format!("plugin `{id}` is not enabled")));
        }
        bundle::bundle_plugin_ui(id, false)?;
        let issued = issue_token(id)?;
        let events = self.emit_host_event();
        let mut sidecar = SidecarClient::spawn(id, &issued.token, Arc::clone(&events)).ok();
        if let Some(client) = sidecar.as_mut() {
            if client
                .request(
                    "hello",
                    json!({
                        "pluginId": id,
                        "token": issued.token,
                        "hostPid": issued.host_pid,
                    }),
                )
                .is_err()
            {
                sidecar = None;
            }
        }
        let host = if sidecar.is_none() {
            let mut engine = HostEngine::new(id.to_string(), events);
            engine.dispatch(
                "hello",
                &json!({
                    "pluginId": id,
                    "token": issued.token,
                    "hostPid": issued.host_pid,
                }),
            )?;
            Some(engine)
        } else {
            None
        };

        let mut deno = None;
        let mut hooks = Vec::new();
        let mut tools: Vec<PluginToolSpec> = Vec::new();
        if grant_has(id, "agent.tools")
            || grant_has(id, "agent.hooks")
            || manifest.host.entry.is_some()
        {
            if let Some(entry) = manifest
                .host
                .entry
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                match DenoBridge::spawn(id, &plugin_dir(id)?, entry, None) {
                    Ok(mut bridge) => {
                        if let Ok(desc) = bridge.describe() {
                            hooks = desc
                                .get("hooks")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str().map(str::to_string))
                                        .collect()
                                })
                                .unwrap_or_default();
                            tools = parse_tool_specs(id, &desc);
                        }
                        deno = Some(Arc::new(Mutex::new(bridge)));
                    }
                    Err(err) => {
                        tracing::warn!(plugin = %id, %err, "plugin deno host not started");
                    }
                }
            }
        }

        let agent_tools: Vec<String> = if grant_has(id, "agent.tools") {
            tools.iter().map(|spec| spec.local_name.clone()).collect()
        } else {
            Vec::new()
        };

        if let Some(registry) = self.registry() {
            unregister_plugin_tools(registry.as_ref(), id);
            if grant_has(id, "agent.tools") {
                if let Some(bridge) = &deno {
                    register_plugin_tools(registry.as_ref(), id, &tools, Arc::clone(bridge));
                }
            }
        }

        let session = PluginSession {
            sidecar,
            host,
            deno,
            hooks,
            agent_tools,
        };
        if let Ok(mut map) = self.sessions.lock() {
            map.insert(id.to_string(), session);
        }
        Ok(())
    }

    fn stop_all_sessions(&self) {
        let ids: Vec<String> = self
            .sessions
            .lock()
            .ok()
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default();
        for id in ids {
            self.stop_session(&id);
        }
    }

    fn stop_session(&self, id: &str) {
        if let Some(registry) = self.registry() {
            unregister_plugin_tools(registry.as_ref(), id);
        }
        revoke_token(id);
        if let Ok(mut map) = self.sessions.lock() {
            if let Some(mut session) = map.remove(id) {
                if let Some(mut sidecar) = session.sidecar.take() {
                    let _ = sidecar.request("shutdown", json!({}));
                    sidecar.kill();
                }
                if let Some(mut host) = session.host.take() {
                    host.shutdown();
                }
                if let Some(deno) = session.deno.take() {
                    if let Ok(mut bridge) = deno.lock() {
                        bridge.kill();
                    }
                }
            }
        }
    }

    pub fn host_rpc(&self, id: &str, method: &str, params: Value) -> Result<Value, ToolError> {
        if safe::is_plugin_safe_mode() {
            return Err(ToolError::new("plugin safe mode: host RPC disabled"));
        }
        if !get_grant(id).enabled {
            return Err(ToolError::new(format!("plugin `{id}` is not enabled")));
        }
        if method.starts_with("pty.") && !grant_has(id, "pty") {
            return Err(ToolError::new("plugin is not granted pty"));
        }
        if method.starts_with("computer.") {
            return super::computer::dispatch_rpc(id, method, &params);
        }
        let mut map = self
            .sessions
            .lock()
            .map_err(|_| ToolError::new("plugin runtime lock"))?;
        let session = map
            .get_mut(id)
            .ok_or_else(|| ToolError::new(format!("plugin `{id}` is not running")))?;
        if method.starts_with("pty.") {
            if let Some(sidecar) = session.sidecar.as_mut() {
                return sidecar.request(method, params);
            }
            if let Some(host) = session.host.as_mut() {
                return host.dispatch(method, &params);
            }
            return Err(ToolError::new("plugin host unavailable"));
        }
        if let Some(deno) = session.deno.as_ref().map(Arc::clone) {
            drop(map);
            let mut bridge = deno.lock().map_err(|_| ToolError::new("deno lock"))?;
            return bridge.call_host(method, params);
        }
        if let Some(sidecar) = session.sidecar.as_mut() {
            return sidecar.request(method, params);
        }
        if let Some(host) = session.host.as_mut() {
            return host.dispatch(method, &params);
        }
        Err(ToolError::new("plugin host unavailable"))
    }

    pub fn call_hook(&self, hook: &str, payload: Value) -> Value {
        let Ok(map) = self.sessions.lock() else {
            return payload;
        };
        let mut current = payload;
        for (id, session) in map.iter() {
            if !grant_has(id, "agent.hooks") {
                continue;
            }
            if !session.hooks.iter().any(|h| h == hook)
                && !session.hooks.is_empty()
                && hook != "on_user_message"
                && hook != "on_before_tool"
                && hook != "on_after_tool"
                && hook != "on_turn_end"
            {
                continue;
            }
            let Some(deno) = &session.deno else {
                continue;
            };
            let deno = Arc::clone(deno);
            let hook_name = hook.to_string();
            let send_payload = current.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let result = deno
                    .lock()
                    .map_err(|_| ToolError::new("deno lock"))
                    .and_then(|mut bridge| bridge.call_hook(&hook_name, send_payload));
                let _ = tx.send(result);
            });
            match rx.recv_timeout(std::time::Duration::from_secs(2)) {
                Ok(Ok(value)) => {
                    current = value;
                }
                Ok(Err(err)) => {
                    tracing::warn!(plugin = %id, hook, %err, "plugin hook failed");
                }
                Err(_) => {
                    tracing::warn!(plugin = %id, hook, "plugin hook timed out");
                }
            }
        }
        current
    }

    pub fn plugin_prompt_suffix(&self) -> Option<String> {
        if safe::is_plugin_safe_mode() {
            return None;
        }
        let Ok(list) = super::list_plugins() else {
            return None;
        };
        let sessions = self.sessions.lock().ok();
        let mut blocks = Vec::new();
        for item in list {
            if !item.enabled || !grant_has(&item.id, "agent.tools") {
                continue;
            }
            let Ok(manifest) = load_manifest(&item.id) else {
                continue;
            };
            if !manifest.contributes.agent.tools {
                continue;
            }
            let local_tools = sessions
                .as_ref()
                .and_then(|map| map.get(&item.id))
                .map(|session| session.agent_tools.clone())
                .unwrap_or_default();
            let extra = if grant_has(&item.id, "agent.prompt") {
                prompt::extra_from_manifest(
                    &item.id,
                    &manifest.contributes.agent.prompt,
                    &manifest.contributes.agent.skills,
                )
            } else {
                String::new()
            };
            blocks.push(prompt::format_block(
                &item.id,
                &item.name,
                &item.description,
                &local_tools,
                &extra,
            ));
        }
        prompt::assemble(&blocks)
    }
}

fn parse_tool_specs(plugin_id: &str, desc: &Value) -> Vec<PluginToolSpec> {
    let Some(arr) = desc.get("tools").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|tool| {
            let name = tool.get("name")?.as_str()?.trim();
            if name.is_empty() {
                return None;
            }
            Some(PluginToolSpec {
                _plugin_id: plugin_id.to_string(),
                local_name: name.to_string(),
                description: tool
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Plugin tool")
                    .to_string(),
                parameters: tool
                    .get("parameters")
                    .cloned()
                    .unwrap_or_else(|| json!({ "type": "object", "properties": {} })),
            })
        })
        .collect()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn shared_runtime() -> &'static PluginRuntime {
    static RUNTIME: OnceLock<PluginRuntime> = OnceLock::new();
    RUNTIME.get_or_init(PluginRuntime::new)
}

pub fn rewrite_user_message(content: &str) -> String {
    let payload = json!({ "content": content });
    let result = shared_runtime().call_hook("on_user_message", payload);
    result
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or(content)
        .to_string()
}

pub fn before_tool(tool_name: &str, args: Value) -> Result<Value, ToolError> {
    let result = shared_runtime().call_hook(
        "on_before_tool",
        json!({ "tool": tool_name, "args": args, "allow": true }),
    );
    if result.get("allow").and_then(|v| v.as_bool()) == Some(false) {
        let reason = result
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("plugin hook denied this tool");
        return Err(ToolError::new(reason));
    }
    Ok(result.get("args").cloned().unwrap_or(args))
}

pub fn after_tool(tool_name: &str, success: bool, result: &str) {
    let _ = shared_runtime().call_hook(
        "on_after_tool",
        json!({
            "tool": tool_name,
            "success": success,
            "result": result.chars().take(2000).collect::<String>(),
        }),
    );
}

pub fn on_turn_end(session_id: &str) {
    let _ = shared_runtime().call_hook("on_turn_end", json!({ "sessionId": session_id }));
}
