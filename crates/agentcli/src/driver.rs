//! One driver per CLI, behind one trait.
//!
//! Each vendor prints its own shape and changes it without asking. A driver
//! turns those lines into the typed parts the contract carries, so a new CLI
//! is a new file here and nothing else.

use devpit_rpc::{Context, Part, SessionInit};

use crate::claude_lines::{assistant_parts, system, tool_results};

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
        context: Option<Context>,
    },
    /// The CLI described itself: its commands, skills and model.
    Init(SessionInit),
    /// Nothing to show — a keepalive, a blank line.
    Nothing,
}

pub trait Driver: Send + Sync {
    /// The name this driver answers to, and what a conversation records.
    fn name(&self) -> &'static str;

    /// The models it offers. The composer picks from these; it never picks a
    /// driver, because that is fixed for the conversation's life.
    fn models(&self) -> &'static [&'static str];

    /// How hard it may be asked to think, and what it does by default.
    ///
    /// Empty for a CLI with no such control — the composer draws no chip
    /// rather than a chip with one option in it.
    fn efforts(&self) -> &'static [&'static str] {
        &[]
    }

    fn effort_default(&self) -> Option<&'static str> {
        None
    }

    /// Reads one line of output.
    fn read(&self, line: &str) -> Read;

    /// The CLI's own id for this conversation, if the line carries it.
    ///
    /// Kept so the next turn resumes the same thread: without it the agent
    /// starts over and the transcript on screen is the only memory left.
    fn session(&self, line: &str) -> Option<String>;

    /// The id the CLI's transcript gives this line, when it is the agent's own
    /// message — what a rewind names to fork at. None for a CLI with no such id.
    fn anchor(&self, _line: &str) -> Option<String> {
        None
    }
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

    /// `--effort`, as the CLI's own help lists it.
    fn efforts(&self) -> &'static [&'static str] {
        &["low", "medium", "high", "xhigh", "max"]
    }

    fn effort_default(&self) -> Option<&'static str> {
        Some("high")
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
                context: context_of(&value),
            },
            Some("assistant") => Read::Parts(assistant_parts(&value)),
            Some("user") => Read::Parts(tool_results(&value)),
            Some("system") => system(&value),
            _ => Read::Nothing,
        }
    }

    fn session(&self, line: &str) -> Option<String> {
        serde_json::from_str::<serde_json::Value>(line)
            .ok()?
            .get("session_id")?
            .as_str()
            .map(str::to_owned)
    }

    /// Measured: a stream-json assistant line's `uuid` is the transcript's.
    fn anchor(&self, line: &str) -> Option<String> {
        let value = serde_json::from_str::<serde_json::Value>(line).ok()?;
        // A subagent's messages live in its own transcript, not this one.
        let own = value
            .get("parent_tool_use_id")
            .is_none_or(|parent| parent.is_null());
        if value.get("type")?.as_str()? != "assistant" || !own {
            return None;
        }
        value.get("uuid")?.as_str().map(str::to_owned)
    }
}

/// Measured on 2.1.270: `usage.iterations` holds one entry per request the
/// turn made, and the last one is what the context held when it ended — the
/// top-level `usage` sums them, which overcounts any turn that used a tool.
/// The window is the main model's, from `modelUsage`.
fn context_of(result: &serde_json::Value) -> Option<Context> {
    let last = result.get("usage")?.get("iterations")?.as_array()?.last()?;
    let tokens = |key: &str| last.get(key).and_then(|n| n.as_u64()).unwrap_or(0);
    let used = tokens("input_tokens")
        + tokens("cache_read_input_tokens")
        + tokens("cache_creation_input_tokens")
        + tokens("output_tokens");
    let window = result
        .get("modelUsage")?
        .as_object()?
        .values()
        .filter_map(|model| model.get("contextWindow")?.as_u64())
        .max()?;
    Some(Context {
        used: u32::try_from(used).ok()?,
        window: u32::try_from(window).ok()?,
    })
}

/// The driver a conversation named, or nothing when it is not installed.
pub fn driver(name: &str) -> Option<Box<dyn Driver>> {
    match name {
        "claude" => Some(Box::new(Claude)),
        _ => None,
    }
}
