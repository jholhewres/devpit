//! "Always in this chat": what a person allowed once and would allow again
//! in the same conversation, so it is not asked a second time.
//!
//! Held in memory, by the CLI's session, and never written anywhere: a rule
//! the person gave one conversation is not a setting of theirs.
//!
//! Narrow on purpose. An edit allowed is every edit in the chat — the mode
//! that asks is mostly asking about those. A command allowed is that command,
//! exactly: `rm -rf build` allowed is not `rm -rf ~` allowed.

use std::collections::HashMap;
use std::sync::Mutex;

use devpit_rpc::RpcError;

/// What one "always" covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Rule {
    Edits,
    Command(String),
    Tool(String),
}

const EDITS: [&str; 4] = ["Edit", "MultiEdit", "Write", "NotebookEdit"];

/// The rule an answer to this tool, with this input, would keep.
pub(crate) fn rule_for(tool: &str, input: &str) -> Rule {
    if EDITS.contains(&tool) {
        return Rule::Edits;
    }
    if tool == "Bash" {
        let command = serde_json::from_str::<serde_json::Value>(input)
            .ok()
            .and_then(|input| input.get("command")?.as_str().map(str::to_owned))
            .unwrap_or_default();
        return Rule::Command(command.trim().to_owned());
    }
    Rule::Tool(tool.to_owned())
}

/// The rules kept, by session.
#[derive(Default)]
pub struct Kept(Mutex<HashMap<String, Vec<Rule>>>);

impl Kept {
    pub(crate) fn keep(&self, session_id: &str, rule: Rule) {
        if let Ok(mut all) = self.0.lock() {
            let rules = all.entry(session_id.to_owned()).or_default();
            if !rules.contains(&rule) {
                rules.push(rule);
            }
        }
    }

    /// Whether a question is already answered by a rule its session keeps.
    pub(crate) fn covers(&self, session_id: &str, tool: &str, input: &str) -> bool {
        let wanted = rule_for(tool, input);
        if wanted == Rule::Command(String::new()) {
            return false;
        }
        self.0
            .lock()
            .ok()
            .and_then(|all| all.get(session_id).map(|rules| rules.contains(&wanted)))
            .unwrap_or(false)
    }
}

/// `permission.always` — allows this question, and the ones like it for the
/// rest of the conversation.
#[tauri::command]
#[specta::specta]
pub fn permission_always(
    asking: tauri::State<'_, crate::asking::Asking>,
    kept: tauri::State<'_, Kept>,
    id: String,
    session_id: String,
    tool: String,
    input: String,
) -> Result<(), RpcError> {
    kept.keep(&session_id, rule_for(&tool, &input));
    crate::asking::permission_answer(asking, id, crate::asking::Answer::Allow)
}

#[cfg(test)]
#[path = "asking_kept_tests.rs"]
mod tests;
