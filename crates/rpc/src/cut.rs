//! What of a tool's output the window is sent: a long one cut to its head and
//! tail, an image named rather than carried.
//!
//! The transcript keeps every byte; this is only what crosses to the screen.
//! A screenshot is a megabyte of base64 the window parses and never shows as
//! text, and a conversation of them froze it for seconds on every open.

use serde_json::Value;

use crate::chat::{Message, Part};

/// A text this long or shorter goes whole.
pub const WHOLE: usize = 4 * 1024;
/// What a longer one keeps of its start and of its end.
pub const HEAD: usize = 2 * 1024;
pub const TAIL: usize = 1024;

/// Marks a cut text: it starts the line said in its place.
pub const CUT_MARK: &str = "\u{2026} [cut: ";

/// `text`, or its head and tail with how much was left out between them.
/// Cut on character boundaries: a byte count that lands inside one moves back.
pub fn cut_text(text: &str) -> Option<String> {
    if text.len() <= WHOLE {
        return None;
    }
    let mut head = HEAD;
    while !text.is_char_boundary(head) {
        head -= 1;
    }
    let mut tail = text.len() - TAIL;
    while !text.is_char_boundary(tail) {
        tail += 1;
    }
    Some(format!(
        "{}\n{CUT_MARK}{} bytes in all]\n{}",
        &text[..head],
        text.len(),
        &text[tail..],
    ))
}

/// A tool's output as the window gets it. Claude's content blocks stay
/// blocks: an image keeps its kind and size and loses its data, and each text
/// is cut on its own. Anything else is text.
pub fn cut_output(output: &str) -> Option<String> {
    // Blocks are looked into at any length: an image is never carried, however small.
    let Ok(Value::Array(mut blocks)) = serde_json::from_str::<Value>(output) else {
        return cut_text(output);
    };
    let mut changed = false;
    for block in &mut blocks {
        let Some(kind) = block.get("type").and_then(Value::as_str) else {
            continue;
        };
        match kind {
            "image" => {
                let source = block.get("source");
                let data = source
                    .and_then(|source| source.get("data"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let mime = source
                    .and_then(|source| source.get("media_type"))
                    .cloned()
                    .unwrap_or(Value::Null);
                *block = serde_json::json!({
                    "type": "image",
                    "mime": mime,
                    // What the image weighs, not the base64 that carried it.
                    "bytes": data.len() / 4 * 3,
                    "cut": true,
                });
                changed = true;
            }
            "text" => {
                if let Some(cut) = block.get("text").and_then(Value::as_str).and_then(cut_text) {
                    block["text"] = Value::String(cut);
                    changed = true;
                }
            }
            _ => {}
        }
    }
    if !changed {
        return None;
    }
    serde_json::to_string(&blocks).ok()
}

/// The part as the window gets it; `None` when it goes whole. A tool call's
/// input is never cut: the edits the screen shows as a diff are in it.
pub fn cut_part(part: &Part) -> Option<Part> {
    match part {
        Part::ToolResult {
            call_id,
            output,
            is_error,
            parent,
        } => cut_output(output).map(|output| Part::ToolResult {
            call_id: call_id.clone(),
            output,
            is_error: *is_error,
            parent: parent.clone(),
        }),
        _ => None,
    }
}

/// The frame as the window gets it: its part or message cut.
pub fn cut_frame(frame: crate::Frame) -> crate::Frame {
    match frame {
        crate::Frame::Part { message_id, part } => crate::Frame::Part {
            message_id,
            part: cut_part(&part).unwrap_or(part),
        },
        crate::Frame::Opened { mut message } => {
            cut_message(&mut message);
            crate::Frame::Opened { message }
        }
        other => other,
    }
}

/// Cuts every part of a message in place.
pub fn cut_message(message: &mut Message) {
    for part in &mut message.parts {
        if let Some(cut) = cut_part(part) {
            *part = cut;
        }
    }
}

#[cfg(test)]
#[path = "cut_tests.rs"]
mod tests;
