//! What a transcript says an agent spent, one assistant message at a time.
//!
//! The CLI writes the same message's usage on several lines — one per content
//! block — so a message is counted once, by its id and the request that
//! produced it. Counting lines instead reads about twice what was spent.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::Value;

/// One assistant message's usage, as the transcript recorded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// What makes two lines the same message.
    pub key: String,
    /// Unix seconds.
    pub at: i64,
    pub session_id: String,
    pub cwd: String,
    pub model: String,
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write_5m: u64,
    pub cache_write_1h: u64,
}

impl Record {
    pub fn tokens(&self) -> u64 {
        self.input + self.output + self.cache_read + self.cache_write_5m + self.cache_write_1h
    }
}

/// Every assistant message with usage in a transcript's text. Lines that are
/// not one, or that carry no tokens at all, are left out.
pub fn records_in(text: &str) -> Vec<Record> {
    text.lines()
        .filter(|line| line.contains("\"assistant\""))
        .filter_map(record_of)
        .filter(|record| record.tokens() > 0)
        .collect()
}

fn record_of(line: &str) -> Option<Record> {
    let value: Value = serde_json::from_str(line).ok()?;
    if value.get("type")?.as_str()? != "assistant" {
        return None;
    }
    let message = value.get("message")?;
    let usage = message.get("usage")?;
    let count = |field: Option<&Value>| field.and_then(Value::as_u64).unwrap_or(0);
    let written = count(usage.get("cache_creation_input_tokens"));
    let one_hour = count(usage.pointer("/cache_creation/ephemeral_1h_input_tokens"));
    let five_minutes = usage
        .pointer("/cache_creation/ephemeral_5m_input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| written.saturating_sub(one_hour));
    let text = |from: &Value, field: &str| {
        from.get(field)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let key = match (
        message.get("id").and_then(Value::as_str),
        value.get("requestId").and_then(Value::as_str),
    ) {
        (Some(id), Some(request)) => format!("{id}:{request}"),
        (Some(id), None) => format!("msg:{id}"),
        _ => format!("uuid:{}", value.get("uuid")?.as_str()?),
    };
    Some(Record {
        key,
        at: epoch_of(value.get("timestamp")?.as_str()?)?,
        session_id: text(&value, "sessionId"),
        cwd: text(&value, "cwd"),
        model: text(message, "model"),
        input: count(usage.get("input_tokens")),
        output: count(usage.get("output_tokens")),
        cache_read: count(usage.get("cache_read_input_tokens")),
        cache_write_5m: five_minutes,
        cache_write_1h: one_hour,
    })
}

/// Every transcript under an installation's `projects`, subagents included,
/// written to since `since`. Symlinked folders are not followed.
pub fn transcripts(config_dir: &Path, since: SystemTime) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut folders = vec![config_dir.join("projects")];
    while let Some(folder) = folders.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                folders.push(path);
            } else if kind.is_file()
                && path.extension().is_some_and(|ext| ext == "jsonl")
                && entry
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .is_ok_and(|modified| modified >= since)
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Unix seconds of an RFC 3339 time: `Z` or a `±HH:MM` offset.
pub fn epoch_of(iso: &str) -> Option<i64> {
    let number = |at: std::ops::Range<usize>| iso.get(at)?.parse::<i64>().ok();
    let local = days_from_civil(number(0..4)?, number(5..7)?, number(8..10)?) * 86_400
        + number(11..13)? * 3_600
        + number(14..16)? * 60
        + number(17..19)?;
    let tail = iso.get(19..).unwrap_or_default();
    let offset = match tail.rfind(['+', '-']) {
        Some(at) if tail.len() - at == 6 => {
            let sign = if tail.as_bytes()[at] == b'-' { -1 } else { 1 };
            let hours = tail.get(at + 1..at + 3)?.parse::<i64>().ok()?;
            let minutes = tail.get(at + 4..at + 6)?.parse::<i64>().ok()?;
            sign * (hours * 3_600 + minutes * 60)
        }
        _ => 0,
    };
    Some(local - offset)
}

/// The UTC day of a Unix time, as `YYYY-MM-DD`.
pub fn day_of(epoch: i64) -> String {
    let (year, month, day) = civil_from_days(epoch.div_euclid(86_400));
    format!("{year:04}-{month:02}-{day:02}")
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_cycle - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let of_cycle = days - era * 146_097;
    let of_era = (of_cycle - of_cycle / 1_460 + of_cycle / 36_524 - of_cycle / 146_096) / 365;
    let of_year = of_cycle - (365 * of_era + of_era / 4 - of_era / 100);
    let shifted = (5 * of_year + 2) / 153;
    let day = of_year - (153 * shifted + 2) / 5 + 1;
    let month = if shifted < 10 {
        shifted + 3
    } else {
        shifted - 9
    };
    let year = of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
#[path = "spend_scan_tests.rs"]
mod tests;
