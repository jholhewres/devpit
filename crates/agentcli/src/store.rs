//! Where a conversation lives on disk.
//!
//! JSONL, one message a line: a crash mid-turn loses the line it was writing
//! and nothing else, and appending never rewrites what is already there.

use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use devpit_rpc::Message;

pub fn conversation_path(home: &Path, project_id: &str, conversation_id: &str) -> PathBuf {
    home.join("projects")
        .join(project_id)
        .join("sessions")
        .join(format!("{conversation_id}.jsonl"))
}

pub fn append(path: &Path, message: &Message) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(message)?)
}

/// Every message that loads, and the count of lines that did not.
///
/// One bad line must not take the conversation with it: the reader keeps
/// going and says how many it skipped.
pub fn read(path: &Path) -> (Vec<Message>, usize) {
    let Ok(file) = File::open(path) else {
        return (Vec::new(), 0);
    };
    let mut messages = Vec::new();
    let mut skipped = 0;
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Message>(&line) {
            Ok(message) => messages.push(message),
            Err(_) => skipped += 1,
        }
    }
    (messages, skipped)
}
