//! Per-model chat-completions reasoning / thinking payload mapping.
use serde_json::{json, Map, Value};

use crate::models::settings::ReasoningEffort;

use super::is_thinking_off;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReasoningFamily {
    DeepSeek,
    OpenAi,
    KimiK3,
    KimiK2,
    Qwen38,
    Qwen,
    Glm52,
    Glm,
    Claude,
    MiniMax,
    Other,
}

fn reasoning_family(model: &str) -> ReasoningFamily {
    let model = model.trim().to_ascii_lowercase();
    if model.contains("deepseek") {
        return ReasoningFamily::DeepSeek;
    }
    if model.contains("kimi-k3") || model.contains("kimi_k3") || model.contains("kimik3") {
        return ReasoningFamily::KimiK3;
    }
    if model.contains("kimi") || model.contains("moonshot") {
        return ReasoningFamily::KimiK2;
    }
    if model.contains("qwen3.8") || model.contains("qwen3-8") || model.contains("qwen38") {
        return ReasoningFamily::Qwen38;
    }
    if model.contains("qwen") || model.contains("qwq") || model.contains("qvq") {
        return ReasoningFamily::Qwen;
    }
    if model.contains("glm-5") || model.contains("glm5") || model.contains("glm_5") {
        return ReasoningFamily::Glm52;
    }
    if model.contains("glm") || model.contains("chatglm") {
        return ReasoningFamily::Glm;
    }
    if model.contains("claude") || model.contains("anthropic") {
        return ReasoningFamily::Claude;
    }
    if model.contains("minimax") || model.contains("mimo") {
        return ReasoningFamily::MiniMax;
    }
    if model.contains("gpt-5") || is_openai_o_series(&model) {
        return ReasoningFamily::OpenAi;
    }
    ReasoningFamily::Other
}

fn is_openai_o_series(model: &str) -> bool {
    ["o1", "o3", "o4"].iter().any(|needle| {
        model == *needle
            || model.contains(&format!("{needle}-"))
            || model.contains(&format!("-{needle}"))
            || model.contains(&format!("/{needle}"))
            || model.contains(&format!(".{needle}"))
    })
}

pub(super) fn apply_chat_reasoning(
    body: &mut Map<String, Value>,
    model: &str,
    is_deepseek: bool,
    effort: ReasoningEffort,
) {
    if is_deepseek {
        apply_deepseek_thinking(body, effort);
        return;
    }
    match reasoning_family(model) {
        ReasoningFamily::DeepSeek => apply_deepseek_thinking(body, effort),
        ReasoningFamily::KimiK3 => apply_kimi_k3_effort(body, effort),
        ReasoningFamily::KimiK2 => apply_toggle_thinking(body, effort),
        ReasoningFamily::Qwen38 => apply_qwen38_effort(body, effort),
        ReasoningFamily::Qwen => apply_qwen_thinking(body, effort),
        ReasoningFamily::Glm52 => apply_glm52_effort(body, effort),
        ReasoningFamily::Glm => apply_toggle_thinking(body, effort),
        ReasoningFamily::OpenAi | ReasoningFamily::Claude | ReasoningFamily::MiniMax => {
            apply_openai_reasoning_effort(body, effort);
        }
        ReasoningFamily::Other => {}
    }
}

fn openai_effort_wire(effort: ReasoningEffort) -> Option<&'static str> {
    match effort {
        ReasoningEffort::Disabled => None,
        ReasoningEffort::None => Some("none"),
        ReasoningEffort::Minimal => Some("minimal"),
        ReasoningEffort::Low => Some("low"),
        ReasoningEffort::Medium => Some("medium"),
        ReasoningEffort::High => Some("high"),
        ReasoningEffort::Xhigh => Some("xhigh"),
        ReasoningEffort::Max => Some("max"),
    }
}

fn apply_openai_reasoning_effort(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    if let Some(value) = openai_effort_wire(effort) {
        body.insert("reasoning_effort".into(), json!(value));
    }
}

fn apply_deepseek_thinking(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    if is_thinking_off(effort) {
        body.insert("thinking".into(), json!({ "type": "disabled" }));
        return;
    }
    let mapped = match effort {
        ReasoningEffort::Minimal | ReasoningEffort::Low => "low",
        ReasoningEffort::Max => "max",
        _ => "high",
    };
    body.insert("thinking".into(), json!({ "type": "enabled" }));
    body.insert("reasoning_effort".into(), json!(mapped));
}

fn apply_toggle_thinking(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    if is_thinking_off(effort) {
        body.insert("thinking".into(), json!({ "type": "disabled" }));
        body.insert("enable_thinking".into(), json!(false));
        return;
    }
    body.insert("thinking".into(), json!({ "type": "enabled" }));
    body.insert("enable_thinking".into(), json!(true));
}

fn apply_kimi_k3_effort(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    let mapped = match effort {
        ReasoningEffort::Disabled | ReasoningEffort::None => "max",
        ReasoningEffort::Minimal | ReasoningEffort::Low => "low",
        ReasoningEffort::Max | ReasoningEffort::Xhigh => "max",
        ReasoningEffort::Medium | ReasoningEffort::High => "high",
    };
    body.insert("reasoning_effort".into(), json!(mapped));
}

fn apply_qwen38_effort(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    if is_thinking_off(effort) {
        body.insert("enable_thinking".into(), json!(false));
        return;
    }
    let mapped = match effort {
        ReasoningEffort::Minimal | ReasoningEffort::Low => "low",
        ReasoningEffort::Medium => "medium",
        _ => "xhigh",
    };
    body.insert("enable_thinking".into(), json!(true));
    body.insert("reasoning_effort".into(), json!(mapped));
}

fn apply_qwen_thinking(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    if is_thinking_off(effort) {
        body.insert("enable_thinking".into(), json!(false));
        return;
    }
    body.insert("enable_thinking".into(), json!(true));
    if let Some(value) = openai_effort_wire(effort) {
        body.insert("reasoning_effort".into(), json!(value));
    }
}

fn apply_glm52_effort(body: &mut Map<String, Value>, effort: ReasoningEffort) {
    if is_thinking_off(effort) || matches!(effort, ReasoningEffort::Minimal) {
        body.insert("thinking".into(), json!({ "type": "disabled" }));
        body.insert("enable_thinking".into(), json!(false));
        if matches!(effort, ReasoningEffort::None | ReasoningEffort::Minimal) {
            if let Some(value) = openai_effort_wire(effort) {
                body.insert("reasoning_effort".into(), json!(value));
            }
        }
        return;
    }
    body.insert("thinking".into(), json!({ "type": "enabled" }));
    body.insert("enable_thinking".into(), json!(true));
    if let Some(value) = openai_effort_wire(effort) {
        body.insert("reasoning_effort".into(), json!(value));
    }
}
