use crate::core::ai::provider::ProviderError;
use crate::core::runtime::StreamEvent;
use serde_json::Value;
use tokio::sync::mpsc::Sender;

use super::anthropic::read_anthropic_sse_stream;
use super::chat::read_sse_stream;
use super::errors::is_retryable_stream_error;
use super::responses::read_responses_sse_stream;
use super::types::{SseKind, MAX_STREAM_ATTEMPTS, RETRY_BACKOFF, USER_STREAM_INTERRUPTED};

pub(crate) async fn run_chat_stream(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
    tx: &Sender<StreamEvent>,
) -> Result<(), ProviderError> {
    let is_deepseek = body
        .get("model")
        .and_then(Value::as_str)
        .is_some_and(|model| model.trim().to_ascii_lowercase().starts_with("deepseek"));
    run_sse_stream(
        client,
        url,
        api_key,
        body,
        tx,
        SseKind::ChatCompletions { is_deepseek },
    )
    .await
}

pub(crate) async fn run_responses_stream(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
    tx: &Sender<StreamEvent>,
) -> Result<(), ProviderError> {
    let is_deepseek = body
        .get("model")
        .and_then(Value::as_str)
        .is_some_and(|model| model.trim().to_ascii_lowercase().starts_with("deepseek"));
    run_sse_stream(
        client,
        url,
        api_key,
        body,
        tx,
        SseKind::Responses { is_deepseek },
    )
    .await
}

pub(crate) async fn run_anthropic_stream(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
    tx: &Sender<StreamEvent>,
) -> Result<(), ProviderError> {
    run_sse_stream(client, url, api_key, body, tx, SseKind::AnthropicMessages).await
}

async fn run_sse_stream(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
    tx: &Sender<StreamEvent>,
    kind: SseKind,
) -> Result<(), ProviderError> {
    let mut last_error: Option<ProviderError> = None;
    let mut retry_after = None;

    for attempt in 0..MAX_STREAM_ATTEMPTS {
        if attempt > 0 {
            signal_stream_retry(tx, attempt - 1).await;
            let delay = retry_delay(attempt, retry_after.take());
            tracing::warn!(attempt = attempt + 1, delay_ms = delay.as_millis(), error = ?last_error, "retrying provider stream");
            tokio::select! {
                _ = tokio::time::sleep(delay) => {},
                _ = tx.closed() => return Err(ProviderError::cancelled()),
            }
        }

        let attempt_started = std::time::Instant::now();
        let response = match post_stream_request(client, url, api_key, body, kind).await {
            Ok(response) => response,
            Err((error, wait)) => {
                if body["model"]
                    .as_str()
                    .is_some_and(crate::core::ai::registry::looks_like_deepseek_model)
                {
                    crate::core::chat::telemetry::record_provider_metrics(
                        &serde_json::json!({"kind":"wire_attempt", "attempt":attempt + 1, "phase":"request", "duration_ms":attempt_started.elapsed().as_millis(), "succeeded":false}),
                    );
                }
                retry_after = wait;
                last_error = Some(error.clone());
                if attempt + 1 < MAX_STREAM_ATTEMPTS && is_retryable_stream_error(&error) {
                    continue;
                }
                return Err(error);
            }
        };
        let headers_ms = attempt_started.elapsed().as_millis();

        let read = match kind {
            SseKind::ChatCompletions { is_deepseek } => {
                read_sse_stream(response, tx, is_deepseek).await
            }
            SseKind::Responses { is_deepseek } => {
                read_responses_sse_stream(response, tx, is_deepseek).await
            }
            SseKind::AnthropicMessages => read_anthropic_sse_stream(response, tx).await,
        };
        if body["model"]
            .as_str()
            .is_some_and(crate::core::ai::registry::looks_like_deepseek_model)
        {
            let outcome = read.as_ref().ok();
            crate::core::chat::telemetry::record_provider_metrics(&serde_json::json!({
                "kind":"wire_attempt", "attempt":attempt + 1, "phase":"stream", "headers_ms":headers_ms,
                "duration_ms":attempt_started.elapsed().as_millis(),
                "first_sse_ms":outcome.and_then(|o| o.first_sse_ms).map(|n| n + headers_ms),
                "first_content_ms":outcome.and_then(|o| o.first_text_ms).map(|n| n + headers_ms),
                "succeeded":outcome.is_some_and(|o| o.is_complete())
            }));
        }

        match read {
            Ok(outcome) if outcome.is_complete() => {
                let _ = tx.send(StreamEvent::Finish).await;
                return Ok(());
            }
            Ok(outcome) if outcome.emitted => {
                last_error = Some(ProviderError::message(USER_STREAM_INTERRUPTED));
                if attempt + 1 < MAX_STREAM_ATTEMPTS {
                    continue;
                }
                return Err(ProviderError::message(USER_STREAM_INTERRUPTED));
            }
            Ok(_) if attempt + 1 < MAX_STREAM_ATTEMPTS => {
                last_error = Some(ProviderError::message(USER_STREAM_INTERRUPTED));
                continue;
            }
            Ok(_) => return Err(ProviderError::message(USER_STREAM_INTERRUPTED)),
            Err(error)
                if attempt + 1 < MAX_STREAM_ATTEMPTS && is_retryable_stream_error(&error) =>
            {
                last_error = Some(error.clone());
                continue;
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| ProviderError::message(USER_STREAM_INTERRUPTED)))
}

fn retry_delay(attempt: u32, retry_after: Option<std::time::Duration>) -> std::time::Duration {
    let base = RETRY_BACKOFF * (1u32 << attempt.saturating_sub(1).min(4));
    let jitter = std::time::Duration::from_millis((uuid::Uuid::new_v4().as_u128() % 1000) as u64);
    (base + jitter).max(retry_after.unwrap_or_default())
}

fn parse_retry_after(value: &str) -> Option<std::time::Duration> {
    if let Ok(seconds) = value.trim().parse::<u64>() {
        return Some(std::time::Duration::from_secs(seconds));
    }
    let date = chrono::DateTime::parse_from_rfc2822(value).ok()?;
    Some(std::time::Duration::from_secs(
        (date.timestamp() - chrono::Utc::now().timestamp()).max(0) as u64,
    ))
}

async fn signal_stream_retry(tx: &Sender<StreamEvent>, attempt: u32) {
    let _ = tx
        .send(StreamEvent::Status {
            kind: format!("stream_retry:{}:{}", attempt + 1, MAX_STREAM_ATTEMPTS),
        })
        .await;
}

async fn post_stream_request(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
    kind: SseKind,
) -> Result<reqwest::Response, (ProviderError, Option<std::time::Duration>)> {
    let mut request = client
        .post(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json");
    if matches!(kind, SseKind::AnthropicMessages) {
        request = request
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01");
        if crate::core::ai::deepseek::files::json_contains_file_id(body) {
            request = request.header(
                "anthropic-beta",
                crate::core::ai::deepseek::files::ANTHROPIC_FILES_BETA,
            );
        }
    }
    let response = request.json(body).send().await.map_err(|error| {
        (
            ProviderError::message(format!("network error: {error}")),
            None,
        )
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(parse_retry_after);
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        let model = body
            .get("model")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let prefix = match model {
            Some(model_name) => format!("{model_name} API"),
            None => "API".to_string(),
        };
        return Err((
            ProviderError::message(format!("{prefix} {status}: {text}")),
            retry_after,
        ));
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn retry_backoff_grows_and_honors_server_delay() {
        assert!(retry_delay(4, None) >= Duration::from_secs(16));
        assert!(retry_delay(1, Some(Duration::from_secs(90))) >= Duration::from_secs(90));
        assert_eq!(parse_retry_after("30"), Some(Duration::from_secs(30)));
        assert_eq!(
            parse_retry_after("Wed, 21 Oct 2015 07:28:00 GMT"),
            Some(Duration::ZERO)
        );
        assert_eq!(parse_retry_after("invalid"), None);
    }

    #[tokio::test]
    async fn http_520_and_partial_sse_recover_without_merging_attempts() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/chat/completions", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let partial = "data: {\"choices\":[{\"delta\":{\"content\":\"discard this\"}}]}\n\n";
            let complete = "data: {\"choices\":[{\"delta\":{\"content\":\"complete result\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
            for (status, body) in [
                ("520 unavailable", "unavailable"),
                ("200 OK", partial),
                ("200 OK", complete),
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                loop {
                    let mut buffer = [0u8; 4096];
                    let count = socket.read(&mut buffer).await.unwrap();
                    if count == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..count]);
                    if let Some(end) = request.windows(4).position(|b| b == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                        let length = head
                            .lines()
                            .find_map(|line| {
                                line.strip_prefix("content-length:")
                                    .and_then(|n| n.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        if request.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                let response = format!("HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nRetry-After: 0\r\nConnection: close\r\n\r\n{body}", body.len());
                socket.write_all(response.as_bytes()).await.unwrap();
                socket.shutdown().await.unwrap();
            }
        });
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        let task = tokio::spawn(async move {
            run_chat_stream(
                &reqwest::Client::new(),
                &url,
                "test",
                &serde_json::json!({"model":"test-model"}),
                &tx,
            )
            .await
        });
        let received = tokio::time::timeout(Duration::from_secs(20), async move {
            let mut received = Vec::new();
            while let Some(event) = rx.recv().await {
                received.push(event);
            }
            received
        })
        .await
        .unwrap();
        task.await.unwrap().unwrap();
        server.await.unwrap();
        let mut retries = 0;
        let mut completed = false;
        for event in received {
            match event {
                StreamEvent::Status { kind } if kind.starts_with("stream_retry") => retries += 1,
                StreamEvent::TurnComplete { content, .. } => {
                    assert_eq!(content, "complete result");
                    completed = true;
                }
                _ => {}
            }
        }
        assert_eq!(retries, 2);
        assert!(completed);
    }
}
