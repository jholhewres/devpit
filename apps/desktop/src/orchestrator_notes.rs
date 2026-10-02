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

/// The heading in `context/preferences.md` whose lines go with every brief.
const RULES: &str = "## rules for sessions";

/// The person's standing rules for every session the orchestrator starts:
/// the section under `## Rules for sessions` in its preferences, up to the
/// next heading of the same level. Nothing, when there is none.
pub(crate) fn rules(folder: &Path) -> Option<String> {
    let text = std::fs::read_to_string(log_file(folder, "preferences")?).ok()?;
    rules_in(&text)
}

pub(crate) fn rules_in(text: &str) -> Option<String> {
    let mut lines = text
        .lines()
        .skip_while(|line| line.trim().to_lowercase() != RULES);
    lines.next()?;
    let body: Vec<&str> = lines
        .take_while(|line| !line.starts_with("## ") && !line.starts_with("# "))
        .collect();
    let body = body.join("\n");
    let body = body.trim();
    (!body.is_empty()).then(|| body.chars().take(LONGEST).collect())
}

/// The longest brief a session is started with, rules included.
const LONGEST_BRIEF: usize = 8000;

/// A session's brief, with the person's standing rules after it — cut to
/// what still fits, so the rules never make a brief too long to start.
pub(crate) fn briefed(folder: &Path, prompt: &str) -> String {
    let prompt_chars = prompt.trim_end().chars().count();
    match rules(folder) {
        Some(rules) if !prompt.trim().is_empty() && prompt_chars + 64 < LONGEST_BRIEF => {
            let room = LONGEST_BRIEF - prompt_chars - 64;
            let rules: String = rules.chars().take(room).collect();
            format!("{}\n\n## Rules for sessions\n\n{rules}", prompt.trim_end())
        }
        _ => prompt.to_owned(),
    }
}

#[cfg(test)]
#[path = "orchestrator_notes_tests.rs"]
mod tests;
