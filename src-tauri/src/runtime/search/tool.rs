use std::sync::Arc;

use chrono::Local;
use serde_json::{json, Value};

use crate::runtime::search::{SearchQuery, SearchRuntime};
use crate::runtime::tool::{Tool, ToolContext, ToolError};

pub struct SearchTool {
    runtime: Arc<SearchRuntime>,
}

impl SearchTool {
    pub fn new(runtime: Arc<SearchRuntime>) -> Self {
        Self { runtime }
    }
}

fn today_local() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn first_string(args: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        args.get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

pub(crate) fn search_query_from_args(args: &Value) -> String {
    first_string(
        args,
        &[
            "query",
            "q",
            "search",
            "search_query",
            "keyword",
            "keywords",
        ],
    )
    .unwrap_or_default()
}

fn max_results_from_args(args: &Value) -> usize {
    ["max_results", "num", "n", "limit", "count"]
        .iter()
        .find_map(|key| args.get(*key).and_then(Value::as_u64))
        .unwrap_or(8) as usize
}

impl Tool for SearchTool {
    fn name(&self) -> &str {
        "web_search"
    }
    fn description(&self) -> &str {
        "Search the web and return structured result metadata. Prefer including today's date in time-sensitive queries. Use before browser_read when the user asks for current, recent, or externally verifiable information. Skip when repository evidence or stable knowledge already answers the question."
    }
    fn parameters_schema(&self) -> Value {
        let today = today_local();
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": format!(
                        "Search query. Today is {today} (local). For current events, news, prices, scores, or anything time-sensitive, include this date or year in the query."
                    )
                },
                "max_results": { "type": "integer", "minimum": 1, "maximum": 20, "default": 8 },
                "language": { "type": "string" },
                "freshness": { "type": "string", "enum": ["day", "week", "month", "year"] }
            },
            "required": ["query"]
        })
    }
    fn read_only(&self) -> bool {
        true
    }
    fn available(&self) -> bool {
        self.runtime.is_available()
    }
    fn execute(&self, _ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let provider = self.runtime.provider().ok_or_else(|| {
            ToolError::new(
                "web search is not available; enable it in Settings and configure the selected provider API key",
            )
        })?;
        let today = today_local();
        let query = SearchQuery {
            query: search_query_from_args(&args),
            max_results: max_results_from_args(&args),
            language: first_string(&args, &["language", "hl", "lang"]).filter(|s| !s.is_empty()),
            freshness: first_string(&args, &["freshness"]).filter(|s| !s.is_empty()),
        };
        if query.query.is_empty() {
            return Err(ToolError::new(
                "search query is required (parameter `query`; aliases: q, search, search_query)",
            ));
        }
        crate::runtime::isolated::run_isolated_or(
            move || {
                serde_json::to_string_pretty(&json!({
                    "provider": provider.id(),
                    "asOf": today,
                    "results": provider.search(&query)?,
                }))
                .map_err(|error| ToolError::new(error.to_string()))
            },
            |panic| Err(ToolError::new(format!("web_search panicked: {panic}"))),
        )
    }

    fn schema(&self) -> Value {
        let today = today_local();
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": format!(
                    "Search the web and return structured result metadata. Today's date is {today} (local timezone). For current, recent, or time-sensitive information, include this date in the query (and prefer freshness=day/week when appropriate). Use before browser_read when snippets are insufficient."
                ),
                "parameters": self.parameters_schema(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_query_accepts_common_aliases() {
        assert_eq!(
            search_query_from_args(&json!({ "query": "official" })),
            "official"
        );
        assert_eq!(search_query_from_args(&json!({ "q": "news" })), "news");
        assert_eq!(
            search_query_from_args(&json!({ "search_query": "booking" })),
            "booking"
        );
        assert_eq!(search_query_from_args(&json!({ "query": "  " })), "");
    }

    #[test]
    fn max_results_accepts_num_alias() {
        assert_eq!(max_results_from_args(&json!({ "num": 5 })), 5);
        assert_eq!(max_results_from_args(&json!({})), 8);
    }
}
