//! Bot and scheduled-task shapes, plus when the next run is due.

use chrono::{DateTime, Datelike, Days, Local, NaiveTime, Timelike};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Bot {
    pub id: String,
    pub name: String,
    /// Kept for older bots. New bots learn their job from the conversation.
    #[serde(default)]
    pub persona: String,
    /// Data-URI image chosen when the bot was created.
    #[serde(default)]
    pub avatar: String,
    pub session_id: String,
    pub created_at: i64,
}

/// When a task should run. Times are the machine's local clock.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ScheduleSpec {
    Once { at_ms: i64 },
    Interval { every_minutes: u32 },
    Daily { time: String },
    Weekly { weekday: u8, time: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTask {
    pub id: String,
    pub bot_id: String,
    pub title: String,
    pub instruction: String,
    pub spec: ScheduleSpec,
    pub enabled: bool,
    pub next_run_ms: i64,
    pub last_run_ms: Option<i64>,
    pub last_status: String,
    pub last_result: String,
    pub created_at: i64,
}

/// Next instant strictly after `after_ms`, except `once`, which stays at `at_ms`.
pub fn next_run_after(spec: &ScheduleSpec, after_ms: i64) -> Result<i64, String> {
    match spec {
        ScheduleSpec::Once { at_ms } => Ok(*at_ms),
        ScheduleSpec::Interval { every_minutes } => {
            let minutes = (*every_minutes).clamp(1, 7 * 24 * 60) as i64;
            Ok(after_ms.saturating_add(minutes * 60_000))
        }
        ScheduleSpec::Daily { time } => next_clock(after_ms, None, time),
        ScheduleSpec::Weekly { weekday, time } => next_clock(after_ms, Some(*weekday), time),
    }
}

pub fn validate_spec(spec: &ScheduleSpec) -> Result<(), String> {
    match spec {
        ScheduleSpec::Once { at_ms } if *at_ms <= 0 => Err("atMs must be a unix millisecond timestamp".into()),
        ScheduleSpec::Once { .. } => Ok(()),
        ScheduleSpec::Interval { every_minutes } if *every_minutes == 0 => {
            Err("everyMinutes must be at least 1".into())
        }
        ScheduleSpec::Interval { .. } => Ok(()),
        ScheduleSpec::Daily { time } => {
            parse_hhmm(time)?;
            Ok(())
        }
        ScheduleSpec::Weekly { weekday, time } => {
            if *weekday > 6 {
                return Err("weekday must be 0 (Monday) through 6 (Sunday)".into());
            }
            parse_hhmm(time)?;
            Ok(())
        }
    }
}

fn parse_hhmm(time: &str) -> Result<NaiveTime, String> {
    let mut parts = time.trim().split(':');
    let hour: u32 = parts
        .next()
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| "time must be HH:MM".to_string())?;
    let minute: u32 = parts
        .next()
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| "time must be HH:MM".to_string())?;
    if parts.next().is_some() || hour > 23 || minute > 59 {
        return Err("time must be HH:MM".into());
    }
    NaiveTime::from_hms_opt(hour, minute, 0).ok_or_else(|| "time must be HH:MM".to_string())
}

fn next_clock(after_ms: i64, weekday: Option<u8>, time: &str) -> Result<i64, String> {
    let clock = parse_hhmm(time)?;
    let after = millis_to_local(after_ms);
    let mut day = after.date_naive();
    for _ in 0..8 {
        let candidate = day.and_time(clock);
        if let Some(dt) = candidate.and_local_timezone(Local).earliest() {
            let day_ok = weekday
                .map(|wanted| dt.weekday().num_days_from_monday() == u32::from(wanted))
                .unwrap_or(true);
            if day_ok && dt > after {
                return Ok(dt.timestamp_millis());
            }
        }
        day = day.checked_add_days(Days::new(1)).unwrap_or(day);
    }
    Err("could not find a future run time".into())
}

pub fn millis_to_local(ms: i64) -> DateTime<Local> {
    DateTime::from_timestamp_millis(ms)
        .map(|utc| utc.with_timezone(&Local))
        .unwrap_or_else(Local::now)
}

pub fn now_ms() -> i64 {
    Local::now().timestamp_millis()
}

pub fn truncate_chars(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_steps_forward_from_now() {
        let spec = ScheduleSpec::Interval { every_minutes: 15 };
        assert_eq!(next_run_after(&spec, 1_000).unwrap(), 1_000 + 15 * 60_000);
    }

    #[test]
    fn once_keeps_its_timestamp() {
        let spec = ScheduleSpec::Once { at_ms: 50 };
        assert_eq!(next_run_after(&spec, 1_000).unwrap(), 50);
    }

    #[test]
    fn daily_lands_on_the_requested_clock() {
        let spec = ScheduleSpec::Daily {
            time: "09:30".into(),
        };
        let now = now_ms();
        let next = next_run_after(&spec, now).unwrap();
        assert!(next > now);
        let local = millis_to_local(next);
        assert_eq!(local.hour(), 9);
        assert_eq!(local.minute(), 30);
    }

    #[test]
    fn weekly_rejects_a_bad_weekday() {
        let spec = ScheduleSpec::Weekly {
            weekday: 9,
            time: "08:00".into(),
        };
        assert!(validate_spec(&spec).is_err());
    }
}
