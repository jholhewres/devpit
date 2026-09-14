//! What was said in a CLI transcript, for the search index.
//!
//! The words a person typed and the words the agent answered with. Tool
//! output, thinking, attachments and bookkeeping stay out: measured on the
//! machine this was written on, attachments alone were 152 of every 245
//! thousand lines, and nobody remembers a conversation by any of them.
//!
//! Here and not in the store because the line format is the CLI's, and this
//! crate is where the CLI's undocumented shapes are allowed to live.

use std::path::Path;

/// A line longer than this is skipped without being held: the longest lines
/// are tool results, measured at up to 1.2 MB.
pub const MOST_LINE_BYTES: usize = 256 * 1024;

/// The searchable words on one line.
#[derive(Debug, Clone, PartialEq)]
pub struct Said {
    /// `user` or `assistant`.
    pub role: &'static str,
    pub text: String,
}

/// The words on one transcript line, when it carries any worth finding.
pub fn said_on(line: &str) -> Option<Said> {
    if line.len() > MOST_LINE_BYTES {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    let role = match value.get("type")?.as_str()? {
        "user" => "user",
        "assistant" => "assistant",
        _ => return None,
    };
    let content = value.get("message")?.get("content")?;
    let text = match content {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Array(blocks) => blocks
            .iter()
            .filter(|block| block.get("type").and_then(|kind| kind.as_str()) == Some("text"))
            .filter_map(|block| block.get("text").and_then(|text| text.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => return None,
    };
    let text = text.trim();
    (!text.is_empty()).then(|| Said {
        role,
        text: text.to_owned(),
    })
}

/// Every searchable line of a transcript, read without holding any line past
/// the ceiling.
pub fn said_in(path: &Path) -> Vec<Said> {
    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let mut reader = std::io::BufReader::new(file);
    let mut line = Vec::new();
    let mut found = Vec::new();
    while let Some(length) = crate::outside::bounded_line(&mut reader, &mut line, MOST_LINE_BYTES) {
        if length > MOST_LINE_BYTES {
            continue;
        }
        if let Some(said) = std::str::from_utf8(&line).ok().and_then(said_on) {
            found.push(said);
        }
    }
    found
}

#[cfg(test)]
#[path = "transcript_text_tests.rs"]
mod tests;
