//! The conversations a project has had.
//!
//! The transcripts were already on disk and nothing listed them: closing a
//! tab left a file nobody could reach again. This is the index over them.

use std::path::Path;

use crate::head::{read_head, Head};

/// One conversation, as a row in a list.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub id: String,
    /// The first thing the person said, which is what they will recognise.
    pub title: String,
    /// The account it belongs to, and cannot leave.
    pub profile: String,
    pub model: Option<String>,
    pub cost_usd: f64,
    /// Unix seconds, from the file's own mtime — when it was last spoken in.
    pub last_at: f64,
}

/// A conversation's name, from the first thing the person said.
///
/// Their own words rather than a generated summary: the point of the row is
/// that they recognise it, and a title written by a model is one more thing
/// that can be wrong about a conversation they remember perfectly.
pub fn title_of(said: &str) -> String {
    let first = said
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    let short: String = first.chars().take(72).collect();
    if short.is_empty() {
        "Untitled".to_owned()
    } else if first.chars().count() > 72 {
        format!("{}…", short.trim_end())
    } else {
        short
    }
}

/// The first user message in a transcript, without reading the whole file.
///
/// One line is enough: the transcript is one message per line and the person
/// spoke first. A conversation with hundreds of turns costs the same as one
/// with a single turn.
fn opened_with(path: &Path) -> Option<String> {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path).ok()?;
    for line in BufReader::new(file).lines().map_while(Result::ok).take(8) {
        let message: devpit_rpc::Message = serde_json::from_str(&line).ok()?;
        if message.role != devpit_rpc::Role::User {
            continue;
        }
        return Some(
            message
                .parts
                .iter()
                .filter_map(|part| match part {
                    // A subagent's words are not the conversation's.
                    devpit_rpc::Part::Text { text, parent: None } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    None
}

fn seconds(path: &Path) -> f64 {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|since| since.as_secs() as f64)
        .unwrap_or_default()
}

/// Every conversation in a project's `sessions`, most recently spoken in first.
pub fn conversations(dir: &Path) -> Vec<Summary> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut found: Vec<Summary> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .filter_map(|path| {
            let id = path.file_stem()?.to_string_lossy().into_owned();
            let head: Head = read_head(&dir.join(format!("{id}.json"))).unwrap_or(Head {
                profile: String::new(),
                model: None,
                created_at: 0.0,
                cost_usd: 0.0,
                budget_usd: None,
                session_id: None,
                permission: None,
                effort: None,
                title: None,
                rewind: Default::default(),
                cwd: None,
                context: None,
            });
            Some(Summary {
                // The person's own words first — the decision above — and the
                // CLI's title only for a session that has none of them.
                title: opened_with(&path)
                    .filter(|said| !said.trim().is_empty())
                    .map(|said| title_of(&said))
                    .or_else(|| head.title.clone())
                    .unwrap_or_else(|| "Untitled".to_owned()),
                last_at: seconds(&path),
                profile: head.profile,
                model: head.model,
                cost_usd: head.cost_usd,
                id,
            })
        })
        .collect();

    found.sort_by(|a, b| b.last_at.total_cmp(&a.last_at));
    found
}
