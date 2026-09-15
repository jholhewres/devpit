//! A conversation about a card, started from the card.
//!
//! It runs in the card's checkout for its whole life: the folder is fixed on
//! its head when it begins, so a turn after the card is gone still resumes
//! where its session is.

use std::path::Path;

use devpit_agentcli::head::Head;
use devpit_core::Store;
use devpit_rpc::{ErrorCode, RpcError};

use crate::adopting::{plain, write_listed};
use crate::card_activity::{chat_key, heard_now, Doing};

/// Whether a conversation may be fixed to this folder: the card's checkout or
/// the project root, and nowhere a turn would find somebody else's work.
pub(crate) fn fixable_cwd(root: &Path, checkout: Option<&Path>, cwd: &Path) -> bool {
    cwd == root || checkout == Some(cwd)
}

/// Writes a card's conversation: its head fixed to `cwd`, an empty transcript,
/// and the card's link to it.
///
/// No profile until the first turn picks one, when the window has not: a
/// conversation's account is fixed by the turn that spends through it.
pub(crate) fn open_card_chat(
    store: &Store,
    sessions: &Path,
    project_id: &str,
    card_id: &str,
    conversation_id: &str,
    cwd: &Path,
    profile_id: Option<String>,
) -> Result<(), RpcError> {
    let (_, root) = crate::projects::locate(store, project_id)?;
    let card = store
        .card(card_id)?
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "no such card"))?;
    if !fixable_cwd(&root, card.worktree_path.as_deref().map(Path::new), cwd) {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "a card's conversation runs in its checkout or in the project",
        ));
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as f64)
        .unwrap_or_default();
    let head = Head {
        profile: profile_id.unwrap_or_default(),
        model: None,
        created_at: now,
        cost_usd: 0.0,
        budget_usd: None,
        session_id: None,
        permission: None,
        effort: None,
        title: Some(card.title),
        rewind: Default::default(),
        cwd: Some(cwd.display().to_string()),
    };
    write_listed(sessions, conversation_id, &head)
        .map_err(|err| RpcError::internal(err.to_string()))?;
    store.link_chat(card_id, conversation_id)?;
    Ok(())
}

/// A turn of a card's conversation, heard on its card.
pub(crate) fn turn_heard(app: &tauri::AppHandle, conversation_id: &str, state: Doing) {
    let card = crate::projects::store()
        .ok()
        .and_then(|store| store.chat_card(conversation_id).ok().flatten());
    if let Some(card) = card {
        heard_now(app, chat_key(&card, conversation_id), state);
    }
}

/// `card.chat` — a new conversation about this card, in its checkout.
///
/// Answers the conversation's id, which the window opens as a chat tab.
#[tauri::command]
#[specta::specta]
pub async fn card_chat(
    project_id: String,
    card_id: String,
    profile_id: Option<String>,
) -> Result<String, RpcError> {
    if !plain(&project_id) || !plain(&card_id) || profile_id.as_deref().is_some_and(|id| !plain(id))
    {
        return Err(RpcError::new(ErrorCode::Forbidden, "that is not a card"));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let store = crate::projects::store()?;
        if store.live_card_project(&card_id)?.as_deref() != Some(project_id.as_str()) {
            return Err(RpcError::new(
                ErrorCode::NotFound,
                "no such card in this project",
            ));
        }
        // Made here when the card has none: a conversation about a card's work
        // happens where that work is.
        let checkout = crate::checkout::checkout_of(&store, &card_id, |_| {})
            .map_err(|why| RpcError::new(ErrorCode::Internal, why))?;
        let conversation_id = format!("conv_{}", ulid::Ulid::generate());
        let sessions = crate::projects::project_home(&project_id)?.sessions();
        open_card_chat(
            &store,
            &sessions,
            &project_id,
            &card_id,
            &conversation_id,
            &checkout,
            profile_id,
        )?;
        Ok(conversation_id)
    })
    .await
    .map_err(|err| RpcError::internal(err.to_string()))?
}

#[cfg(test)]
#[path = "card_chat_tests.rs"]
mod tests;
