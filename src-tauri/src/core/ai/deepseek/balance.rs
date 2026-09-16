use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::ProviderError;

const BALANCE_URL: &str = "https://api.deepseek.com/user/balance";
const BALANCE_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const BALANCE_REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeepSeekBalanceInfo {
    pub currency: String,
    pub total_balance: String,
    pub granted_balance: String,
    pub topped_up_balance: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeepSeekBalanceReport {
    pub configured: bool,
    pub is_available: Option<bool>,
    pub balances: Vec<DeepSeekBalanceInfo>,
}

#[derive(Debug, Deserialize)]
struct RawBalanceResponse {
    is_available: bool,
    #[serde(default)]
    balance_infos: Vec<RawBalanceInfo>,
}

#[derive(Debug, Deserialize)]
struct RawBalanceInfo {
    currency: String,
    total_balance: String,
    granted_balance: String,
    topped_up_balance: String,
}

/// Parse a DeepSeek `/user/balance` JSON payload.
pub fn parse_balance_payload(text: &str) -> Result<DeepSeekBalanceReport, ProviderError> {
    let payload: RawBalanceResponse = serde_json::from_str(text).map_err(|error| {
        ProviderError::message(format!("invalid balance payload: {error}"))
    })?;
    Ok(DeepSeekBalanceReport {
        configured: true,
        is_available: Some(payload.is_available),
        balances: payload
            .balance_infos
            .into_iter()
            .map(|item| DeepSeekBalanceInfo {
                currency: item.currency,
                total_balance: item.total_balance,
                granted_balance: item.granted_balance,
                topped_up_balance: item.topped_up_balance,
            })
            .collect(),
    })
}

/// `GET /user/balance` — DeepSeek account balance.
pub async fn get_user_balance(api_key: &str) -> Result<DeepSeekBalanceReport, ProviderError> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Ok(DeepSeekBalanceReport {
            configured: false,
            is_available: None,
            balances: Vec::new(),
        });
    }

    let client = reqwest::Client::builder()
        .connect_timeout(BALANCE_CONNECT_TIMEOUT)
        .timeout(BALANCE_REQUEST_TIMEOUT)
        .build()
        .map_err(|error| ProviderError::message(format!("failed to build HTTP client: {error}")))?;
    let response = client
        .get(BALANCE_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
        )
        .send()
        .await
        .map_err(|error| {
            ProviderError::message(format!(
                "network error: {error}. Check the network/proxy settings."
            ))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        return Err(ProviderError::message(format!("API {status}: {text}")));
    }

    let text = response
        .text()
        .await
        .map_err(|error| ProviderError::message(format!("failed to read response: {error}")))?;
    parse_balance_payload(&text)
}

#[cfg(test)]
mod tests {
    use super::parse_balance_payload;

    #[test]
    fn parses_official_example() {
        let report = parse_balance_payload(
            r#"{
              "is_available": true,
              "balance_infos": [
                {
                  "currency": "CNY",
                  "total_balance": "110.00",
                  "granted_balance": "10.00",
                  "topped_up_balance": "100.00"
                }
              ]
            }"#,
        )
        .expect("payload");
        assert!(report.configured);
        assert_eq!(report.is_available, Some(true));
        assert_eq!(report.balances[0].currency, "CNY");
        assert_eq!(report.balances[0].total_balance, "110.00");
        assert_eq!(report.balances[0].granted_balance, "10.00");
        assert_eq!(report.balances[0].topped_up_balance, "100.00");
    }
}
