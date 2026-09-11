//! Dynamic agent tools registered from a plugin Deno host.

use std::sync::{Arc, Mutex};

use serde_json::Value;

use super::deno::DenoBridge;
use super::manifest::tool_prefix;
use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::registry::ToolRegistry;

#[derive(Clone)]
pub struct PluginToolSpec {
    pub _plugin_id: String,
    pub local_name: String,
    pub description: String,
    pub parameters: Value,
}

struct PluginDynamicTool {
    plugin_id: String,
    full_name: String,
    local_name: String,
    description: String,
    parameters: Value,
    deno: Arc<Mutex<DenoBridge>>,
}

impl Tool for PluginDynamicTool {
    fn name(&self) -> &str {
        &self.full_name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters_schema(&self) -> Value {
        self.parameters.clone()
    }
    fn read_only(&self) -> bool {
        super::computer::is_read_only_tool(&self.local_name)
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        if let Some(result) =
            super::computer::try_execute_tool(&self.plugin_id, &self.local_name, &args)
        {
            return result;
        }
        let mut deno = self
            .deno
            .lock()
            .map_err(|_| ToolError::new("plugin deno lock"))?;
        let result = deno.call_tool(&self.local_name, args)?;
        if let Some(content) = result.get("content").and_then(|v| v.as_str()) {
            return Ok(content.to_string());
        }
        Ok(result.to_string())
    }
}

pub fn register_plugin_tools(
    registry: &ToolRegistry,
    plugin_id: &str,
    tools: &[PluginToolSpec],
    deno: Arc<Mutex<DenoBridge>>,
) {
    unregister_plugin_tools(registry, plugin_id);
    for spec in tools {
        let full_name = format!("{}{}", tool_prefix(plugin_id), spec.local_name);
        registry.register_dynamic(Arc::new(PluginDynamicTool {
            plugin_id: plugin_id.to_string(),
            full_name,
            local_name: spec.local_name.clone(),
            description: spec.description.clone(),
            parameters: spec.parameters.clone(),
            deno: Arc::clone(&deno),
        }));
    }
}

pub fn unregister_plugin_tools(registry: &ToolRegistry, plugin_id: &str) {
    registry.unregister_dynamic_prefix(&tool_prefix(plugin_id));
}
