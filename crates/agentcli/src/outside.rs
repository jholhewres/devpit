//! Sessions the CLI holds for a project that devpit never saw.
//!
//! A conversation started in a terminal is written by the CLI under its own
//! configuration, and devpit's chat list had no way to reach it. Measured on
//! Claude Code 2.1.270: the folder is the project root with every `/` and `.`
//! turned into `-`, one `<session>.jsonl` per session, and the title on lines
//! of type `ai-title` — repeated, one file carried it 2,899 times.

use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

/// A line longer than this is not a title line, and is skipped without being
/// held: a transcript line carrying a tool's whole output runs to megabytes.
const MOST_LINE_BYTES: usize = 64 * 1024;

/// One session a CLI installation holds.
#[derive(Debug, Clone, PartialEq)]
pub struct Outside {
    pub session_id: String,
    /// The CLI's own title, when it wrote one.
    pub title: Option<String>,
    /// The installation's configuration directory.
    pub installation: PathBuf,
    /// Unix seconds, from the transcript's mtime.
    pub last_at: f64,
}

/// The folder name the CLI gives a project root: every character that is not
/// an ASCII letter or digit becomes `-`, so `_`, spaces and `@` go too.
pub fn folder_name(root: &Path) -> String {
    root.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Reads one line into `into`, keeping at most the ceiling. Answers the line's
/// full length, or `None` at the end of the file.
pub(crate) fn bounded_line(
    reader: &mut impl BufRead,
    into: &mut Vec<u8>,
    most: usize,
) -> Option<usize> {
    into.clear();
    let mut total = 0;
    loop {
        let chunk = reader.fill_buf().ok()?;
        if chunk.is_empty() {
            return (total > 0).then_some(total);
        }
        let (take, done) = match chunk.iter().position(|&byte| byte == b'\n') {
            Some(at) => (at + 1, true),
            None => (chunk.len(), false),
        };
        if into.len() < most {
            let room = most - into.len();
            into.extend_from_slice(&chunk[..take.min(room)]);
        }
        total += take;
        reader.consume(take);
        if done {
            return Some(total);
        }
    }
}

/// The last title the CLI wrote into a transcript.
pub fn last_title(reader: impl Read) -> Option<String> {
    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();
    let mut title = None;
    while let Some(length) = bounded_line(&mut reader, &mut line, MOST_LINE_BYTES) {
        if length > MOST_LINE_BYTES || !line.windows(10).any(|w| w == b"\"ai-title\"") {
            continue;
        }
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&line) else {
            continue;
        };
        if value.get("type").and_then(|kind| kind.as_str()) == Some("ai-title") {
            if let Some(said) = value.get("aiTitle").and_then(|said| said.as_str()) {
                title = Some(said.to_owned());
            }
        }
    }
    title
}

fn seconds(path: &Path) -> f64 {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|since| since.as_secs() as f64)
        .unwrap_or_default()
}

/// Every session the installations hold for `root` that `known` does not name,
/// most recent first.
pub fn sessions(installations: &[PathBuf], root: &Path, known: &HashSet<String>) -> Vec<Outside> {
    let folder = folder_name(root);
    let mut found: Vec<Outside> = installations
        .iter()
        .flat_map(|installation| {
            let dir = installation.join("projects").join(&folder);
            std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
                .filter_map(|path| {
                    let session_id = path.file_stem()?.to_string_lossy().into_owned();
                    if known.contains(&session_id) {
                        return None;
                    }
                    Some(Outside {
                        title: std::fs::File::open(&path).ok().and_then(last_title),
                        last_at: seconds(&path),
                        installation: installation.clone(),
                        session_id,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect();
    found.sort_by(|a, b| b.last_at.total_cmp(&a.last_at));
    found
}

#[cfg(test)]
#[path = "outside_tests.rs"]
mod tests;
