//! Claude Code's stream-json, line by line.
//!
//! Apart from `driver.rs` because that file is the contract every CLI meets,
//! and this is one vendor's grammar — the part with cases in it, and the part
//! that changes when the vendor does. Shapes were read off a recorded turn of
//! 2.1.270 (`tests/fixtures/`), not off documentation.

use devpit_rpc::{CallState, Part, SessionInit};

use crate::driver::Read;

fn content_of(value: &serde_json::Value) -> &[serde_json::Value] {
    value
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(|content| content.as_array())
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// The `Agent` call a subagent's line belongs to. Top-level on the line, not
/// inside the message — measured against Claude Code 2.1.270.
fn parent_of(value: &serde_json::Value) -> Option<String> {
    value
        .get("parent_tool_use_id")
        .and_then(|parent| parent.as_str())
        .map(str::to_owned)
}

fn strings_at(value: &serde_json::Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|found| found.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|one| one.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn optional_at(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|found| found.as_str())
        .map(str::to_owned)
}

/// The `system` lines worth keeping: the session's self-description, and the
/// lifecycle of work the CLI runs in the background. Hook chatter, token
/// estimates and rate limits stay out of the conversation.
pub(crate) fn system(value: &serde_json::Value) -> Read {
    let task = |status: String| {
        Read::Parts(vec![Part::Task {
            task_id: string_at(value, "task_id"),
            call_id: optional_at(value, "tool_use_id"),
            task_kind: optional_at(value, "task_type"),
            description: optional_at(value, "description"),
            status,
            summary: optional_at(value, "summary"),
        }])
    };
    match value.get("subtype").and_then(|kind| kind.as_str()) {
        Some("init") => Read::Init(SessionInit {
            model: optional_at(value, "model"),
            slash_commands: strings_at(value, "slash_commands"),
            terminal_slash_commands: strings_at(value, "terminal_slash_commands"),
            skills: strings_at(value, "skills"),
            agents: strings_at(value, "agents"),
        }),
        // A slash command's own answer (`/compact`, `/clear`), which the CLI
        // prints as a system line rather than as the agent speaking.
        Some("local_command") => Read::Parts(vec![Part::Command {
            content: string_at(value, "content"),
        }]),
        Some("task_started") => task("started".to_owned()),
        Some("task_progress") => task("running".to_owned()),
        Some("task_updated") => match value
            .get("patch")
            .and_then(|patch| patch.get("status"))
            .and_then(|status| status.as_str())
        {
            Some(status) => task(status.to_owned()),
            None => Read::Nothing,
        },
        Some("task_notification") => {
            task(optional_at(value, "status").unwrap_or_else(|| "completed".to_owned()))
        }
        _ => Read::Nothing,
    }
}

pub(crate) fn assistant_parts(value: &serde_json::Value) -> Vec<Part> {
    let parent = parent_of(value);
    content_of(value)
        .iter()
        .filter_map(
            |block| match block.get("type").and_then(|kind| kind.as_str()) {
                Some("text") => block
                    .get("text")
                    .and_then(|text| text.as_str())
                    .map(|text| Part::Text {
                        text: text.to_owned(),
                        parent: parent.clone(),
                    }),
                Some("thinking") => {
                    block
                        .get("thinking")
                        .and_then(|text| text.as_str())
                        .map(|text| Part::Thinking {
                            text: text.to_owned(),
                            parent: parent.clone(),
                        })
                }
                Some("tool_use") => Some(Part::ToolCall {
                    id: string_at(block, "id"),
                    name: string_at(block, "name"),
                    input: block
                        .get("input")
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                    state: CallState::Running,
                    parent: parent.clone(),
                }),
                _ => None,
            },
        )
        .collect()
}

pub(crate) fn tool_results(value: &serde_json::Value) -> Vec<Part> {
    let parent = parent_of(value);
    content_of(value)
        .iter()
        .filter(|block| block.get("type").and_then(|kind| kind.as_str()) == Some("tool_result"))
        .map(|block| Part::ToolResult {
            call_id: string_at(block, "tool_use_id"),
            output: block
                .get("content")
                .map(|content| match content.as_str() {
                    Some(text) => text.to_owned(),
                    None => content.to_string(),
                })
                .unwrap_or_default(),
            is_error: block
                .get("is_error")
                .and_then(|flag| flag.as_bool())
                .unwrap_or(false),
            parent: parent.clone(),
        })
        .collect()
}

pub(crate) fn string_at(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|found| found.as_str())
        .unwrap_or_default()
        .to_owned()
}
