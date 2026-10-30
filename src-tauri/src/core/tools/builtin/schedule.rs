//! Create and manage the current bot's scheduled tasks.

use serde_json::{json, Value};

use crate::core::bots::{create_task, delete_task, list_tasks, set_task_enabled, ScheduleSpec};
use crate::core::tools::context::{Tool, ToolContext};
use crate::core::tools::error::ToolError;

pub(super) struct ManageScheduleTool;

impl Tool for ManageScheduleTool {
    fn name(&self) -> &str {
        "manage_schedule"
    }
    fn description(&self) -> &str {
        "Create, list, or cancel scheduled tasks for the bot bound to this conversation. Each bot keeps its own tasks and memory. action is create, list, cancel, or set_enabled. For create, pass title, instruction, and spec. spec.type is once (atMs, unix milliseconds), interval (everyMinutes), daily (time as HH:MM local), or weekly (weekday 0=Monday through 6=Sunday, plus time)."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["create", "list", "cancel", "set_enabled"] },
                "title": { "type": "string" },
                "instruction": { "type": "string" },
                "id": { "type": "string" },
                "enabled": { "type": "boolean" },
                "spec": {
                    "type": "object",
                    "properties": {
                        "type": { "type": "string", "enum": ["once", "interval", "daily", "weekly"] },
                        "atMs": { "type": "number" },
                        "everyMinutes": { "type": "number" },
                        "time": { "type": "string" },
                        "weekday": { "type": "number" }
                    },
                    "required": ["type"]
                }
            },
            "required": ["action"]
        })
    }
    fn execute(&self, ctx: &ToolContext, args: Value) -> Result<String, ToolError> {
        let bot = crate::core::bots::bot_for_session(&ctx.session_id).ok_or_else(|| {
            ToolError::new("manage_schedule only works in a bot conversation. Ask the user to create a bot first.")
        })?;
        match args["action"].as_str().unwrap_or("") {
            "list" => {
                let tasks = list_tasks()
                    .map_err(ToolError::new)?
                    .into_iter()
                    .filter(|task| task.bot_id == bot.id)
                    .collect::<Vec<_>>();
                serde_json::to_string(&tasks).map_err(|error| ToolError::new(error.to_string()))
            }
            "cancel" => {
                let id = required_str(&args, "id")?;
                ensure_owned(&bot.id, &id)?;
                delete_task(&id).map_err(ToolError::new)?;
                Ok("cancelled".into())
            }
            "set_enabled" => {
                let id = required_str(&args, "id")?;
                ensure_owned(&bot.id, &id)?;
                let enabled = args["enabled"].as_bool().unwrap_or(true);
                let task = set_task_enabled(&id, enabled).map_err(ToolError::new)?;
                serde_json::to_string(&task).map_err(|error| ToolError::new(error.to_string()))
            }
            "create" => {
                let spec = parse_spec(&args["spec"])?;
                let task = create_task(
                    &bot.id,
                    args["title"].as_str().unwrap_or("").to_string(),
                    args["instruction"].as_str().unwrap_or("").to_string(),
                    spec,
                )
                .map_err(ToolError::new)?;
                serde_json::to_string(&task).map_err(|error| ToolError::new(error.to_string()))
            }
            _ => Err(ToolError::new(
                "action must be create, list, cancel, or set_enabled",
            )),
        }
    }
}

fn required_str(args: &Value, key: &str) -> Result<String, ToolError> {
    args[key]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| ToolError::new(format!("{key} is required")))
}

fn ensure_owned(bot_id: &str, task_id: &str) -> Result<(), ToolError> {
    let tasks = list_tasks().map_err(ToolError::new)?;
    if tasks
        .iter()
        .any(|task| task.id == task_id && task.bot_id == bot_id)
    {
        return Ok(());
    }
    Err(ToolError::new("schedule not found for this bot"))
}

fn parse_spec(value: &Value) -> Result<ScheduleSpec, ToolError> {
    match value["type"].as_str().unwrap_or("") {
        "once" => Ok(ScheduleSpec::Once {
            at_ms: value["atMs"].as_i64().ok_or_else(|| {
                ToolError::new("once spec requires atMs, a unix timestamp in milliseconds")
            })?,
        }),
        "interval" => Ok(ScheduleSpec::Interval {
            every_minutes: value["everyMinutes"].as_u64().ok_or_else(|| {
                ToolError::new("interval spec requires everyMinutes")
            })? as u32,
        }),
        "daily" => Ok(ScheduleSpec::Daily {
            time: required_str(value, "time")?,
        }),
        "weekly" => Ok(ScheduleSpec::Weekly {
            weekday: value["weekday"].as_u64().ok_or_else(|| {
                ToolError::new("weekly spec requires weekday 0 (Monday) through 6 (Sunday)")
            })? as u8,
            time: required_str(value, "time")?,
        }),
        _ => Err(ToolError::new(
            "spec.type must be once, interval, daily, or weekly",
        )),
    }
}
