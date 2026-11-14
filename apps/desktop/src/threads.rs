//! Listing the conversations a project has had.
//!
//! Apart from the chat itself because it is the opposite question: the chat
//! is one conversation as it happens, this is every one that already did.

use devpit_agentcli::head::{marks_path, write_marks};
use devpit_rpc::{ConversationMarks, Conversations, ErrorCode, RpcError, Thread};

/// `chat.list` — every conversation this project has had, but the archived
/// ones, which `chat.archived` lists.
///
/// The transcripts were always on disk; nothing listed them, so closing a tab
/// left a file nobody could reach again.
#[tauri::command]
#[specta::specta]
pub async fn chat_list(project_id: String) -> Result<Conversations, RpcError> {
    crate::off_main::blocking(move || chat_list_now(project_id)).await
}

/// [`chat_list`], on the calling thread. An archived conversation is not listed.
pub(crate) fn chat_list_now(project_id: String) -> Result<Conversations, RpcError> {
    listed(project_id, false)
}

/// `chat.archived` — the project's archived conversations, for Settings → Archive.
#[tauri::command]
#[specta::specta]
pub async fn chat_archived(project_id: String) -> Result<Conversations, RpcError> {
    crate::off_main::blocking(move || listed(project_id, true)).await
}

fn listed(project_id: String, archived: bool) -> Result<Conversations, RpcError> {
    Ok(Conversations {
        conversations: devpit_agentcli::history::listed(
            &crate::projects::project_home(&project_id)?.sessions(),
            archived,
        )
        .into_iter()
        .map(|one| Thread {
            id: one.id,
            title: one.title,
            profile: one.profile,
            model: one.model,
            cost_usd: one.cost_usd,
            last_at: one.last_at,
            cwd: one.cwd,
            marks: one.marks,
        })
        .collect(),
    })
}

/// `chat.mark` — keeps what the person set on a conversation: its name, a
/// pin, settled, snoozed or unread. Answers the marks as kept.
#[tauri::command]
#[specta::specta]
pub async fn chat_mark(
    project_id: String,
    conversation_id: String,
    marks: ConversationMarks,
) -> Result<ConversationMarks, RpcError> {
    crate::off_main::blocking(move || chat_mark_now(project_id, conversation_id, marks)).await
}

/// [`chat_mark`], on the calling thread.
pub(crate) fn chat_mark_now(
    project_id: String,
    conversation_id: String,
    marks: ConversationMarks,
) -> Result<ConversationMarks, RpcError> {
    let sessions = crate::projects::project_home(&project_id)?.sessions();
    if !devpit_core::home::plain_id(&conversation_id)
        || !sessions.join(format!("{conversation_id}.jsonl")).is_file()
    {
        return Err(RpcError::new(ErrorCode::NotFound, "no such conversation"));
    }
    write_marks(&marks_path(&sessions, &conversation_id), marks)
        .map_err(|err| RpcError::internal(err.to_string()))
}
