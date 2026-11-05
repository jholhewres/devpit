//! What an agent asks the person rather than asks to do: `AskUserQuestion`,
//! a plan to approve. A chat that draws the session answers it in any mode.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use devpit_rpc::{ErrorCode, RpcError};

use crate::asking::{decision, Answer, Asking};

/// The tools that put something to the person.
const TO_THE_PERSON: [&str; 2] = ["AskUserQuestion", "ExitPlanMode"];

pub fn to_the_person(tool: &str) -> bool {
    TO_THE_PERSON.contains(&tool)
}

/// The sessions chats draw, and the picks waiting for their hook's reply.
#[derive(Default)]
pub struct Picks {
    drawn: Mutex<HashSet<String>>,
    chosen: Mutex<HashMap<String, Vec<Vec<String>>>>,
}

impl Picks {
    /// A chat turn's session, once its stream names it.
    pub fn draw(&self, session_id: &str) {
        if let Ok(mut drawn) = self.drawn.lock() {
            drawn.insert(session_id.to_owned());
        }
    }

    /// Whether this tool waits for the person: a question, in a session a
    /// chat draws. A terminal's own is its terminal's to ask.
    pub fn holds(&self, session_id: &str, tool: &str) -> bool {
        to_the_person(tool)
            && self
                .drawn
                .lock()
                .is_ok_and(|drawn| drawn.contains(session_id))
    }

    pub(crate) fn picked(&self, id: &str, chosen: Vec<Vec<String>>) {
        if let Ok(mut all) = self.chosen.lock() {
            all.insert(id.to_owned(), chosen);
        }
    }

    pub(crate) fn taken(&self, id: &str) -> Option<Vec<Vec<String>>> {
        self.chosen.lock().ok()?.remove(id)
    }
}

/// Keeps the picks, then allows the question; nothing is kept for one that
/// no longer waits.
pub(crate) fn pick(asking: &Asking, picks: &Picks, id: &str, chosen: Vec<Vec<String>>) -> bool {
    picks.picked(id, chosen);
    let answered = asking.answer(id, Answer::Allow);
    if !answered {
        picks.taken(id);
    }
    answered
}

/// The hook's reply. Allowed with picks, they go in as the tool's `answers`,
/// keyed by each question as it was asked, several choices joined by ", ".
pub fn reply_for(picks: &Picks, id: &str, input: &str, answer: Answer) -> String {
    let chosen = picks.taken(id);
    let Some(chosen) = chosen.filter(|_| answer == Answer::Allow) else {
        return decision(answer);
    };
    let Ok(mut input) = serde_json::from_str::<serde_json::Value>(input) else {
        return decision(answer);
    };
    let asked = input
        .get("questions")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|one| one.get("question")?.as_str());
    let answers: serde_json::Map<String, serde_json::Value> = asked
        .zip(chosen)
        .map(|(question, labels)| (question.to_owned(), labels.join(", ").into()))
        .collect();
    input["answers"] = answers.into();
    serde_json::json!({ "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "allow",
        "permissionDecisionReason": "answered in devpit",
        "updatedInput": input,
    }})
    .to_string()
}

/// `permission.pick` — the person's choices to an `AskUserQuestion`: one
/// list per question, in the order asked.
#[tauri::command]
#[specta::specta]
pub async fn permission_pick(
    app: tauri::AppHandle,
    id: String,
    chosen: Vec<Vec<String>>,
) -> Result<(), RpcError> {
    crate::off_main::blocking(move || {
        use tauri::Manager;
        if pick(&app.state::<Asking>(), &app.state::<Picks>(), &id, chosen) {
            return Ok(());
        }
        Err(RpcError::new(
            ErrorCode::NotFound,
            "that question is no longer waiting — it timed out, or the turn ended",
        ))
    })
    .await
}

#[cfg(test)]
#[path = "asking_picks_tests.rs"]
mod tests;
