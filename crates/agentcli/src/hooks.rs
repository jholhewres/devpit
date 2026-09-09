//! Hearing what an agent is doing, from the agent itself.
//!
//! `claude agents --json` answers "is it working" once per poll. A hook answers
//! "it is about to run a command called X" at the moment it happens, and it is
//! the agent telling us rather than us inferring it from a terminal title or a
//! regex over output — which is guesswork that breaks the first time the output
//! changes.
//!
//! The shape is taken from what the CLI actually sends, recorded rather than
//! read from documentation:
//!
//! ```text
//! PreToolUse -> cwd, hook_event_name, permission_mode, prompt_id, session_id,
//!               tool_input, tool_name, tool_use_id, transcript_path
//! Stop       -> ..., last_assistant_message, stop_hook_active
//! ```
//!
//! Two rules the receiver keeps, both learned from how these fail elsewhere:
//!
//! 1. **The hook runs on the agent's critical path.** It gets a short timeout
//!    and gives up rather than holding the agent while a dead app is waited on.
//! 2. **The endpoint lives on disk and is read on every invocation.** A pty
//!    that outlived a restart would otherwise keep posting to a port nothing is
//!    listening on.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// What a hook told us.
#[derive(Debug, Clone, PartialEq)]
pub struct Happening {
    pub session_id: String,
    pub event: Event,
    pub cwd: String,
}

/// The events worth reacting to.
///
/// Deliberately fewer than the CLI sends: an event the board cannot draw is an
/// event nobody asked for, and every one of these changes what a card shows.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// About to run a tool, named.
    Using { tool: String },
    /// Finished a tool.
    Used { tool: String },
    /// The turn ended, with the last thing it said.
    Stopped { said: Option<String> },
    /// Waiting on a person — the state that matters most, because nothing
    /// moves until someone comes back.
    Waiting,
}

#[derive(Deserialize)]
struct Raw {
    #[serde(default)]
    hook_event_name: String,
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    cwd: String,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    last_assistant_message: Option<String>,
}

/// Reads one hook payload.
///
/// An event this build has no use for is `None`, not an error: the CLI is free
/// to send more of them, and a receiver that fails on an unknown name is a
/// receiver that breaks on an upgrade.
pub fn read(payload: &str) -> Option<Happening> {
    let raw: Raw = serde_json::from_str(payload).ok()?;
    if raw.session_id.is_empty() {
        return None;
    }

    let event = match raw.hook_event_name.as_str() {
        "PreToolUse" => Event::Using {
            tool: raw.tool_name?,
        },
        "PostToolUse" => Event::Used {
            tool: raw.tool_name?,
        },
        "Stop" | "SubagentStop" => Event::Stopped {
            said: raw.last_assistant_message,
        },
        "Notification" | "PermissionRequest" => Event::Waiting,
        _ => return None,
    };

    Some(Happening {
        session_id: raw.session_id,
        event,
        cwd: raw.cwd,
    })
}

/// Where the endpoint is written, and read back from.
pub fn endpoint_file(root: &Path) -> PathBuf {
    root.join("hook-endpoint")
}

#[cfg(test)]
#[path = "hooks_tests.rs"]
mod tests;
