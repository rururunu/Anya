//! SSE streaming for chat-completions, responses, and Anthropic wire formats.

mod anthropic;
mod chat;
mod errors;
mod responses;
mod runner;
mod sse;
mod types;

pub(crate) use types::RETRY_BACKOFF;
pub(crate) use errors::emit_stream_error;
pub(crate) use runner::{run_anthropic_stream, run_chat_stream, run_responses_stream};

#[cfg(test)]
pub(crate) use types::{StreamReadOutcome, USER_STREAM_INTERRUPTED};
#[cfg(test)]
pub(crate) use errors::user_facing_stream_error;

#[cfg(test)]
mod utf8_stream_tests {
    use crate::runtime::encoding::append_utf8_chunk;

    #[test]
    fn sse_buffer_survives_split_multibyte_utf8() {
        let payload = "data: {\"choices\":[{\"delta\":{\"content\":\"你好\"}}]}\n\n";
        let bytes = payload.as_bytes();
        let mut pending = Vec::new();
        let mut buffer = String::new();
        for window in bytes.chunks(3) {
            append_utf8_chunk(&mut pending, window, &mut buffer);
        }
        assert!(pending.is_empty());
        assert!(buffer.contains("你好"));
        assert!(!buffer.contains('\u{FFFD}'));
    }
}

#[cfg(test)]
mod responses_event_tests {
    use super::responses::{apply_responses_event, responses_usage};
    use super::types::{ApiTokenUsage, StreamReadOutcome, ToolCallBuilder};
    use serde_json::json;
    use std::collections::HashMap;

    fn apply(
        event: serde_json::Value,
    ) -> (
        String,
        String,
        StreamReadOutcome,
        HashMap<usize, ToolCallBuilder>,
    ) {
        let mut content = String::new();
        let mut reasoning = String::new();
        let mut tool_calls = HashMap::new();
        let mut outcome = StreamReadOutcome::default();
        apply_responses_event(
            &event,
            &mut content,
            &mut reasoning,
            &mut tool_calls,
            &mut outcome,
        );
        (content, reasoning, outcome, tool_calls)
    }

    #[test]
    fn reasoning_summary_delta_is_collected() {
        let (content, reasoning, outcome, _) = apply(json!({
            "type": "response.reasoning_summary_text.delta",
            "delta": "step one"
        }));
        assert_eq!(reasoning, "step one");
        assert!(content.is_empty());
        assert!(outcome.emitted);
    }

    #[test]
    fn output_text_delta_is_collected() {
        let (content, reasoning, outcome, _) = apply(json!({
            "type": "response.output_text.delta",
            "delta": "hello"
        }));
        assert_eq!(content, "hello");
        assert!(reasoning.is_empty());
        assert!(outcome.emitted);
    }

    #[test]
    fn reasoning_summary_part_added_is_collected() {
        let (_, reasoning, outcome, _) = apply(json!({
            "type": "response.reasoning_summary_part.added",
            "part": { "type": "summary_text", "text": "consider energy" }
        }));
        assert_eq!(reasoning, "consider energy");
        assert!(outcome.emitted);
    }

    #[test]
    fn reasoning_item_done_is_collected() {
        let (_, reasoning, outcome, _) = apply(json!({
            "type": "response.output_item.done",
            "output_index": 0,
            "item": {
                "type": "reasoning",
                "summary": [{ "type": "summary_text", "text": "full trace" }]
            }
        }));
        assert_eq!(reasoning, "full trace");
        assert!(outcome.emitted);
    }

    #[test]
    fn function_call_item_is_collected() {
        let (_, _, outcome, tool_calls) = apply(json!({
            "type": "response.output_item.done",
            "output_index": 0,
            "item": {
                "type": "function_call",
                "call_id": "call-1",
                "name": "read_file",
                "arguments": "{\"path\":\"a.rs\"}"
            }
        }));
        let call = tool_calls.get(&0).expect("tool call");
        assert_eq!(call.id, "call-1");
        assert_eq!(call.name, "read_file");
        assert_eq!(call.arguments, "{\"path\":\"a.rs\"}");
        assert!(outcome.emitted);
    }

    #[test]
    fn completed_marks_stream_done() {
        let (_, _, outcome, _) = apply(json!({
            "type": "response.completed",
            "response": { "status": "completed" }
        }));
        assert!(outcome.is_complete());
        assert_eq!(outcome.finish_reason.as_deref(), Some("stop"));
    }

    #[test]
    fn cache_read_prefers_first_party_hit_tokens() {
        let usage: ApiTokenUsage = serde_json::from_value(json!({
            "prompt_tokens": 100,
            "completion_tokens": 10,
            "prompt_cache_hit_tokens": 80,
            "prompt_tokens_details": { "cached_tokens": 0 }
        }))
        .unwrap();
        assert_eq!(usage.cache_read_tokens(), 80);

        let usage: ApiTokenUsage = serde_json::from_value(json!({
            "prompt_tokens": 100,
            "completion_tokens": 10,
            "prompt_tokens_details": { "cached_tokens": 72 }
        }))
        .unwrap();
        assert_eq!(usage.cache_read_tokens(), 72);
    }

    #[test]
    fn responses_usage_reads_deepseek_cache_hit_field() {
        let parsed = responses_usage(&json!({
            "usage": {
                "input_tokens": 100,
                "output_tokens": 8,
                "prompt_cache_hit_tokens": 90
            }
        }))
        .expect("usage");
        assert_eq!(parsed, (100, 8, Some(90), None));
    }
}

#[cfg(test)]
mod retry_tests {
    use super::errors::{deepseek_http_status, is_retryable_stream_error};
    use crate::core::ai::provider::ProviderError;

    #[test]
    fn retries_transient_deepseek_http_errors_but_not_client_errors() {
        assert!(is_retryable_stream_error(&ProviderError::message(
            r#"DeepSeek API 500 Internal Server Error: {"type":"error"}"#
        )));
        assert!(is_retryable_stream_error(&ProviderError::message(
            "DeepSeek API 429 Too Many Requests: rate limited"
        )));
        assert!(!is_retryable_stream_error(&ProviderError::message(
            r#"DeepSeek API 400 Bad Request: {"error":{"message":"Unsupported model mimo-v2-omni"}}"#
        )));
        assert!(!is_retryable_stream_error(&ProviderError::message(
            "DeepSeek API 401 Unauthorized: bad key"
        )));
        assert_eq!(
            deepseek_http_status("DeepSeek API 500 Internal Server Error: x"),
            Some(500)
        );
    }
}
