//! Native file delivery. Successful structured results are durably recorded in
//! the owning assistant's tool activities, so replay needs no transient store.
use crate::core::tools::path::{normalize_path, resolve_tool_path};
use crate::core::tools::path_permission::PathAccess;
use crate::core::tools::{context::ToolContext, error::ToolError, Tool};
use serde_json::{json, Value};

pub struct PresentTool;
pub(crate) const MAX_RESULT_BYTES: usize = 1024 * 1024;
pub(crate) fn is_delivery_metadata(result: &str) -> bool {
    if result.len() > MAX_RESULT_BYTES {
        return false;
    }
    let Ok(value) = serde_json::from_str::<Value>(result) else {
        return false;
    };
    value["version"] == 1
        && value["files"].as_array().is_some_and(|files| {
            !files.is_empty()
                && files.len() <= 8
                && files.iter().all(|file| {
                    file["path"].as_str().is_some()
                        && file["absolutePath"].as_str().is_some()
                        && file["name"].as_str().is_some()
                        && file["size"].as_u64().is_some()
                        && (file["description"].is_null() || file["description"].is_string())
                })
        })
}
fn contract() -> &'static Value {
    crate::core::tools::dsh::contracts()
        .iter()
        .find(|s| s["function"]["name"] == "present")
        .expect("Imported present contract")
}
impl Tool for PresentTool {
    fn name(&self) -> &str {
        "present"
    }
    fn description(&self) -> &str {
        contract()["function"]["description"].as_str().unwrap()
    }
    fn parameters_schema(&self) -> Value {
        contract()["function"]["parameters"].clone()
    }
    fn read_only(&self) -> bool {
        true
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        execute(ctx, &args)
    }
}

pub(crate) fn execute(ctx: &ToolContext, args: &Value) -> Result<String, ToolError> {
    ctx.ensure_not_cancelled()?;
    let files = args["files"]
        .as_array()
        .ok_or_else(|| ToolError::new("files must be an array"))?;
    if files.is_empty() || files.len() > 8 {
        return Err(ToolError::new("present accepts 1 to 8 files"));
    }
    let mut deliveries = Vec::new();
    for file in files {
        let object = file
            .as_object()
            .ok_or_else(|| ToolError::new("Each file must be an object"))?;
        if object
            .keys()
            .any(|key| key != "path" && key != "description")
        {
            return Err(ToolError::new("Unexpected file property"));
        }
        let raw = file["path"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| ToolError::new("File path is required"))?;
        let description = match object.get("description") {
            Some(value) => Some(
                value
                    .as_str()
                    .ok_or_else(|| ToolError::new("description must be a string"))?,
            ),
            None => None,
        };
        let lexical = resolve_tool_path(ctx, raw, PathAccess::Read, "present")?;
        let metadata = std::fs::symlink_metadata(&lexical).map_err(|_| {
            ToolError::new(format!(
                "Cannot present {raw}: file not found or inaccessible"
            ))
        })?;
        if !metadata.file_type().is_file() {
            return Err(ToolError::new(format!(
                "Cannot present {raw}: not a regular file"
            )));
        }
        let canonical = normalize_path(&std::fs::canonicalize(&lexical)?);
        let canonical = canonical.to_string_lossy();
        let native = if let Some(rest) = canonical.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{rest}")
        } else {
            canonical
                .strip_prefix(r"\\?\")
                .unwrap_or(&canonical)
                .to_string()
        };
        let target = resolve_tool_path(ctx, &native, PathAccess::Read, "present")?;
        let metadata = std::fs::metadata(&target)?;
        if !metadata.is_file() {
            return Err(ToolError::new("Delivery target is not a regular file"));
        }
        deliveries.push(json!({"path":raw, "absolutePath":target.to_string_lossy(),
            "name":target.file_name().unwrap_or_default().to_string_lossy(),
            "description":description,"size":metadata.len()}));
    }
    ctx.ensure_not_cancelled()?;
    let result = json!({"version":1,"files":deliveries}).to_string();
    if result.len() > MAX_RESULT_BYTES {
        return Err(ToolError::new(
            "Delivery metadata is too large; shorten descriptions or present fewer files",
        ));
    }
    Ok(result)
}
