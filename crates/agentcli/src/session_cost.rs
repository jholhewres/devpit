//! What one session has spent so far, read off its transcript as it grows.
//!
//! From where the last read stopped, never the whole file again: a session
//! hours long is megabytes, and it is asked about every few seconds.

use std::collections::HashSet;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::spend_prices::cost_of;
use crate::spend_scan::record_of;

/// A transcript's spend, as far as it has been read.
#[derive(Debug, Default, Clone)]
pub struct Tally {
    offset: u64,
    /// A line still being written when it was read, kept for the next read.
    partial: String,
    seen: HashSet<String>,
    pub cost_usd: f64,
    /// Spent since the person last wrote to it.
    pub last_turn_usd: f64,
    /// Spent before devpit first read it: a resumed session's history.
    pub before_usd: Option<f64>,
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub model: Option<String>,
    /// Tokens of a model there is no price for.
    pub unpriced: u64,
}

impl Tally {
    /// Reads what was added since the last call. A file that got shorter was
    /// replaced, and is read again from the start.
    pub fn advance(&mut self, path: &Path) -> std::io::Result<()> {
        let mut file = std::fs::File::open(path)?;
        let size = file.metadata()?.len();
        if size < self.offset {
            *self = Tally::default();
        }
        file.seek(SeekFrom::Start(self.offset))?;
        let mut added = Vec::new();
        file.take(size - self.offset).read_to_end(&mut added)?;
        self.offset = size;
        let text = std::mem::take(&mut self.partial) + &String::from_utf8_lossy(&added);
        let (whole, rest) = match text.rfind('\n') {
            Some(at) => text.split_at(at + 1),
            None => ("", text.as_str()),
        };
        self.partial = rest.to_owned();
        let first = self.before_usd.is_none();
        for line in whole.lines() {
            self.line(line);
        }
        if first {
            self.before_usd = Some(self.cost_usd);
        }
        Ok(())
    }

    fn line(&mut self, line: &str) {
        if prompted(line) {
            self.last_turn_usd = 0.0;
            return;
        }
        let Some(record) = record_of(line) else {
            return;
        };
        if record.tokens() == 0 || !self.seen.insert(record.key.clone()) {
            return;
        }
        match cost_of(&record) {
            Some(cost) => {
                self.cost_usd += cost;
                self.last_turn_usd += cost;
            }
            None => self.unpriced += record.tokens(),
        }
        self.input += record.input;
        self.output += record.output;
        self.cache_read += record.cache_read;
        self.cache_write += record.cache_write_5m + record.cache_write_1h;
        self.model = Some(record.model);
    }
}

/// A line where the person wrote, not a tool's result coming back.
fn prompted(line: &str) -> bool {
    if !line.contains("\"user\"") {
        return false;
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return false;
    };
    if value.get("type").and_then(|kind| kind.as_str()) != Some("user") {
        return false;
    }
    match value.pointer("/message/content") {
        Some(serde_json::Value::String(_)) => true,
        Some(serde_json::Value::Array(blocks)) => blocks
            .iter()
            .any(|block| block.get("type").and_then(|kind| kind.as_str()) == Some("text")),
        _ => false,
    }
}

#[cfg(test)]
#[path = "session_cost_tests.rs"]
mod tests;
