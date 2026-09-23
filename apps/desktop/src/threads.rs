//! Listing the conversations a project has had.
//!
//! Apart from the chat itself because it is the opposite question: the chat
//! is one conversation as it happens, this is every one that already did.

use devpit_rpc::{Conversations, RpcError, Thread};

/// `chat.list` — every conversation this project has had.
///
/// The transcripts were always on disk; nothing listed them, so closing a tab
/// left a file nobody could reach again.
#[tauri::command]
#[specta::specta]
pub async fn chat_list(project_id: String) -> Result<Conversations, RpcError> {
    crate::off_main::blocking(move || chat_list_now(project_id)).await
}

/// [`chat_list`], on the calling thread.
pub(crate) fn chat_list_now(project_id: String) -> Result<Conversations, RpcError> {
    Ok(Conversations {
        conversations: devpit_agentcli::history::conversations(
            &crate::projects::project_home(&project_id)?.sessions(),
        )
        .into_iter()
        .map(|one| Thread {
            id: one.id,
            title: one.title,
            profile: one.profile,
            model: one.model,
            cost_usd: one.cost_usd,
            last_at: one.last_at,
        })
        .collect(),
    })
}
