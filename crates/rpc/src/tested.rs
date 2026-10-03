//! A test run's report, kept as the run's evidence. Read by
//! `devpit_steps::report`; counts never come from the exit code.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Evidence version of a test report (a review is [`crate::REVIEW_EVIDENCE`]).
pub const TESTS_EVIDENCE: i64 = 2;

/// Failures kept by name; `failed` stays exact past it.
pub const MOST_FAILURES_KEPT: usize = 50;

/// One test that failed, as the report named it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FailedTest {
    pub name: String,
    /// Relative to where the run worked when it was under it.
    pub file: Option<String>,
    /// The first line of what it said, cut short.
    pub message: Option<String>,
}

/// What every report a run left said, added up.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Tested {
    pub passed: u32,
    /// Tests that failed, and files that failed before any test in them ran.
    pub failed: u32,
    pub skipped: u32,
    /// At most [`MOST_FAILURES_KEPT`]; `failed` says how many there were.
    pub failures: Vec<FailedTest>,
    /// Formats read: `vitest`, `cargo test`, `JUnit`.
    pub read_from: Vec<String>,
}

impl Tested {
    /// Adds another report's counts and failures to these.
    pub fn add(&mut self, other: Tested) {
        self.passed += other.passed;
        self.failed += other.failed;
        self.skipped += other.skipped;
        let room = MOST_FAILURES_KEPT.saturating_sub(self.failures.len());
        self.failures.extend(other.failures.into_iter().take(room));
        for format in other.read_from {
            if !self.read_from.contains(&format) {
                self.read_from.push(format);
            }
        }
    }
}
