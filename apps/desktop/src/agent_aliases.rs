//! One name for each thing an agent points at, whatever a tool calls it.
//!
//! A card is `cardId` on the board's tools, `card` on Devflow's and `epic` on
//! a plan's; the session a tool acts on is `name`, where resume calls it
//! `session`. An agent sending the other spelling was told "no card ``" and
//! guessed again, so every spelling is moved to the one the method reads
//! before anything reads it.

use serde_json::{Map, Value};

/// The spellings of a card, in the order they are taken.
const CARD: [&str; 3] = ["cardId", "card", "epic"];

/// The spellings of the session a tool acts on.
const SESSION: [&str; 2] = ["name", "session"];

/// The key `method` reads its card from, when it takes one.
pub(crate) fn card_key(method: &str) -> Option<&'static str> {
    match method {
        "devflow_brief" | "devflow_evidence" | "devflow_review" | "devflow_gate"
        | "devflow_stage" => Some("card"),
        "devflow_plan" | "devflow_autopilot" => Some("epic"),
        "card" | "comment" | "update" | "move" | "start_card" | "finish_card" | "handoff"
        | "repo" | "remind" | "resolve_reminder" | "decide" | "rubric_run" | "start" => {
            Some("cardId")
        }
        _ => None,
    }
}

/// The key `method` reads the session it acts on from. Not resume's or
/// start's `name`: there it is the new name, and taking it as the target
/// would rename instead of choose.
fn session_key(method: &str) -> Option<&'static str> {
    match method {
        "screen" | "stop" | "transcript" | "draft" | "send_draft" => Some("name"),
        _ => None,
    }
}

/// `params` with the card and the session under the keys `method` reads.
/// The other spelling is dropped once moved: Devflow's tools refuse a field
/// they do not take.
pub(crate) fn aliased(method: &str, mut params: Value) -> Value {
    let Some(fields) = params.as_object_mut() else {
        return params;
    };
    if let Some(key) = card_key(method) {
        moved(fields, key, &CARD);
    }
    if let Some(key) = session_key(method) {
        moved(fields, key, &SESSION);
    }
    params
}

/// Puts the first spelling given under `key`, unless `key` is given itself:
/// then both stay, and a tool that refuses the other says so.
fn moved(fields: &mut Map<String, Value>, key: &str, spellings: &[&str]) {
    if fields.get(key).is_some_and(given) {
        return;
    }
    let Some(other) = spellings
        .iter()
        .find(|one| **one != key && fields.get(**one).is_some_and(given))
    else {
        return;
    };
    if let Some(value) = fields.remove(*other) {
        fields.insert(key.to_owned(), value);
    }
}

fn given(value: &Value) -> bool {
    value.as_str().is_some_and(|said| !said.trim().is_empty())
}

#[cfg(test)]
#[path = "agent_aliases_tests.rs"]
mod tests;
