//! A hook's payload, cut down to what devpit reads from it.
//!
//! Claude Code sends everything it has: a `PostToolUse` after a Read carries
//! the whole file, after a Bash the whole output. devpit reads the tool's
//! name and input, never its response — except an `Agent` call's, which says
//! which subagent it started. What is not read is not carried, and no string
//! is longer than devpit would ever show.

use serde_json::Value;

/// A payload bigger than this is cut down before it is read.
pub const SLIM_ABOVE: usize = 64 * 1024;
/// The longest any string in a cut payload may be, in characters.
const LONGEST: usize = 16 * 1024;
/// The most items any array in a cut payload keeps.
const MOST_ITEMS: usize = 256;
/// How deep a cut payload goes; anything deeper is dropped.
const DEEPEST: usize = 32;

/// The tools whose response devpit reads: an `Agent` call answers with the
/// subagent it started.
const RESPONSE_READ: [&str; 2] = ["Agent", "Task"];

/// `body`, without the response devpit does not read and with every string
/// and array under a ceiling. `None` when it is not JSON.
pub fn slimmed(body: &str) -> Option<String> {
    let mut value: Value = serde_json::from_str(body).ok()?;
    if let Some(object) = value.as_object_mut() {
        let reads_response = object
            .get("tool_name")
            .and_then(Value::as_str)
            .is_some_and(|tool| RESPONSE_READ.contains(&tool));
        if !reads_response {
            object.remove("tool_response");
        }
    }
    cut(&mut value, 0);
    serde_json::to_string(&value).ok()
}

fn cut(value: &mut Value, depth: usize) {
    match value {
        Value::String(text) => {
            if let Some((at, _)) = text.char_indices().nth(LONGEST) {
                text.truncate(at);
                text.push('…');
            }
        }
        Value::Array(items) => {
            items.truncate(MOST_ITEMS);
            for item in items {
                deeper(item, depth);
            }
        }
        Value::Object(fields) => {
            for (_, field) in fields.iter_mut() {
                deeper(field, depth);
            }
        }
        _ => {}
    }
}

fn deeper(value: &mut Value, depth: usize) {
    if depth + 1 >= DEEPEST {
        *value = Value::Null;
    } else {
        cut(value, depth + 1);
    }
}

#[cfg(test)]
#[path = "slim_tests.rs"]
mod tests;
