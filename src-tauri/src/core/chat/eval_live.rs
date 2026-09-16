//! Optional HTTP provider for headless, real-model evaluations.
//! Credentials are supplied explicitly through environment variables, never reports.
use crate::core::ai::provider::{AIProvider, ProviderError};
use crate::core::runtime::{ChatRequest, Role, StreamEvent, ToolCallPayload};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct LiveEvalConfig {
    pub endpoint: String,
    pub model: String,
    api_key: String,
}

impl std::fmt::Debug for LiveEvalConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveEvalConfig")
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}

impl LiveEvalConfig {
    pub fn from_env() -> Result<Self, String> {
        let endpoint = std::env::var("ANYA_EVAL_ENDPOINT")
            .map_err(|_| "ANYA_EVAL_ENDPOINT must be a full chat-completions endpoint")?;
        let model = std::env::var("ANYA_EVAL_MODEL").map_err(|_| "ANYA_EVAL_MODEL is required")?;
        let api_key =
            std::env::var("ANYA_EVAL_API_KEY").map_err(|_| "ANYA_EVAL_API_KEY is required")?;
        let url = reqwest::Url::parse(&endpoint).map_err(|_| "invalid evaluation endpoint")?;
        if !url.username().is_empty() || url.password().is_some() || url.query().is_some() {
            return Err(
                "Use the dedicated API key environment variable, not credentials in the URL".into(),
            );
        }
        if url.scheme() != "https"
            && !(url.scheme() == "http"
                && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")))
        {
            return Err(
                "Evaluation endpoint must use HTTPS (HTTP is allowed for localhost)".into(),
            );
        }
        if model.trim().is_empty() || api_key.trim().is_empty() {
            return Err("Evaluation model and API key must be non-empty".into());
        }
        Ok(Self {
            endpoint,
            model,
            api_key,
        })
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalMetrics {
    pub duration_ms: u64,
    pub model_calls: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub token_usage_reported: bool,
    pub tool_calls: usize,
    pub failed_tool_calls: usize,
    pub repeated_tool_calls: usize,
    pub user_interventions: usize,
}

pub struct LiveProvider {
    config: LiveEvalConfig,
    client: reqwest::Client,
    pub metrics: Arc<Mutex<EvalMetrics>>,
}

impl LiveProvider {
    pub fn new(config: LiveEvalConfig, metrics: Arc<Mutex<EvalMetrics>>) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            config,
            client,
            metrics,
        })
    }
}

fn request_body(request: &ChatRequest, model: &str) -> Value {
    let messages: Vec<_> = request.messages.iter().map(|message| {
        let role = match message.role { Role::System => "system", Role::User => "user", Role::Assistant => "assistant", Role::Tool => "tool" };
        let mut value = json!({"role":role,"content":message.content});
        if let Some(id) = &message.tool_call_id { value["tool_call_id"] = json!(id); }
        if let Some(reasoning) = &message.reasoning { value["reasoning_content"] = json!(reasoning); }
        if let Some(calls) = &message.tool_calls {
            if !calls.is_empty() { value["tool_calls"] = json!(calls.iter().map(|call| json!({"id":call.id,"type":"function","function":{"name":call.name,"arguments":call.arguments}})).collect::<Vec<_>>()); }
        }
        value
    }).collect();
    let mut body = json!({"model":model,"messages":messages,"stream":false});
    if !request.tools.is_empty() {
        body["tools"] = json!(request.tools.as_ref());
    }
    if let Some(temperature) = request.temperature {
        body["temperature"] = json!(temperature);
    }
    if let Some(max_tokens) = request.max_tokens {
        body["max_tokens"] = json!(max_tokens);
    }
    body
}

fn parse_completion(value: &Value) -> Result<StreamEvent, ProviderError> {
    let choice = value["choices"]
        .as_array()
        .and_then(|items| items.first())
        .ok_or_else(|| ProviderError::message("evaluation provider returned no choices"))?;
    let message = &choice["message"];
    let mut calls = Vec::new();
    if let Some(items) = message["tool_calls"].as_array() {
        for item in items {
            let id = item["id"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| ProviderError::message("tool call missing id"))?;
            let name = item["function"]["name"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| ProviderError::message("tool call missing name"))?;
            let arguments = item["function"]["arguments"]
                .as_str()
                .ok_or_else(|| ProviderError::message("tool arguments must be a JSON string"))?;
            calls.push(ToolCallPayload {
                id: id.into(),
                name: name.into(),
                arguments: arguments.into(),
                thought_signature: None,
            });
        }
    }
    Ok(StreamEvent::TurnComplete {
        content: message["content"].as_str().unwrap_or("").into(),
        reasoning: message["reasoning_content"].as_str().map(str::to_string),
        tool_calls: calls,
        finish_reason: choice["finish_reason"].as_str().map(str::to_string),
    })
}

#[async_trait::async_trait]
impl AIProvider for LiveProvider {
    fn id(&self) -> &'static str {
        "live-eval"
    }
    async fn stream(
        &self,
        request: ChatRequest,
        tx: tokio::sync::mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.model_calls += 1;
        }
        let response = self
            .client
            .post(&self.config.endpoint)
            .bearer_auth(&self.config.api_key)
            .json(&request_body(&request, &self.config.model))
            .send()
            .await
            .map_err(|_| ProviderError::message("evaluation provider network request failed"))?;
        if !response.status().is_success() {
            return Err(ProviderError::message(format!(
                "evaluation provider HTTP {}",
                response.status().as_u16()
            )));
        }
        let value: Value = response
            .json()
            .await
            .map_err(|_| ProviderError::message("invalid evaluation provider JSON"))?;
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.token_usage_reported |= value["usage"].is_object();
            metrics.input_tokens += value["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
            metrics.output_tokens += value["usage"]["completion_tokens"].as_u64().unwrap_or(0);
        }
        tx.send(parse_completion(&value)?)
            .await
            .map_err(|_| ProviderError::cancelled())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_real_tool_calls_and_rejects_malformed_responses() {
        let event = parse_completion(&json!({"choices":[{"message":{"content":null,"tool_calls":[{"id":"1","function":{"name":"read_file","arguments":"{\"path\":\"a.txt\"}"}}]},"finish_reason":"tool_calls"}]})).unwrap();
        assert!(
            matches!(event, StreamEvent::TurnComplete { tool_calls, .. } if tool_calls.len() == 1 && tool_calls[0].name == "read_file")
        );
        assert!(parse_completion(&json!({"error":"no"})).is_err());
        assert!(!format!(
            "{:?}",
            LiveEvalConfig {
                endpoint: "https://example.test".into(),
                model: "test".into(),
                api_key: "secret-token".into()
            }
        )
        .contains("secret-token"));
    }

    #[tokio::test]
    async fn http_provider_runs_the_real_agent_loop_and_records_usage() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!(
            "http://{}/v1/chat/completions",
            listener.local_addr().unwrap()
        );
        let server = std::thread::spawn(move || {
            for step in 0..3 {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 4096];
                loop {
                    let count = socket.read(&mut buffer).unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&buffer[..count]);
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                        let length: usize = headers
                            .lines()
                            .find_map(|line| line.strip_prefix("content-length:"))
                            .unwrap()
                            .trim()
                            .parse()
                            .unwrap();
                        if bytes.len() >= end + 4 + length {
                            let request: Value =
                                serde_json::from_slice(&bytes[end + 4..end + 4 + length]).unwrap();
                            assert_eq!(request["model"], "local-mock");
                            if step > 0 {
                                assert!(request["messages"]
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .any(|m| m["role"] == "tool"));
                            }
                            break;
                        }
                    }
                }
                let message = match step {
                    0 => {
                        json!({"content":"", "tool_calls":[{"id":"write-1","type":"function","function":{"name":"write_file","arguments":"{\"path\":\"answer.txt\",\"content\":\"verified\"}"}}]})
                    }
                    1 => {
                        json!({"content":"", "tool_calls":[{"id":"read-1","type":"function","function":{"name":"read_file","arguments":"{\"path\":\"answer.txt\"}"}}]})
                    }
                    _ => json!({"content":"Created and read back answer.txt."}),
                };
                let body = json!({"choices":[{"message":message,"finish_reason":"stop"}],"usage":{"prompt_tokens":11,"completion_tokens":7}}).to_string();
                write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            }
        });
        let root =
            std::env::temp_dir().join(format!("anya-live-eval-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("task.json"), json!({"id":"http_tool_roundtrip", "prompt":"Write answer.txt containing verified and check it.", "assertions":[{"type":"fileEquals","path":"answer.txt","text":"verified"},{"type":"finishReason","reason":"stop"}]}).to_string()).unwrap();
        let report = super::super::run_eval(super::super::EvalOptions {
            challenges: true,
            compact: true,
            tasks_dir: root.clone(),
            results_dir: root.join("results"),
            seeds: 1,
            live: Some(LiveEvalConfig {
                endpoint,
                model: "local-mock".into(),
                api_key: "test-only".into(),
            }),
            ..Default::default()
        })
        .await
        .unwrap();
        server.join().unwrap();
        assert_eq!(report.failed, 0, "{:?}", report.results);
        let metrics = &report.results[0].metrics;
        assert_eq!(metrics.model_calls, 3);
        assert_eq!(metrics.tool_calls, 2);
        assert_eq!(metrics.input_tokens, 33);
        assert_eq!(metrics.output_tokens, 21);
        std::fs::remove_dir_all(root).unwrap();
    }
}
