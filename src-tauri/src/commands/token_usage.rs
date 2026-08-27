use std::collections::BTreeMap;

use chrono::{Datelike, Duration, Local, TimeZone};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::State;

use crate::app_state::AppState;
use crate::core::token::{TokenAccuracy, TokenUsage};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsageReportRequest {
    pub from: Option<i64>,
    pub to: Option<i64>,
    #[serde(default = "default_granularity")]
    pub granularity: String,
}

fn default_granularity() -> String {
    "day".into()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsageReport {
    pub from: i64,
    pub to: i64,
    pub granularity: String,
    pub total: TokenUsage,
    pub model_calls: usize,
    pub by_model: Vec<ModelUsage>,
    pub timeline: Vec<UsageBucket>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub model: String,
    pub provider: Option<String>,
    pub usage: TokenUsage,
    pub calls: usize,
    pub share: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageBucket {
    pub bucket: String,
    pub label: String,
    pub total_tokens: usize,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub models: BTreeMap<String, usize>,
}

fn bucket_start(timestamp: i64, granularity: &str) -> chrono::DateTime<Local> {
    let date = Local
        .timestamp_millis_opt(timestamp)
        .single()
        .unwrap_or_else(Local::now);
    let day = date.date_naive();
    let start = match granularity {
        "week" => {
            let days = date.weekday().num_days_from_monday() as i64;
            day - Duration::days(days)
        }
        "month" => day.with_day(1).unwrap_or(day),
        _ => day,
    };
    Local
        .from_local_datetime(&start.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .unwrap_or(date)
}

fn add_usage(target: &mut TokenUsage, row: &sqlx::sqlite::SqliteRow) {
    let was_empty = target.total_tokens == 0;
    target.input_tokens += row.get::<i64, _>("input_tokens").max(0) as usize;
    target.output_tokens += row.get::<i64, _>("output_tokens").max(0) as usize;
    target.system_tokens += row.get::<i64, _>("system_tokens").max(0) as usize;
    target.context_tokens += row.get::<i64, _>("context_tokens").max(0) as usize;
    target.tool_call_tokens += row.get::<i64, _>("tool_call_tokens").max(0) as usize;
    target.tool_result_tokens += row.get::<i64, _>("tool_result_tokens").max(0) as usize;
    target.memory_tokens += row.get::<i64, _>("memory_tokens").max(0) as usize;
    target.total_tokens = target.input_tokens.saturating_add(target.output_tokens);
    if let Ok(cache_read) = row.try_get::<i64, _>("cache_read_tokens") {
        if cache_read > 0 {
            target.cache_read_tokens = Some(
                target
                    .cache_read_tokens
                    .unwrap_or(0)
                    .saturating_add(cache_read as usize),
            );
        }
    }
    let accuracy = match row.get::<String, _>("accuracy").as_str() {
        "exact" => TokenAccuracy::Exact,
        "mixed" => TokenAccuracy::Mixed,
        _ => TokenAccuracy::Estimated,
    };
    target.accuracy = if was_empty {
        accuracy
    } else {
        target.accuracy.merge(accuracy)
    };
}

#[tauri::command]
pub async fn get_token_usage_report(
    state: State<'_, AppState>,
    request: TokenUsageReportRequest,
) -> Result<TokenUsageReport, String> {
    let now = Local::now().timestamp_millis();
    let from = request
        .from
        .unwrap_or_else(|| now - 30 * 24 * 60 * 60 * 1000);
    let to = request.to.unwrap_or(now + 1);
    let granularity = match request.granularity.as_str() {
        "week" | "month" => request.granularity,
        _ => "day".into(),
    };
    let rows = crate::core::chat::db::load_token_usage_records(
        &state.core.chat().conversation().db_pool(),
        from,
        to,
    )
    .await?;
    let mut total = TokenUsage::default();
    let mut models: BTreeMap<String, ModelUsage> = BTreeMap::new();
    let mut timeline: BTreeMap<i64, UsageBucket> = BTreeMap::new();

    for row in &rows {
        add_usage(&mut total, row);
        let model = row.get::<String, _>("model");
        let provider = row.try_get::<String, _>("provider").ok();
        let entry = models.entry(model.clone()).or_insert_with(|| ModelUsage {
            model: model.clone(),
            provider: provider.clone(),
            usage: TokenUsage::default(),
            calls: 0,
            share: 0.0,
        });
        add_usage(&mut entry.usage, row);
        entry.calls += 1;
        let start = bucket_start(row.get("recorded_at"), &granularity);
        let key = start.timestamp_millis();
        let bucket = timeline.entry(key).or_insert_with(|| UsageBucket {
            bucket: start.to_rfc3339(),
            label: match granularity.as_str() {
                "month" => start.format("%Y-%m").to_string(),
                _ => start.format("%m/%d").to_string(),
            },
            total_tokens: 0,
            input_tokens: 0,
            output_tokens: 0,
            models: BTreeMap::new(),
        });
        let tokens = row.get::<i64, _>("total_tokens").max(0) as usize;
        bucket.total_tokens += tokens;
        bucket.input_tokens += row.get::<i64, _>("input_tokens").max(0) as usize;
        bucket.output_tokens += row.get::<i64, _>("output_tokens").max(0) as usize;
        *bucket.models.entry(model).or_default() += tokens;
    }
    for model in models.values_mut() {
        model.share = if total.total_tokens == 0 {
            0.0
        } else {
            model.usage.total_tokens as f64 / total.total_tokens as f64
        };
    }
    let mut by_model = models.into_values().collect::<Vec<_>>();
    by_model.sort_by(|left, right| right.usage.total_tokens.cmp(&left.usage.total_tokens));
    Ok(TokenUsageReport {
        from,
        to,
        granularity,
        total,
        model_calls: rows.len(),
        by_model,
        timeline: timeline.into_values().collect(),
    })
}
