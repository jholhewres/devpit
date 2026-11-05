//! A whole conversation read for what it says: where a word is, and what the
//! person asked. The window holds a page of it; these answer for all of it.

use std::borrow::Cow;

use devpit_rpc::{ConversationPrompts, FoundIn, Message, Part, Prompt, Role};

/// The most of a prompt the rail shows.
const OPENING: usize = 140;

/// The messages whose prose holds `query`, ignoring case, and how many times
/// each does: the prose the window draws, not tool output or thinking.
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
                .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|text| !text.is_empty())
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

/// The prose a message draws: its own text, not a subagent's (drawn in the
/// folded work), and a person's words without the attachment line drawn as chips.
fn prose(message: &Message) -> impl Iterator<Item = Cow<'_, str>> {
    let person = message.role == Role::User;
    message.parts.iter().filter_map(move |part| match part {
        Part::Text { text, parent: None } if person => Some(typed(text)),
        Part::Text { text, parent: None } => Some(Cow::Borrowed(text.as_str())),
        _ => None,
    })
}

/// A sent prompt without its `@path` line, as the window's `sentWith` splits
/// it: the first line, or the second under a picked skill's.
fn typed(text: &str) -> Cow<'_, str> {
    let lines = text.split('\n').collect::<Vec<_>>();
    if attached(lines[0]) {
        return Cow::Owned(lines[1..].join("\n"));
    }
    let skilled = lines[0].starts_with('/') || lines[0].starts_with("Use the ");
    if skilled && lines.get(1).is_some_and(|line| attached(line)) {
        return Cow::Owned([&lines[..1], &lines[2..]].concat().join("\n"));
    }
    Cow::Borrowed(text)
}

fn attached(line: &str) -> bool {
    line.starts_with('@')
        && line
            .split(' ')
            .all(|token| token.len() > 1 && token.starts_with('@'))
}

#[cfg(test)]
#[path = "store_find_tests.rs"]
mod tests;
