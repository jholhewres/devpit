//! Reading a permission question out of a hook payload.
//!
//! Apart from the listener because it is about the CLI's shape, not about
//! sockets — and because a rule a test cannot call without opening a port is
//! a rule the test cannot guard.

use crate::asking::Question;

/// The question in a `PreToolUse` payload, or nothing when it is another
/// event.
///
/// Read from the raw payload rather than from `Happening`: that type carries
/// what the board draws, and a permission question needs the tool's input,
/// which the board has no use for.
pub fn question_in(body: &str) -> Option<Question> {
    let raw: serde_json::Value = serde_json::from_str(body).ok()?;
    if raw.get("hook_event_name")?.as_str()? != "PreToolUse" {
        return None;
    }
    Some(Question {
        id: raw
            .get("tool_use_id")
            .and_then(|found| found.as_str())
            .unwrap_or_default()
            .to_owned(),
        session_id: raw
            .get("session_id")
            .and_then(|found| found.as_str())
            .unwrap_or_default()
            .to_owned(),
        tool: raw
            .get("tool_name")
            .and_then(|found| found.as_str())
            .unwrap_or_default()
            .to_owned(),
        input: raw
            .get("tool_input")
            .map(ToString::to_string)
            .unwrap_or_default(),
        cwd: raw
            .get("cwd")
            .and_then(|found| found.as_str())
            .unwrap_or_default()
            .to_owned(),
    })
    .filter(|question| !question.id.is_empty())
}

/// Whether a tool changes something, which is what Supervised asks about:
/// "Ask before commands and file changes". A read, a search or devpit's own
/// board tools — which the chat's settings already allow — are left to the
/// CLI's permission mode, so the window never asks before a `Read`.
pub fn changes_something(tool: &str) -> bool {
    const CHANGING: &[&str] = &["Bash", "Edit", "Write", "MultiEdit", "NotebookEdit"];
    CHANGING.contains(&tool)
        || (tool.starts_with("mcp__") && !devpit_agentcli::ALLOWED_TOOLS.contains(&tool))
}

#[cfg(test)]
#[path = "question_tests.rs"]
mod tests;
