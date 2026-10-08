//! What the person last said to a session, read from the session's own log.
//!
//! An orchestrator's word approves nothing: only the person's does. That word
//! used to count only when typed in devpit's composer, so from the Claude app
//! (Remote Control) a draft the person asked for stayed where it was. The log
//! says who spoke, so devpit reads it there itself rather than taking the
//! agent's account of it. Measured on Claude Code 2.1.294, a `user` entry is:
//!
//! - devpit's composer: no `origin`, `promptSource: "sdk"`;
//! - Remote Control: `origin.kind: "human"`;
//! - another session: `origin.kind: "peer"` and `isMeta`;
//! - an idle notice or a task's: `isMeta` or another `origin.kind`;
//! - a compaction's summary: `isCompactSummary`;
//! - a tool's result: a `tool_result` block, the turn going on.
//!
//! Only the first two are the person, and devpit's own notice at the top of a
//! composer turn — `[devpit, not the person] …` — is cut off before reading.

use serde_json::Value;

/// The line devpit puts above a turn the person started, saying what changed.
const DEVPIT_NOTICE: &str = "[devpit, not the person]";

/// Where the person said it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum From {
    Composer,
    RemoteControl,
}

impl From {
    pub fn label(self) -> &'static str {
        match self {
            From::Composer => "the chat",
            From::RemoteControl => "Remote Control",
        }
    }
}

/// One thing the person said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    /// The entry's own id: one message authorises once.
    pub id: String,
    pub text: String,
    pub from: From,
}

/// The person's last words in a log's text, past whatever the turn did since.
pub fn last_said(log: &str) -> Option<Said> {
    log.lines().rev().find_map(|line| {
        let entry = serde_json::from_str::<Value>(line).ok()?;
        (entry["type"] == "user").then_some(())?;
        person_said(&entry)
    })
}

/// What one `user` entry says the person said, if it is the person's.
fn person_said(entry: &Value) -> Option<Said> {
    if entry["isMeta"] == true || entry["isCompactSummary"] == true {
        return None;
    }
    let from = match entry["origin"]["kind"].as_str() {
        Some("human") => From::RemoteControl,
        None if entry["promptSource"] == "sdk" => From::Composer,
        _ => return None,
    };
    let content = &entry["message"]["content"];
    let text = match content {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => {
            if blocks.iter().any(|block| block["type"] == "tool_result") {
                return None;
            }
            blocks
                .iter()
                .filter(|block| block["type"] == "text")
                .filter_map(|block| block["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => return None,
    };
    let text = without_notice(&text).trim().to_owned();
    Some(Said {
        id: entry["uuid"].as_str()?.to_owned(),
        text,
        from,
    })
}

/// `text` without devpit's notice above it.
fn without_notice(text: &str) -> &str {
    if !text.trim_start().starts_with(DEVPIT_NOTICE) {
        return text;
    }
    text.trim_start()
        .split_once("\n\n")
        .map(|(_, rest)| rest)
        .unwrap_or("")
}

/// Words that, said alone, send the one draft waiting.
const SEND: [&str; 11] = [
    "send",
    "send it",
    "go ahead",
    "envie",
    "envia",
    "manda",
    "mande",
    "pode",
    "pode enviar",
    "pode mandar",
    "confirma",
];

/// Words that turn a request around: "não manda ainda" is not a yes.
const NOT: [&str; 9] = [
    "não", "nao", "not", "don't", "dont", "never", "nunca", "espera", "wait",
];

/// Verbs that, beside a session's name, ask for something to reach it.
const RELAY: [&str; 18] = [
    "send", "tell", "run", "envie", "envia", "manda", "mande", "pede", "peça", "peca", "avisa",
    "avise", "diga", "diz", "ativa", "ative", "roda", "rode",
];

/// Whether what the person said lets the draft for `session` go, with
/// `waiting` drafts in all: a bare "send" when it is the only one, or the
/// session named beside a verb that asks for something to reach it.
pub fn authorises(said: &str, session: &str, waiting: usize) -> bool {
    if bare(said) {
        return waiting == 1;
    }
    let said = said.trim().to_lowercase();
    let words: Vec<&str> = said
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '\'')
        .filter(|word| !word.is_empty())
        .collect();
    names(&words, session)
        && words.iter().any(|word| RELAY.contains(word))
        && !words.iter().any(|word| NOT.contains(word))
}

/// Whether `said` is a send word alone, which names no session: it lets one
/// draft go, whichever it is, and so only once.
pub fn bare(said: &str) -> bool {
    let said = said.trim().to_lowercase();
    SEND.contains(&said.trim_end_matches(['.', '!', ' ']))
}

/// Whether `words` name the session: whole, or by a part of its name long
/// enough to mean it (`noiseless` for `noiseless-mvp`).
fn names(words: &[&str], session: &str) -> bool {
    let session = session.to_lowercase();
    words.iter().any(|word| {
        *word == session
            || session
                .split('-')
                .filter(|part| part.chars().count() >= 4)
                .any(|part| part == *word)
    })
}

#[cfg(test)]
#[path = "person_tests.rs"]
mod tests;
