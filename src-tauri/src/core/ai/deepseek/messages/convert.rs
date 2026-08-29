//! Chat-completions message JSON conversion and tool-protocol cleanup.
use std::collections::HashSet;

use serde_json::{json, Value};

use crate::core::runtime::{ChatMessage, Role};

pub(super) fn parse_multimodal_content(content: &str) -> Value {
    let content = crate::core::ai::image_gen::strip_edit_region_images(content);
    if !content.contains("![image](") {
        return json!(content);
    }

    let re = match regex::Regex::new(r"!\[image\]\((.*?)\)") {
        Ok(re) => re,
        Err(_) => return json!(content),
    };

    let mut parts = Vec::new();
    let mut last_index = 0;

    for cap in re.captures_iter(&content) {
        if let Some(mat) = cap.get(0) {
            let before = &content[last_index..mat.start()];
            if !before.trim().is_empty() {
                parts.push(json!({
                    "type": "text",
                    "text": before,
                }));
            }

            if let Some(url_match) = cap.get(1) {
                let url = url_match.as_str();
                parts.push(json!({
                    "type": "image_url",
                    "image_url": {
                        "url": url,
                    }
                }));
            }

            last_index = mat.end();
        }
    }

    if last_index < content.len() {
        let after = &content[last_index..];
        if !after.trim().is_empty() {
            parts.push(json!({
                "type": "text",
                "text": after,
            }));
        }
    }

    if parts.is_empty() {
        json!(content)
    } else {
        Value::Array(parts)
    }
}

/// Drop `role=tool` rows that are not a response to the nearest preceding
/// assistant `tool_calls`. DeepSeek (and other OpenAI-compatible APIs) return
/// 400 otherwise. Also fill empty tool-call ids so the pair can still match.
pub(super) fn normalize_tool_protocol(messages: &mut Vec<ChatMessage>) {
    let mut i = 0;
    while i < messages.len() {
        if messages[i].role == Role::Assistant {
            if let Some(calls) = messages[i].tool_calls.as_mut() {
                for (index, call) in calls.iter_mut().enumerate() {
                    if call.id.trim().is_empty() {
                        call.id = format!("call-{index}");
                    }
                }
                if calls.is_empty() {
                    messages[i].tool_calls = None;
                }
            }
            i += 1;
            continue;
        }

        if messages[i].role != Role::Tool {
            i += 1;
            continue;
        }

        let mut block_start = i;
        while block_start > 0 && messages[block_start - 1].role == Role::Tool {
            block_start -= 1;
        }
        let assistant_idx = block_start.checked_sub(1);
        let pending: Vec<String> = assistant_idx
            .and_then(|idx| {
                let assistant = &messages[idx];
                if assistant.role != Role::Assistant {
                    return None;
                }
                assistant.tool_calls.as_ref().and_then(|calls| {
                    if calls.is_empty() {
                        None
                    } else {
                        Some(calls.iter().map(|call| call.id.clone()).collect())
                    }
                })
            })
            .unwrap_or_default();

        if pending.is_empty() {
            messages.remove(i);
            continue;
        }

        let used: HashSet<String> = messages[block_start..i]
            .iter()
            .filter_map(|message| message.tool_call_id.clone())
            .collect();
        let raw_id = messages[i]
            .tool_call_id
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_string();
        let matched = if raw_id.is_empty() {
            pending.iter().find(|id| !used.contains(*id)).cloned()
        } else if pending.contains(&raw_id) && !used.contains(&raw_id) {
            Some(raw_id)
        } else {
            None
        };

        match matched {
            Some(id) => {
                messages[i].tool_call_id = Some(id);
                i += 1;
            }
            None => {
                messages.remove(i);
            }
        }
    }
}

pub(crate) fn message_to_api_json(
    message: &ChatMessage,
    pass_tool_reasoning: bool,
    is_deepseek: bool,
) -> Value {
    if message.role == Role::Tool {
        // DeepSeek rejects empty tool output; other providers tolerate "".
        let content = if is_deepseek && message.content.trim().is_empty() {
            "(no output)".to_string()
        } else {
            message.content.clone()
        };
        let mut payload = json!({
            "role": "tool",
            "tool_call_id": message.tool_call_id.clone().unwrap_or_default(),
            "content": content,
        });
        if let Some(name) = message.name.as_deref().filter(|name| !name.is_empty()) {
            payload
                .as_object_mut()
                .expect("tool payload object")
                .insert("name".into(), json!(name));
        }
        return payload;
    }

    if message.role == Role::Assistant {
        if let Some(tool_calls) = message
            .tool_calls
            .as_ref()
            .filter(|calls| !calls.is_empty())
        {
            let calls: Vec<Value> = tool_calls
                .iter()
                .map(|call| {
                    json!({
                        "id": call.id,
                        "type": "function",
                        "function": {
                            "name": call.name,
                            "arguments": call.arguments,
                        }
                    })
                })
                .collect();
            let mut payload = json!({
                "role": "assistant",
                // DeepSeek (and some gateways) reject null content on text-less
                // tool-call turns — send an empty string instead.
                "content": if message.content.is_empty() {
                    if is_deepseek { json!("") } else { Value::Null }
                } else { json!(message.content) },
                "tool_calls": calls,
            });
            if pass_tool_reasoning {
                // Protocol requires reasoning_content on every tool-call
                // assistant message once thinking was used; space placeholder
                // if empty.
                let reasoning = message
                    .reasoning
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(" ");
                payload
                    .as_object_mut()
                    .expect("assistant payload object")
                    .insert("reasoning_content".into(), json!(reasoning));
            }
            return payload;
        }
    }

    json!({
        "role": role_to_api(message.role),
        "content": parse_multimodal_content(&message.content),
    })
}

fn role_to_api(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
        Role::Tool => "tool",
    }
}
