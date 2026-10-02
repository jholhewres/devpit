//! `devpit agent hook`: events from any agent, not only Claude Code.
//!
//! A small, fixed protocol so aider, opencode, a CI job or a script of one's
//! own shows up on the island and the card with no code in devpit for it:
//!
//! ```text
//! devpit-agent hook --agent aider SessionStart
//! devpit-agent hook --agent aider PreToolUse --tool Edit --target src/app.rs
//! echo '{"tool_name":"Bash","tool_input":{"command":"npm test"}}' | devpit-agent hook --agent aider PreToolUse
//! devpit-agent hook --agent aider Stop --said "Tests pass"
//! ```
//!
//! What it sends is a Claude Code hook payload — the shape devpit already
//! reads — so nothing on the other side knows a second shape. It never sends
//! a permission question: an approval asked this way could pass for Claude
//! Code's, and approving is the person's alone.

use serde_json::{json, Map, Value};

/// The most JSON an agent may pipe in.
pub(crate) const MOST_BYTES: u64 = 64 * 1024;

/// The events this protocol takes, as Claude Code names them.
pub const EVENTS: [&str; 9] = [
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PostToolUseFailure",
    "Notification",
    "Stop",
    "StopFailure",
    "SessionEnd",
];

/// One event, as the command line gave it.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub agent: String,
    pub event: String,
    pub session: Option<String>,
    pub tool: Option<String>,
    pub target: Option<String>,
    pub said: Option<String>,
}

/// Whether `name` may name an agent: short, plain, and not Claude Code's,
/// which speaks through its own hooks and must not be spoken for.
pub fn agent_name(name: &str) -> bool {
    (1..=24).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && name != "claude"
}

/// The call, from `--agent NAME EVENT` and its options.
pub fn parsed(options: &[(&str, &str)], plain: &[&str]) -> Result<Call, String> {
    let option = |name: &str| {
        options
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| (*value).to_owned())
            .filter(|value| !value.is_empty())
    };
    let agent = option("agent").ok_or("hook needs --agent NAME")?;
    if !agent_name(&agent) {
        return Err(format!(
            "`{agent}` is not an agent name: up to 24 of a-z, 0-9 and -, and not claude"
        ));
    }
    let event = plain
        .first()
        .copied()
        .ok_or("hook needs an event, such as SessionStart")?;
    if event == "PermissionRequest" {
        return Err(
            "an approval is never asked this way: the person answers in the agent itself"
                .to_owned(),
        );
    }
    if !EVENTS.contains(&event) {
        return Err(format!("`{event}` is not an event: {}", EVENTS.join(", ")));
    }
    Ok(Call {
        agent,
        event: event.to_owned(),
        session: option("session"),
        tool: option("tool"),
        target: option("target"),
        said: option("said"),
    })
}

/// The Claude Code payload for `call`, over whatever JSON was piped in, in
/// the folder `cwd`. The session is the one named, or the agent's process.
pub fn payload(call: &Call, piped: &Value, cwd: &str) -> String {
    let mut body: Map<String, Value> = piped.as_object().cloned().unwrap_or_default();
    body.insert("hook_event_name".into(), json!(call.event));
    body.insert("devpit_agent".into(), json!(call.agent));
    let session = call
        .session
        .clone()
        .or_else(|| {
            body.get("session_id")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| default_session(&call.agent));
    body.insert("session_id".into(), json!(session));
    if !body.contains_key("cwd") {
        body.insert("cwd".into(), json!(cwd));
    }
    if let Some(tool) = &call.tool {
        body.insert("tool_name".into(), json!(tool));
    }
    if let Some(target) = &call.target {
        // Read as a path when it looks like one, as a description otherwise.
        let field = if target.contains(['/', '\\', '.']) {
            "file_path"
        } else {
            "description"
        };
        let input = body.entry("tool_input").or_insert_with(|| json!({}));
        if let Some(input) = input.as_object_mut() {
            input.insert(field.into(), json!(target));
        }
    }
    if let Some(said) = &call.said {
        let field = if call.event == "StopFailure" {
            "error"
        } else {
            "last_assistant_message"
        };
        body.insert(field.into(), json!(said));
    }
    // An agent with no tool names one, so a step reads as something.
    if matches!(
        call.event.as_str(),
        "PreToolUse" | "PostToolUse" | "PostToolUseFailure"
    ) && !body.contains_key("tool_name")
    {
        body.insert("tool_name".into(), json!("Work"));
    }
    Value::Object(body).to_string()
}

/// One session per agent process: the agent that runs this command is its
/// parent, and every event it sends is the same session.
fn default_session(agent: &str) -> String {
    #[cfg(unix)]
    let parent = std::os::unix::process::parent_id();
    #[cfg(not(unix))]
    let parent = 0;
    format!("{agent}-{parent}")
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
