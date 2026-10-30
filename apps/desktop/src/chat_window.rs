//! What of a conversation the window reads: its history with long outputs
//! cut, and one of those outputs whole when a row is opened.

use devpit_agentcli::head::{head_path, read_head};
use devpit_agentcli::store::{conversation_path, read};
use devpit_rpc::{
    Conversation, ConversationAfter, ConversationHead, ErrorCode, Message, Part, RpcError,
};

/// `chat.history` — what was said in this conversation, in order: all of it,
/// or with `limit` the last that many before `before`, and whether older are left.
#[tauri::command]
#[specta::specta]
pub async fn chat_history(
    project_id: String,
    conversation_id: String,
    limit: Option<u32>,
    before: Option<String>,
) -> Result<Conversation, RpcError> {
    crate::off_main::blocking(move || {
        conversation(&conversation_id)?;
        let page = limit.map(|most| (most.max(1) as usize, before.as_deref()));
        // The window gets long outputs cut; the transcript keeps them whole,
        // and `chat_part_full` hands one back when it is opened.
        let mut read = chat_page_now(project_id, conversation_id, page)?;
        read.messages
            .iter_mut()
            .for_each(devpit_rpc::cut::cut_message);
        Ok(read)
    })
    .await
}

/// The conversation with all its messages, or with one page of them: up to
/// `most` before the one named `before`, read from the end of the file.
pub(crate) fn chat_page_now(
    project_id: String,
    conversation_id: String,
    page: Option<(usize, Option<&str>)>,
) -> Result<Conversation, RpcError> {
    let sessions = crate::projects::project_home(&project_id)?.sessions();
    let file = conversation_path(&sessions, &conversation_id);
    let (messages, older, skipped) = match page {
        None => {
            let (messages, skipped) = read(&file);
            (messages, false, skipped)
        }
        Some((most, before)) => {
            let page = devpit_agentcli::store_page::read_page(&file, before, most);
            (page.messages, page.older, page.skipped)
        }
    };
    if skipped > 0 {
        // Reported rather than hidden: a conversation missing a line should
        // say so, not quietly show less than happened.
        eprintln!("{conversation_id}: {skipped} unreadable line(s)");
    }
    let head = read_head(&head_path(&sessions, &conversation_id));
    // Only an empty transcript is backfilled, and a page before a message is never one.
    let messages = match page {
        Some((_, Some(_))) => messages,
        _ => crate::adopted_history::or_backfilled(messages, head.as_ref(), &project_id, &file),
    };
    let card = crate::card_chat::conversation_card(&crate::projects::store()?, &conversation_id)?;
    let told = crate::chat_window::conversation_head(head.as_ref());
    Ok(Conversation {
        id: conversation_id,
        project_id,
        card_id: card.id,
        card_title: card.title,
        card_on_board: card.on_board,
        // Empty until the first turn settles it: a conversation nobody has
        // spoken in belongs to no account yet.
        profile: head
            .as_ref()
            .map(|head| head.profile.clone())
            .unwrap_or_default(),
        model: head.as_ref().and_then(|head| head.model.clone()),
        permission: head.as_ref().and_then(|head| head.permission.clone()),
        effort: head.as_ref().and_then(|head| head.effort.clone()),
        cwd: head.as_ref().and_then(|head| head.cwd.clone()),
        session_id: told.session_id,
        cost_usd: told.cost_usd,
        context: told.context,
        rewindable: told.rewindable,
        created_at: head
            .map(|head| head.created_at)
            .or_else(|| messages.first().map(|first| first.created_at))
            .unwrap_or_else(crate::chat::now),
        messages,
        older,
    })
}

/// What a conversation's head says of it: its session, what it cost, how
/// full its context was, and the turns it can be rewound to.
pub(crate) fn conversation_head(
    head: Option<&devpit_agentcli::head::Head>,
) -> devpit_rpc::ConversationHead {
    devpit_rpc::ConversationHead {
        session_id: head.and_then(|head| head.session_id.clone()),
        cost_usd: head.map(|head| head.cost_usd).unwrap_or_default(),
        context: head.and_then(|head| head.context),
        rewindable: head
            .map(|head| {
                head.rewind
                    .anchors
                    .iter()
                    .map(|at| at.turn_id.clone())
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// `chat.head` — what a turn's end needs, without reading the conversation.
#[tauri::command]
#[specta::specta]
pub async fn chat_head(
    project_id: String,
    conversation_id: String,
) -> Result<ConversationHead, RpcError> {
    crate::off_main::blocking(move || {
        conversation(&conversation_id)?;
        let sessions = crate::projects::project_home(&project_id)?.sessions();
        let head = read_head(&head_path(&sessions, &conversation_id));
        Ok(conversation_head(head.as_ref()))
    })
    .await
}

/// `chat.after` — the messages saved after `after`, cut as the history is:
/// what a chat that rejoined a turn appends, instead of reading it all again.
/// Every message when `after` is not among them.
#[tauri::command]
#[specta::specta]
pub async fn chat_after(
    project_id: String,
    conversation_id: String,
    after: Option<String>,
) -> Result<ConversationAfter, RpcError> {
    crate::off_main::blocking(move || {
        conversation(&conversation_id)?;
        let sessions = crate::projects::project_home(&project_id)?.sessions();
        let read = read(&conversation_path(&sessions, &conversation_id)).0;
        Ok(ConversationAfter {
            messages: messages_after(read, after.as_deref()),
        })
    })
    .await
}

/// The messages after `after`, cut; all of them when it is not among them.
fn messages_after(mut messages: Vec<Message>, after: Option<&str>) -> Vec<Message> {
    if let Some(at) = after.and_then(|id| messages.iter().position(|message| message.id == id)) {
        messages.drain(..=at);
    }
    messages.iter_mut().for_each(devpit_rpc::cut::cut_message);
    messages
}

/// `chat.part_full` — a tool's output as the transcript keeps it, uncut, found
/// by the call it answers in the message that holds it.
#[tauri::command]
#[specta::specta]
pub async fn chat_part_full(
    relay: tauri::State<'_, crate::chat_relay::Relay>,
    project_id: String,
    conversation_id: String,
    message_id: String,
    call_id: String,
) -> Result<Part, RpcError> {
    conversation(&conversation_id)?;
    if let Some(part) = relay.running_output(&conversation_id, &message_id, &call_id) {
        return Ok(part);
    }
    crate::off_main::blocking(move || {
        let sessions = crate::projects::project_home(&project_id)?.sessions();
        let file = conversation_path(&sessions, &conversation_id);
        whole_output(read(&file).0, &message_id, &call_id).ok_or_else(|| {
            RpcError::new(
                ErrorCode::NotFound,
                "that output is not in this conversation",
            )
        })
    })
    .await
}

/// The tool output answering `call_id` in message `message_id`, as written.
fn whole_output(
    messages: Vec<devpit_rpc::Message>,
    message_id: &str,
    call_id: &str,
) -> Option<Part> {
    messages
        .into_iter()
        .find(|message| message.id == message_id)?
        .parts
        .into_iter()
        .find(
            |part| matches!(part, Part::ToolResult { call_id: answers, .. } if answers == call_id),
        )
}

/// A conversation id is one plain path segment: it names a file in the
/// project's sessions folder and must not reach out of it.
fn conversation(id: &str) -> Result<(), RpcError> {
    if devpit_core::home::plain_id(id) {
        Ok(())
    } else {
        Err(RpcError::new(
            ErrorCode::Invalid,
            "that is not a conversation",
        ))
    }
}

#[cfg(test)]
#[path = "chat_window_tests.rs"]
mod tests;
