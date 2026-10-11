//! What a slow check last said, kept so a reader never waits on it.
//!
//! `claude mcp list` connects to every server before it answers, which takes
//! seconds. The footer asks it on a timer and on demand; a session's start
//! only reads what is kept, and when that is old or missing, asks again
//! behind itself for the next one.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long an answer is taken as current: the footer asks a little more often.
pub(crate) const FRESH_FOR: Duration = Duration::from_secs(10 * 60);

/// What is kept for one key.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Known<T> {
    Fresh(T),
    /// Older than it should be: still the best word there is.
    Stale(T),
    Unknown,
}

impl<T> Known<T> {
    pub(crate) fn value(self) -> Option<T> {
        match self {
            Known::Fresh(value) | Known::Stale(value) => Some(value),
            Known::Unknown => None,
        }
    }
}

struct Entry<T> {
    kept: Option<(Instant, T)>,
    asking: bool,
}

/// Answers by key, each with when it was had.
pub(crate) struct Kept<T> {
    entries: Arc<Mutex<HashMap<String, Entry<T>>>>,
}

impl<T> Default for Kept<T> {
    fn default() -> Self {
        Self {
            entries: Arc::default(),
        }
    }
}

impl<T: Clone + Send + 'static> Kept<T> {
    /// What is kept for `key`, as of `fresh_for`.
    pub(crate) fn known(&self, key: &str, fresh_for: Duration) -> Known<T> {
        let Ok(entries) = self.entries.lock() else {
            return Known::Unknown;
        };
        match entries.get(key).and_then(|entry| entry.kept.as_ref()) {
            Some((at, value)) if at.elapsed() < fresh_for => Known::Fresh(value.clone()),
            Some((_, value)) => Known::Stale(value.clone()),
            None => Known::Unknown,
        }
    }

    pub(crate) fn put(&self, key: &str, value: T) {
        if let Ok(mut entries) = self.entries.lock() {
            let entry = entries.entry(key.to_owned()).or_insert(Entry {
                kept: None,
                asking: false,
            });
            entry.kept = Some((Instant::now(), value));
        }
    }

    /// What is kept, read without waiting. When it is old or missing, `ask`
    /// runs on a thread of its own and its answer is kept for the next reader;
    /// one ask per key at a time, and a failed ask keeps the old answer.
    pub(crate) fn read_and_refresh(
        &self,
        key: &str,
        fresh_for: Duration,
        ask: impl FnOnce() -> Option<T> + Send + 'static,
    ) -> Known<T> {
        let known = self.known(key, fresh_for);
        if matches!(known, Known::Fresh(_)) || !self.begin(key) {
            return known;
        }
        let entries = Arc::clone(&self.entries);
        let key = key.to_owned();
        std::thread::spawn(move || {
            let answer = ask();
            if let Ok(mut entries) = entries.lock() {
                if let Some(entry) = entries.get_mut(&key) {
                    entry.asking = false;
                    if let Some(value) = answer {
                        entry.kept = Some((Instant::now(), value));
                    }
                }
            }
        });
        known
    }

    /// Marks `key` as being asked; false when it already was.
    fn begin(&self, key: &str) -> bool {
        let Ok(mut entries) = self.entries.lock() else {
            return false;
        };
        let entry = entries.entry(key.to_owned()).or_insert(Entry {
            kept: None,
            asking: false,
        });
        !std::mem::replace(&mut entry.asking, true)
    }
}

#[cfg(test)]
#[path = "mcp_cache_tests.rs"]
mod tests;
