//! A conversation about a card, started from the card.
//!
//! It runs in the card's checkout for its whole life: the folder is fixed on
//! its head when it begins, so a turn after the card is gone still resumes
//! where its session is.

use std::path::Path;

use devpit_agentcli::head::Head;
use devpit_core::store::CardLinks;
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
        context: None,
    };
    write_listed(sessions, conversation_id, &head)
        .map_err(|err| RpcError::internal(err.to_string()))?;
    store.link_chat(card_id, conversation_id)?;
    Ok(())
}

/// Where a session taken into a chat goes on, by the plan's cwd decision: the
/// folder its run recorded, the folder its background session recorded, or the
/// card's checkout for a session found in the card's terminal.
///
/// Never a folder made for it: a checkout created now is not where the session
/// is, and a folder nobody recorded is refused rather than guessed.
pub(crate) fn adopted_cwd(
    links: &CardLinks,
    session_id: &str,
    checkout: Option<&str>,
) -> Result<String, RpcError> {
    let recorded = |cwd: Option<&String>| {
        cwd.filter(|cwd| !cwd.is_empty()).cloned().ok_or_else(|| {
            RpcError::new(
                ErrorCode::Invalid,
                "the folder this session ran in was not recorded, so it cannot go on in a chat",
            )
        })
    };
    if let Some(run) = links.runs.iter().find(|run| run.session_id == session_id) {
        return recorded(run.cwd.as_ref());
    }
    if let Some(background) = links
        .background
        .as_ref()
        .filter(|background| background.session_id == session_id)
    {
        return recorded(background.cwd.as_ref());
    }
    checkout
        .filter(|checkout| !checkout.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            RpcError::new(
                ErrorCode::Invalid,
                "this card has no checkout for the session to go on in",
            )
        })
}

/// The folder a card's session goes on in, with the card checked against the
/// project and the folder against what a conversation may be fixed to.
pub(crate) fn card_session_cwd(
    store: &Store,
    project_id: &str,
    card_id: &str,
    session_id: &str,
) -> Result<String, RpcError> {
    if store.live_card_project(card_id)?.as_deref() != Some(project_id) {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "no such card in this project",
        ));
    }
    let (_, root) = crate::projects::locate(store, project_id)?;
    let checkout = store.card(card_id)?.and_then(|card| card.worktree_path);
    let cwd = adopted_cwd(&store.card_links(card_id)?, session_id, checkout.as_deref())?;
    if !fixable_cwd(&root, checkout.as_deref().map(Path::new), Path::new(&cwd)) {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "this session ran outside its card's checkout and the project",
        ));
    }
    Ok(cwd)
}

/// The card a conversation is filed under, as the chat's header draws it.
#[derive(Default)]
pub(crate) struct ConversationCard {
    pub id: Option<String>,
    pub title: Option<String>,
    pub on_board: bool,
}

pub(crate) fn conversation_card(
    store: &Store,
    conversation_id: &str,
) -> Result<ConversationCard, RpcError> {
    let Some(id) = store.chat_card(conversation_id)? else {
        return Ok(ConversationCard::default());
    };
    Ok(ConversationCard {
        title: store.card(&id)?.map(|card| card.title),
        on_board: store.live_card_project(&id)?.is_some(),
        id: Some(id),
    })
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
