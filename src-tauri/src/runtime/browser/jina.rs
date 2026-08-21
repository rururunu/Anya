use std::net::IpAddr;
use std::time::Duration;

use reqwest::blocking::Client;

use crate::runtime::browser::{BrowserDocument, BrowserProvider};
use crate::runtime::tool::ToolError;

const MAX_MARKDOWN_CHARS: usize = 120_000;

pub struct JinaReaderProvider {
    base_url: String,
    api_key: Option<String>,
    client: Client,
}

impl JinaReaderProvider {
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Result<Self, ToolError> {
        let client = crate::runtime::isolated::run_isolated(|| {
            Client::builder()
                .connect_timeout(Duration::from_secs(3))
                .timeout(Duration::from_secs(12))
                .user_agent("Anya-Runtime/3")
                .build()
                .map_err(|error| error.to_string())
        })
        .map_err(ToolError::new)?;
        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').into(),
            api_key,
            client,
        })
    }
}

fn validate_public_url(value: &str) -> Result<reqwest::Url, ToolError> {
    let url = reqwest::Url::parse(value)
        .map_err(|error| ToolError::new(format!("invalid URL: {error}")))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(ToolError::new("browser URL must use HTTP or HTTPS"));
    }
    let host = url
        .host_str()
        .ok_or_else(|| ToolError::new("browser URL has no host"))?;
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".local") {
        return Err(ToolError::new("local browser targets are not allowed"));
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        let blocked = match ip {
            IpAddr::V4(ip) => {
                ip.is_private() || ip.is_loopback() || ip.is_link_local() || ip.is_unspecified()
            }
            IpAddr::V6(ip) => {
                ip.is_loopback()
                    || ip.is_unspecified()
                    || ip.is_unique_local()
                    || ip.is_unicast_link_local()
            }
        };
        if blocked {
            return Err(ToolError::new("private browser targets are not allowed"));
        }
    }
    Ok(url)
}

impl BrowserProvider for JinaReaderProvider {
    fn id(&self) -> &'static str {
        "jina-reader"
    }

    fn read(&self, value: &str) -> Result<BrowserDocument, ToolError> {
        let url = validate_public_url(value)?;
        let mut request = self.client.get(format!("{}/{}", self.base_url, url));
        if let Some(api_key) = &self.api_key {
            request = request.bearer_auth(api_key);
        }
        let markdown = request
            .header("Accept", "text/markdown")
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .and_then(reqwest::blocking::Response::text)
            .map_err(|error| ToolError::new(format!("Jina Reader request failed: {error}")))?;
        let truncated = markdown.chars().count() > MAX_MARKDOWN_CHARS;
        Ok(BrowserDocument {
            url: url.to_string(),
            markdown: markdown.chars().take(MAX_MARKDOWN_CHARS).collect(),
            truncated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_local_targets() {
        assert!(validate_public_url("http://127.0.0.1/admin").is_err());
        assert!(validate_public_url("http://localhost/admin").is_err());
        assert!(validate_public_url("https://example.com").is_ok());
    }
}
