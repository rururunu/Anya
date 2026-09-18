use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;
use serde_json::{json, Value};

pub struct SearchToolsTool;

impl Tool for SearchToolsTool {
    fn name(&self) -> &str {
        "search_tools"
    }
    fn description(&self) -> &str {
        "Look up tools by capability when unsure which name to call. Search by capability or exact tool name (Office, browser/computer control, Git, LSP, MCP, plugins, memory, image generation, subagents, workspace search, …). Matching tools are already in the current schema — refine the query if nothing matches."
    }
    fn read_only(&self) -> bool {
        true
    }
    fn parameters_schema(&self) -> Value {
        json!({"type":"object","properties":{"query":{"type":"string","description":"Capability or tool name, e.g. Excel cell, browser click, git diff, subagent"}},"required":["query"]})
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let query = args["query"]
            .as_str()
            .filter(|q| !q.trim().is_empty())
            .ok_or_else(|| ToolError::new("query must be non-empty"))?;
        let registry = ctx
            .registry
            .as_ref()
            .ok_or_else(|| ToolError::new("registry unavailable"))?;
        let schemas = if crate::core::tools::plan_mode::shared_plan_mode_store()
            .is_active(ctx.root_session_id())
        {
            registry.filter_for_plan_mode().schemas()
        } else {
            registry.schemas()
        };
        Ok(json!({"tools":rank_tools(&schemas, query),"hint":"Matching tools are already callable by name; refine the query if no matches."}).to_string())
    }
}

fn rank_tools(schemas: &[Value], query: &str) -> Vec<Value> {
    let mut query = query.to_lowercase();
    for (word, alias) in [
        ("表格", "excel spreadsheet"),
        ("文档", "word document"),
        ("浏览器", "browser"),
        ("搜索", "search"),
        ("子代理", "subagent"),
        ("图片", "image"),
        ("记忆", "memory"),
        ("插件", "plugin"),
        ("幻灯片", "ppt slide"),
    ] {
        if query.contains(word) {
            query.push(' ');
            query.push_str(alias);
        }
    }
    let terms: Vec<_> = query.split_whitespace().collect();
    let mut ranked: Vec<_> = schemas
        .iter()
        .filter_map(|schema| {
            let function = &schema["function"];
            let name = function["name"].as_str().unwrap_or("").to_lowercase();
            if name == "search_tools" {
                return None;
            }
            let description = function["description"]
                .as_str()
                .unwrap_or("")
                .to_lowercase();
            let score: usize = terms
                .iter()
                .map(|term| {
                    if name.contains(term) {
                        5
                    } else if description.contains(term) {
                        1
                    } else {
                        0
                    }
                })
                .sum();
            (score > 0).then_some((score, name, schema.clone()))
        })
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    ranked.into_iter().take(8).map(|(_,_,schema)| json!({"function":{
        "name":schema["function"]["name"],
        "description":crate::core::chat::limits::truncate_chars(schema["function"]["description"].as_str().unwrap_or(""), 300)
    }})).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_ranks_names_and_accepts_chinese_capabilities() {
        let tools = vec![
            json!({"function":{"name":"word_read","description":"document"}}),
            json!({"function":{"name":"excel_read","description":"spreadsheet"}}),
        ];
        assert_eq!(
            rank_tools(&tools, "表格")[0]["function"]["name"],
            "excel_read"
        );
        assert!(rank_tools(&tools, "does-not-exist").is_empty());
    }
}
