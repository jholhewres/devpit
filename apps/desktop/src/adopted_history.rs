//! What a session taken in from a terminal had said before it was taken.
//!
//! Adopting copied nothing: the first turn resumes the CLI session, and the
//! CLI knows its own past. The window did not — a resumed conversation opened
//! blank, which reads as "nothing was loaded" however right the next answer
//! is. So the words of the CLI's transcript are written into devpit's own,
//! once, while it is still empty: what each side said, not the tool calls in
//! between, which the CLI's transcript keeps in a shape this does not draw.

use std::path::Path;

use devpit_agentcli::store::append;
use devpit_agentcli::transcript_text::said_in;
use devpit_rpc::{Message, Part, Role};

/// Past this many messages, only the latest are brought in. A months-old
/// session can hold thousands, and the screen draws every one it is given.
const MOST: usize = 400;

/// The CLI's lines as messages: consecutive lines of one side joined, and the
/// person's bookkeeping — slash-command echoes, caveats, reminders — left out.
pub(crate) fn from_transcript(path: &Path, at: f64) -> Vec<Message> {
    let mut found: Vec<Message> = Vec::new();
    for said in said_in(path) {
        let role = if said.role == "user" {
            Role::User
        } else {
            Role::Assistant
        };
        if role == Role::User && (said.text.starts_with('<') || said.text.starts_with("Caveat:")) {
            continue;
        }
        if let Some(last) = found.last_mut().filter(|last| last.role == role) {
            if let Some(Part::Text { text, .. }) = last.parts.last_mut() {
                text.push_str("\n\n");
                text.push_str(&said.text);
                continue;
            }
        }
        found.push(Message {
            id: format!("msg_{}", ulid::Ulid::generate()),
            turn_id: None,
            role,
            parts: vec![Part::Text {
                text: said.text,
                parent: None,
            }],
            created_at: at,
            streaming: false,
        });
    }
    let skip = found.len().saturating_sub(MOST);
    found.split_off(skip)
}

/// Fills an empty conversation from the session it resumes, wherever an
/// installation holds that session for `root`. Answers what it wrote.
pub(crate) fn backfill(file: &Path, session_id: &str, root: &Path, at: f64) -> Vec<Message> {
    let folder = devpit_agentcli::outside::folder_name(root);
    let Some(path) = crate::installations::found()
        .unwrap_or_default()
        .into_iter()
        .map(|one| {
            one.directory
                .join("projects")
                .join(&folder)
                .join(format!("{session_id}.jsonl"))
        })
        .find(|path| path.is_file())
    else {
        return Vec::new();
    };
    let messages = from_transcript(&path, at);
    for message in &messages {
        if append(file, message).is_err() {
            break;
        }
    }
    messages
}

/// What `chat.history` shows: the conversation as kept, or — kept empty and
/// resuming a session — that session's words, written in on the way.
pub(crate) fn or_backfilled(
    messages: Vec<Message>,
    head: Option<&devpit_agentcli::head::Head>,
    project_id: &str,
    file: &Path,
) -> Vec<Message> {
    let Some((session, head)) = head.and_then(|head| Some((head.session_id.as_deref()?, head)))
    else {
        return messages;
    };
    if !messages.is_empty() {
        return messages;
    }
    let root = match &head.cwd {
        Some(cwd) => std::path::PathBuf::from(cwd),
        None => match crate::projects::store()
            .and_then(|store| crate::projects::locate(&store, project_id))
        {
            Ok((_, root)) => root,
            Err(_) => return messages,
        },
    };
    backfill(file, session, &root, head.created_at)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_of_each_side_come_in_and_the_bookkeeping_stays_out() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("s.jsonl");
        std::fs::write(
            &path,
            [
                r#"{"type":"user","message":{"role":"user","content":"<command-name>/clear</command-name>"}}"#,
                r#"{"type":"user","message":{"role":"user","content":"fix the build"}}"#,
                r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Looking."}]}}"#,
                r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"ok"}]}}"#,
                r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Fixed."}]}}"#,
            ]
            .join("\n"),
        )
        .expect("write");
        let messages = from_transcript(&path, 1.0);
        let said: Vec<(Role, String)> = messages
            .iter()
            .map(|one| match &one.parts[0] {
                Part::Text { text, .. } => (one.role, text.clone()),
                _ => panic!("text"),
            })
            .collect();
        assert_eq!(
            said,
            [
                (Role::User, "fix the build".to_owned()),
                (Role::Assistant, "Looking.\n\nFixed.".to_owned()),
            ]
        );
    }
}
