use std::sync::Arc;

use serde_json::{json, Value};

use crate::runtime::browser::BrowserProvider;
use crate::runtime::tool::{Tool, ToolContext, ToolError};

pub struct BrowserTool {
    provider: Arc<dyn BrowserProvider>,
}

impl BrowserTool {
    pub fn new(provider: Arc<dyn BrowserProvider>) -> Self {
        Self { provider }
    }
}

impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser_read"
    }
    fn description(&self) -> &str {
        "Read a public web page through the configured browser provider and return clean Markdown. Use after web_search when full source content is needed."
    }
    fn parameters_schema(&self) -> Value {
        json!({ "type": "object", "properties": { "url": { "type": "string" } }, "required": ["url"] })
    }
    fn read_only(&self) -> bool {
        true
    }
    /// Hidden when web search is disabled; browser_read is a companion of web_search.
    fn available(&self) -> bool {
        crate::runtime::search::shared_search_runtime().is_available()
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let provider = Arc::clone(&self.provider);
        let url = ["url", "uri", "href", "link"]
            .iter()
            .find_map(|key| {
                args.get(*key)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            })
            .unwrap_or("")
            .to_string();
        if url.is_empty() {
            return Err(ToolError::new(
                "url is required (parameter `url`; aliases: uri, href, link)",
            ));
        }
        crate::runtime::isolated::run_isolated_or(
            move || {
                let document = provider.read(&url)?;
                serde_json::to_string_pretty(&json!({
                    "provider": provider.id(),
                    "document": document,
                }))
                .map_err(|error| ToolError::new(error.to_string()))
            },
            |panic| Err(ToolError::new(format!("browser_read panicked: {panic}"))),
        )
    }
}
