//! A whole conversation read for what it says: where a word is, and what the
//! person asked. The window holds a page of it; these answer for all of it.

use devpit_rpc::{ConversationPrompts, FoundIn, Message, Part, Prompt, Role};

/// The most of a prompt the rail shows.
const OPENING: usize = 140;

/// The messages whose prose holds `query`, ignoring case, and how many times
/// each does. Tool output and thinking are left out: they are drawn folded.
pub fn found(messages: &[Message], query: &str) -> Vec<FoundIn> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    messages
        .iter()
        .filter_map(|message| {
            let count = prose(message)
                .map(|text| text.to_lowercase().matches(needle.as_str()).count())
                .sum::<usize>();
            (count > 0).then(|| FoundIn {
                message_id: message.id.clone(),
                count: u32::try_from(count).unwrap_or(u32::MAX),
            })
        })
        .collect()
}

/// Each message the person sent, where it sits and how it opens.
pub fn prompts(messages: &[Message]) -> ConversationPrompts {
    let prompts = messages
        .iter()
        .enumerate()
        .filter(|(_, message)| message.role == Role::User)
        .filter_map(|(at, message)| {
            let words = prose(message)
                .flat_map(str::split_whitespace)
                .collect::<Vec<_>>()
                .join(" ");
            (!words.is_empty()).then(|| Prompt {
                message_id: message.id.clone(),
                at: u32::try_from(at).unwrap_or(u32::MAX),
                text: words.chars().take(OPENING).collect(),
            })
        })
        .collect();
    ConversationPrompts {
        prompts,
        total: u32::try_from(messages.len()).unwrap_or(u32::MAX),
    }
}

fn prose(message: &Message) -> impl Iterator<Item = &str> {
    message.parts.iter().filter_map(|part| match part {
        Part::Text { text, .. } => Some(text.as_str()),
        _ => None,
    })
}

#[cfg(test)]
#[path = "store_find_tests.rs"]
mod tests;
