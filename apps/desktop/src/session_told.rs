//! What a session said, read from its own transcript.
//!
//! The screen holds the last lines of a terminal, and nothing at all for a
//! session outside devpit's terminals — so an orchestrator asking "what did it
//! end on" was reading transcript files by hand. This reads the same file the
//! CLI writes, by the session's id, and answers with the replies and prompts.

use std::path::{Path, PathBuf};

use devpit_rpc::{Message, Part, Role, RpcError};
use serde::Deserialize;
use serde_json::{json, Value};

/// How much of one reply or prompt comes back, in characters.
const REPLY: usize = 2000;
const PROMPT: usize = 600;
/// The most replies asked for at once.
const MOST: usize = 10;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Listed {
    pid: Option<i32>,
    name: Option<String>,
    session_id: Option<String>,
    cwd: Option<String>,
}

/// The transcript of the session called `name` under this profile: the live
/// one, or the latest that ran here and ended.
pub(crate) fn transcript_of(profile_id: &str, name: &str) -> Result<Option<PathBuf>, RpcError> {
    let config = crate::live_sessions::config_of(profile_id)?;
    let found = std::fs::read_dir(config.join("sessions"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter(|entry| entry.metadata().is_ok_and(|meta| meta.len() <= 64 * 1024))
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|text| serde_json::from_str::<Listed>(&text).ok())
        .find(|one| {
            one.name.as_deref() == Some(name) && one.pid.is_some_and(crate::live_sessions::alive)
        });
    let live = found.and_then(|one| Some((one.session_id?, one.cwd?)));
    let ended = || {
        crate::projects::store()
            .ok()?
            .seen_session(profile_id, name)
            .ok()
            .flatten()
            .map(|row| (row.session_id, row.cwd))
    };
    Ok(live.or_else(ended).and_then(|(id, cwd)| {
        crate::adopting::plain(&id)
            .then(|| devpit_agentcli::transcript_path(&config, Path::new(&cwd), &id))
    }))
}

fn text_of(message: &Message) -> String {
    message
        .parts
        .iter()
        .filter_map(|part| match part {
            Part::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn cut(said: &str, most: usize) -> String {
    match said.char_indices().nth(most) {
        Some((at, _)) => format!("{}…", &said[..at]),
        None => said.to_owned(),
    }
}

/// The latest replies and prompts of a conversation, oldest first.
pub(crate) fn told(messages: &[Message], last: usize) -> Value {
    let last = last.clamp(1, MOST);
    let of = |role: Role, keep: usize, most: usize| -> Vec<String> {
        let all: Vec<String> = messages
            .iter()
            .filter(|message| message.role == role)
            .map(text_of)
            .filter(|text| !text.trim().is_empty())
            .collect();
        all[all.len().saturating_sub(keep)..]
            .iter()
            .map(|text| cut(text, most))
            .collect()
    };
    json!({
        "replies": of(Role::Assistant, last, REPLY),
        "prompts": of(Role::User, 3, PROMPT),
        "messages": messages.len(),
    })
}

/// [`told`], for the live session called `name`.
pub(crate) fn told_by(profile_id: &str, name: &str, last: usize) -> Result<Value, String> {
    let path = transcript_of(profile_id, name)
        .map_err(|err| err.message)?
        .ok_or_else(|| format!("no session called {name} is running or has run here"))?;
    if !path.is_file() {
        return Ok(
            json!({ "replies": [], "prompts": [], "messages": 0, "note": "it has written no transcript yet" }),
        );
    }
    Ok(told(
        &crate::adopted_history::from_transcript(&path, 0.0),
        last,
    ))
}

#[cfg(test)]
#[path = "session_told_tests.rs"]
mod tests;
