//! Reading what a session cost, from the transcript it leaves on disk.
//!
//! A headless step reports its own cost on the way out. A session someone
//! drove by hand reports nothing — there is no stream to read. Without this,
//! the card would go quiet for exactly the sessions that did the most work.
//!
//! The transcript is a JSONL file whose lines are typed. One of them,
//! `cost-state`, is the running total for the session.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// What a session has spent, and how much it changed.
#[derive(Debug, Clone, PartialEq)]
pub struct Cost {
    pub total_usd: f64,
    pub total_duration_ms: i64,
    pub lines_added: i64,
    pub lines_removed: i64,
}

/// Where a session's transcript lives.
///
/// The directory is the working directory with every character that is not a
/// letter or digit replaced by `-`. A path with dots, slashes and underscores
/// all collapse the same way.
pub fn transcript_path(home: &Path, cwd: &Path, session_id: &str) -> PathBuf {
    let slug: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    home.join(".claude")
        .join("projects")
        .join(slug)
        .join(format!("{session_id}.jsonl"))
}

#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default, rename = "totalCostUSD")]
    total_cost_usd: f64,
    #[serde(default, rename = "totalDuration")]
    total_duration: i64,
    #[serde(default, rename = "totalLinesAdded")]
    lines_added: i64,
    #[serde(default, rename = "totalLinesRemoved")]
    lines_removed: i64,
}

/// The last cost line of a transcript, or `None` when there is not one yet.
///
/// A session that has only just started has no cost line, and that is not a
/// failure — it is a session that has not spent anything. A line that does not
/// parse is skipped rather than fatal: a truncated write at the end of the
/// file must not hide the totals before it.
pub fn read_cost(path: &Path) -> Option<Cost> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines()
        .filter_map(|line| serde_json::from_str::<Line>(line).ok())
        .rfind(|line| line.kind == "cost-state")
        .map(|line| Cost {
            total_usd: line.total_cost_usd,
            total_duration_ms: line.total_duration,
            lines_added: line.lines_added,
            lines_removed: line.lines_removed,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shaped after a real transcript, including the lines we ignore.
    const TRANSCRIPT: &str = r#"{"type":"user","message":{}}
{"type":"ai-title","aiTitle":"fix the parser"}
{"type":"cost-state","totalCostUSD":1.5,"totalDuration":1000,"totalLinesAdded":3,"totalLinesRemoved":1}
{"type":"assistant","message":{}}
{"type":"cost-state","totalCostUSD":4.9830205,"totalDuration":1213380,"totalLinesAdded":12,"totalLinesRemoved":4}
"#;

    fn written(contents: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("session.jsonl");
        std::fs::write(&path, contents).expect("write");
        (dir, path)
    }

    /// The running total is the last one, not the first.
    #[test]
    fn the_latest_total_is_the_one_reported() {
        let (_dir, path) = written(TRANSCRIPT);
        let cost = read_cost(&path).expect("a cost line");
        assert!((cost.total_usd - 4.9830205).abs() < 1e-9);
        assert_eq!(cost.total_duration_ms, 1_213_380);
        assert_eq!(cost.lines_added, 12);
    }

    /// A session that just started has spent nothing. Not an error.
    #[test]
    fn a_transcript_without_a_cost_line_reports_nothing() {
        let (_dir, path) = written("{\"type\":\"user\",\"message\":{}}\n");
        assert_eq!(read_cost(&path), None);
    }

    /// A write cut off mid-line must not hide the totals before it.
    #[test]
    fn a_truncated_last_line_does_not_lose_the_rest() {
        let (_dir, path) = written(&format!("{TRANSCRIPT}{{\"type\":\"cost-st"));
        let cost = read_cost(&path).expect("a cost line");
        assert!((cost.total_usd - 4.9830205).abs() < 1e-9);
    }

    #[test]
    fn a_missing_transcript_reports_nothing() {
        assert_eq!(read_cost(Path::new("/nonexistent/session.jsonl")), None);
    }

    #[test]
    fn the_path_collapses_everything_that_is_not_alphanumeric() {
        let path = transcript_path(
            Path::new("/home/x"),
            Path::new("/home/x/Work/my.app-2"),
            "abc",
        );
        assert_eq!(
            path,
            Path::new("/home/x/.claude/projects/-home-x-Work-my-app-2/abc.jsonl")
        );
    }
}
