//! An orchestrator's log, written by devpit for it.
//!
//! Its brief asks it to keep `context/sessions.md` and `context/preferences.md`
//! up to date, and it did — with `echo >>` and heredocs, a hundred times a
//! week. One call does the same: a dated line, appended, nothing else touched.

use std::io::Write as _;
use std::path::{Path, PathBuf};

/// The logs an orchestrator keeps, by the name a call gives.
const LOGS: [&str; 2] = ["sessions", "preferences"];
/// How long one entry may be, in characters.
const LONGEST: usize = 4000;

/// The file a log name stands for, inside the orchestrator's folder. Only
/// the names above: the name comes from the agent and is never a path.
pub(crate) fn log_file(folder: &Path, log: &str) -> Option<PathBuf> {
    LOGS.contains(&log)
        .then(|| folder.join("context").join(format!("{log}.md")))
}

/// One entry as it is written: a dated bullet, its later lines indented
/// under it.
pub(crate) fn entry(day: &str, text: &str) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() || text.chars().count() > LONGEST {
        return Err(format!("an entry is between 1 and {LONGEST} characters"));
    }
    if text
        .chars()
        .any(|ch| ch.is_control() && ch != '\n' && ch != '\t')
    {
        return Err("an entry cannot hold control characters".to_owned());
    }
    let body = text.lines().collect::<Vec<_>>().join("\n  ");
    Ok(format!("- {day} — {body}\n"))
}

/// Appends an entry to one of the orchestrator's logs; answers what it wrote.
pub(crate) fn note(folder: &Path, log: &str, text: &str) -> Result<String, String> {
    let file =
        log_file(folder, log).ok_or_else(|| format!("the logs are {}", LOGS.join(" and ")))?;
    let day = devpit_agentcli::spend_scan::day_of(devpit_core::reports::now() as i64);
    let line = entry(&day, text)?;
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
        .and_then(|mut opened| opened.write_all(line.as_bytes()))
        .map_err(|err| err.to_string())?;
    Ok(line)
}

#[cfg(test)]
#[path = "orchestrator_notes_tests.rs"]
mod tests;
