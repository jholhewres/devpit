//! What of a conversation the window reads: its history with long outputs
//! cut, and one of those outputs whole when a row is opened.

use devpit_agentcli::store::{conversation_path, read};
use devpit_rpc::{Conversation, ErrorCode, Part, RpcError};

use crate::chat::chat_history_now;

/// `chat.history` — everything said in this conversation, in order.
#[tauri::command]
#[specta::specta]
pub async fn chat_history(
    project_id: String,
    conversation_id: String,
) -> Result<Conversation, RpcError> {
    crate::off_main::blocking(move || {
        // The window gets long outputs cut; the transcript keeps them whole,
        // and `chat_part_full` hands one back when it is opened.
        let mut read = chat_history_now(project_id, conversation_id)?;
        read.messages
            .iter_mut()
            .for_each(devpit_rpc::cut::cut_message);
        Ok(read)
    })
    .await
}

/// `chat.part_full` — a tool's output as the transcript keeps it, uncut, found
/// by the call it answers in the message that holds it.
#[tauri::command]
#[specta::specta]
pub async fn chat_part_full(
    project_id: String,
    conversation_id: String,
    message_id: String,
    call_id: String,
) -> Result<Part, RpcError> {
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

#[cfg(test)]
#[path = "chat_window_tests.rs"]
mod tests;
