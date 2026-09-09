//! One driver per CLI, behind one trait.
//!
//! Each vendor prints its own shape and changes it without asking. A driver
//! turns those lines into the typed parts the contract carries, so a new CLI
//! is a new file here and nothing else.

use devpit_rpc::{CallState, Part};

/// What one line of a CLI's output turned into.
#[derive(Debug, Clone, PartialEq)]
pub enum Read {
    /// Parts to append to the message being built.
    Parts(Vec<Part>),
    /// The turn ended, with the CLI's own reason.
    Ended {
        stop_reason: Option<String>,
        cost_usd: Option<f64>,
        is_error: bool,
    },
    /// Nothing to show — a keepalive, a blank line.
    Nothing,
}

pub trait Driver: Send + Sync {
    /// The name this driver answers to, and what a conversation records.
    fn name(&self) -> &'static str;

    /// The models it offers. The composer picks from these; it never picks a
    /// driver, because that is fixed for the conversation's life.
    fn models(&self) -> &'static [&'static str];

    /// Reads one line of output.
    fn read(&self, line: &str) -> Read;
}

/// The Claude Code CLI, which prints one JSON object per line.
pub struct Claude;

impl Driver for Claude {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn models(&self) -> &'static [&'static str] {
        &["default", "opus", "sonnet", "haiku"]
    }

    fn read(&self, line: &str) -> Read {
        let line = line.trim();
        if line.is_empty() {
            return Read::Nothing;
        }

        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            // A line we cannot parse is still output. Dropping it would hide
            // exactly the errors a CLI prints plainly.
            return Read::Parts(vec![Part::Unknown {
                text: line.to_owned(),
            }]);
        };

        match value.get("type").and_then(|kind| kind.as_str()) {
            Some("result") => Read::Ended {
                stop_reason: value
                    .get("subtype")
                    .and_then(|reason| reason.as_str())
                    .map(str::to_owned),
                cost_usd: value.get("total_cost_usd").and_then(|cost| cost.as_f64()),
                is_error: value
                    .get("is_error")
                    .and_then(|flag| flag.as_bool())
                    .unwrap_or(false),
            },
            Some("assistant") => Read::Parts(assistant_parts(&value)),
            Some("user") => Read::Parts(tool_results(&value)),
            _ => Read::Nothing,
        }
    }
}

fn content_of(value: &serde_json::Value) -> &[serde_json::Value] {
    value
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(|content| content.as_array())
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn assistant_parts(value: &serde_json::Value) -> Vec<Part> {
    content_of(value)
        .iter()
        .filter_map(
            |block| match block.get("type").and_then(|kind| kind.as_str()) {
                Some("text") => block
                    .get("text")
                    .and_then(|text| text.as_str())
                    .map(|text| Part::Text {
                        text: text.to_owned(),
                    }),
                Some("thinking") => {
                    block
                        .get("thinking")
                        .and_then(|text| text.as_str())
                        .map(|text| Part::Thinking {
                            text: text.to_owned(),
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
                }),
                _ => None,
            },
        )
        .collect()
}

fn tool_results(value: &serde_json::Value) -> Vec<Part> {
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
        })
        .collect()
}

fn string_at(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|found| found.as_str())
        .unwrap_or_default()
        .to_owned()
}

/// The driver a conversation named, or nothing when it is not installed.
pub fn driver(name: &str) -> Option<Box<dyn Driver>> {
    match name {
        "claude" => Some(Box::new(Claude)),
        _ => None,
    }
}
