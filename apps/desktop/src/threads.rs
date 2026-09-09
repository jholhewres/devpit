//! Listing the conversations a project has had.
//!
//! Apart from the chat itself because it is the opposite question: the chat
//! is one conversation as it happens, this is every one that already did.

use devpit_rpc::{Conversations, RpcError, Thread};

use crate::chat::home;

/// `chat.list` — every conversation this project has had.
///
/// The transcripts were always on disk; nothing listed them, so closing a tab
/// left a file nobody could reach again.
#[tauri::command]
#[specta::specta]
pub fn chat_list(project_id: String) -> Result<Conversations, RpcError> {
    Ok(Conversations {
        conversations: devpit_agentcli::history::conversations(&home(), &project_id)
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
