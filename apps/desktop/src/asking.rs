//! Permission questions, from the agent to the person and back.
//!
//! A `PreToolUse` hook can answer with a decision, which is the round trip
//! this needs: the agent asks, the hook holds, the window shows it, the person
//! answers. The hook that reports what a card is doing already runs on this
//! path — the same event, asked one more question.
//!
//! **The critical path stays fast by default.** Only a session that has said
//! it wants to be asked is held; every other one is answered at once with no
//! decision, which leaves the CLI's own permission mode in charge.

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

/// How long a question waits before it answers itself.
///
/// A hook that waits forever hangs the agent when the window is gone. Two
/// minutes is long enough for a person to look up and short enough that a
/// forgotten turn does not sit there for the afternoon.
const WAITS: Duration = Duration::from_secs(120);

/// What the agent wants to do, as the screen puts it.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    /// This question, so the answer can name it.
    pub id: String,
    /// The CLI's session, which is what ties it to a conversation.
    pub session_id: String,
    pub tool: String,
    /// The tool's input, verbatim. Parsing it here would be guessing at a
    /// shape the provider changes without asking.
    pub input: String,
    pub cwd: String,
}

/// What a person answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Answer {
    Allow,
    /// Refused. The refusal goes back to the agent as a tool result rather
    /// than killing the turn — an agent told "no" can try something else.
    Deny,
}

/// The questions in flight, and the sessions that want to be asked.
#[derive(Default)]
pub struct Asking {
    waiting: Arc<Mutex<HashMap<String, Sender<Answer>>>>,
    /// Sessions whose turns should stop and ask. Everything else is answered
    /// at once, so a board step never waits on a window nobody is watching.
    asked: Arc<Mutex<Vec<String>>>,
}

impl Asking {
    /// Whether this session wants its tools held for an answer.
    pub fn asks(&self, session_id: &str) -> bool {
        self.asked
            .lock()
            .map(|held| held.iter().any(|one| one == session_id))
            .unwrap_or(false)
    }

    pub fn ask_from_now(&self, session_id: &str, on: bool) {
        let Ok(mut held) = self.asked.lock() else {
            return;
        };
        held.retain(|one| one != session_id);
        if on {
            held.push(session_id.to_owned());
        }
    }

    /// Registers a question and hands back the end that waits on it.
    pub fn opened(&self, id: &str) -> Receiver<Answer> {
        let (tell, hear) = channel();
        if let Ok(mut held) = self.waiting.lock() {
            held.insert(id.to_owned(), tell);
        }
        hear
    }

    /// Waits for the person, or gives up.
    ///
    /// Giving up denies: a question nobody answered is not a question someone
    /// said yes to.
    pub fn wait(&self, id: &str, hear: Receiver<Answer>) -> Answer {
        let answer = hear.recv_timeout(WAITS).unwrap_or(Answer::Deny);
        if let Ok(mut held) = self.waiting.lock() {
            held.remove(id);
        }
        answer
    }

    /// Answers a question that is waiting. False when none was.
    pub fn answer(&self, id: &str, answer: Answer) -> bool {
        let Ok(held) = self.waiting.lock() else {
            return false;
        };
        held.get(id)
            .map(|tell| tell.send(answer).is_ok())
            .unwrap_or(false)
    }
}

/// The hook's reply, in the shape the CLI reads it.
///
/// `ask` is deliberately never sent: this *is* the asking, and handing the
/// question back would loop.
pub fn decision(answer: Answer) -> String {
    let (verdict, why) = match answer {
        Answer::Allow => ("allow", "allowed in devpit"),
        Answer::Deny => ("deny", "not allowed in devpit"),
    };
    format!(
        r#"{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","permissionDecision":"{verdict}","permissionDecisionReason":"{why}"}}}}"#
    )
}

/// `permission.answer` — what the person said.
#[tauri::command]
#[specta::specta]
pub fn permission_answer(
    state: tauri::State<'_, Asking>,
    id: String,
    answer: Answer,
) -> Result<(), RpcError> {
    if state.answer(&id, answer) {
        return Ok(());
    }
    Err(RpcError::new(
        ErrorCode::NotFound,
        "that question is no longer waiting — it timed out, or the turn ended",
    ))
}

/// `permission.ask_from_now` — whether this conversation stops to ask.
///
/// Per session rather than global: a board step running unattended must not
/// start waiting because a chat window asked to be consulted.
#[tauri::command]
#[specta::specta]
pub fn permission_ask_from_now(
    state: tauri::State<'_, Asking>,
    session_id: String,
    on: bool,
) -> Result<(), RpcError> {
    state.ask_from_now(&session_id, on);
    Ok(())
}

/// `permission.questions` — the shape the stream carries.
///
/// It exists so the generated contract carries `Question`: the questions
/// themselves arrive on an event, which specta does not describe.
#[tauri::command]
#[specta::specta]
pub fn permission_questions() -> Result<Vec<Question>, RpcError> {
    Ok(Vec::new())
}

#[cfg(test)]
#[path = "asking_tests.rs"]
mod tests;
