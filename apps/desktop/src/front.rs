//! What a line of work changed, and putting it away safely.

use devpit_core::Store;
use devpit_rpc::{
    ArchivedCard, ArchivedCards, Board, CardDeleted, DeleteRefusal, ErrorCode, Front, RpcError,
};
use tauri::State;

use crate::board::board_get_now;
use crate::sessions::{layout_of, tab_for_card, SessionState};

fn store() -> Result<Store, RpcError> {
    Ok(Store::open_default()?)
}

/// `card.diff` — what this front changed, against the ref it began from.
///
/// Never against HEAD: that answers a different question, and drifts further
/// from this one with every commit anyone lands on the base branch.
#[tauri::command]
#[specta::specta]
pub async fn card_diff(card_id: String) -> Result<Front, RpcError> {
    crate::off_main::blocking(move || card_diff_now(card_id)).await
}

/// [`card_diff`], on the calling thread.
pub(crate) fn card_diff_now(card_id: String) -> Result<Front, RpcError> {
    let store = store()?;
    let card = store
        .card(&card_id)?
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "no such card"))?;

    let (Some(path), Some(base)) = (&card.worktree_path, &card.base_ref) else {
        // A card with no front of its own is not an error: most cards never
        // get one.
        return Ok(Front {
            worktree_path: card.worktree_path,
            base_ref: card.base_ref,
            files: Vec::new(),
            diff: String::new(),
            unsaved: Vec::new(),
        });
    };

    let worktree = std::path::Path::new(path);
    Ok(Front {
        files: devpit_git::changed_since(worktree, base)
            .map_err(|err| RpcError::internal(err.to_string()))?,
        diff: devpit_git::diff_since(worktree, base)
            .map_err(|err| RpcError::internal(err.to_string()))?,
        unsaved: devpit_git::unsaved_in(worktree)
            .map_err(|err| RpcError::internal(err.to_string()))?,
        worktree_path: card.worktree_path,
        base_ref: card.base_ref,
    })
}

/// `card.archive` — and it refuses while the front holds unsaved work, or an
/// agent works in the card's terminal.
///
/// `force` is the person saying they know. Nothing here decides on its own
/// that work nobody committed was not worth keeping.
#[tauri::command]
#[specta::specta]
pub async fn card_archive(
    app: tauri::AppHandle,
    project_id: String,
    card_id: String,
    force: bool,
) -> Result<Board, RpcError> {
    crate::off_main::blocking(move || {
        let state = tauri::Manager::state::<SessionState>(&app);
        card_archive_now(&state, project_id, card_id, force)
    })
    .await
}

/// [`card_archive`], on the calling thread.
pub(crate) fn card_archive_now(
    state: &SessionState,
    project_id: String,
    card_id: String,
    force: bool,
) -> Result<Board, RpcError> {
    let store = store()?;
    let leaves = card_tab_leaves(&project_id, &card_id);

    if !force {
        let agent = live_agent_in(&project_id, &leaves)?;
        let path = store.card(&card_id)?.and_then(|card| card.worktree_path);
        let unsaved = match &path {
            Some(path) => devpit_git::unsaved_in(std::path::Path::new(path))
                .map_err(|err| RpcError::internal(err.to_string()))?
                .len(),
            None => 0,
        };
        let path = path.unwrap_or_default();
        if let Some(refused) = archive_refusal(agent.as_deref(), unsaved, &path) {
            return Err(RpcError::new(ErrorCode::Conflict, refused));
        }
    }

    // Archived is put away: a terminal left open on it is one nothing lists.
    if !leaves.is_empty() {
        close_card_tab(state, &project_id, &card_id)?;
    }
    store.archive_card(&card_id)?;
    crate::card_activity::card_ended(&card_id);
    board_get_now(project_id)
}

/// Why `card.archive` asks first, or nothing. Every answer can be forced:
/// archiving is the person's call, once they know what it stops.
pub(crate) fn archive_refusal(
    live_agent: Option<&str>,
    unsaved: usize,
    path: &str,
) -> Option<String> {
    if let Some(agent) = live_agent {
        return Some(format!(
            "{agent} is running in this card's terminal, and archiving stops it — archive anyway?"
        ));
    }
    (unsaved > 0).then(|| {
        format!(
            "{unsaved} change{} in {path} that nothing has saved — archive anyway?",
            if unsaved == 1 { "" } else { "s" }
        )
    })
}

/// The panes of a card's terminal tab, none when it has no tab.
fn card_tab_leaves(project_id: &str, card_id: &str) -> Vec<String> {
    layout_of(project_id, &tab_for_card(card_id))
        .map(|layout| {
            layout
                .tree
                .leaves()
                .into_iter()
                .map(|(leaf, _)| leaf.to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// The agent open in one of these panes, by the name a person knows it by.
fn live_agent_in(project_id: &str, leaves: &[String]) -> Result<Option<String>, RpcError> {
    // Only asked when the card has a terminal: it spawns tmux and ps.
    if leaves.is_empty() {
        return Ok(None);
    }
    Ok(crate::shell_launch::running_in(project_id)?
        .into_iter()
        .find(|pane| pane.agent.is_some() && leaves.contains(&pane.pane_id))
        .map(|pane| pane.label))
}

/// Closes a card's terminal tab, and tells the window, which lets a tab
/// closed from here go the way it does one an orchestrator stopped.
fn close_card_tab(state: &SessionState, project_id: &str, card_id: &str) -> Result<(), RpcError> {
    let tab_id = tab_for_card(card_id);
    let closed = crate::arranging::close_tab(state, project_id, &tab_id)?;
    crate::card_activity::panes_closed(state.app(), &closed);
    let _ = tauri::Emitter::emit(
        state.app(),
        crate::stopping::TAB_CLOSED,
        serde_json::json!({ "projectId": project_id, "tabId": tab_id }),
    );
    Ok(())
}

/// Why `card.delete` will not go ahead, or nothing.
///
/// A run in flight and an agent in the card's terminal are work happening now,
/// and no `force` deletes them. Unsaved changes are the person's to throw away.
pub(crate) fn delete_refusal(
    running_runs: usize,
    live_agent_in_tab: Option<&str>,
    unsaved: usize,
    force: bool,
) -> Option<DeleteRefusal> {
    if running_runs > 0 {
        return Some(DeleteRefusal {
            reason: "a run is still going on this card — stop it first".to_owned(),
            forcible: false,
        });
    }
    if let Some(agent) = live_agent_in_tab {
        return Some(DeleteRefusal {
            reason: format!("{agent} is running in this card's terminal — close it first"),
            forcible: false,
        });
    }
    if unsaved > 0 && !force {
        return Some(DeleteRefusal {
            reason: format!(
                "{unsaved} change{} in the checkout that nothing has saved — delete anyway?",
                if unsaved == 1 { "" } else { "s" }
            ),
            forcible: true,
        });
    }
    None
}

/// `card.delete` — the card and what hangs off it, never its checkout or branch.
#[tauri::command]
#[specta::specta]
pub async fn card_delete(
    app: tauri::AppHandle,
    project_id: String,
    card_id: String,
    force: bool,
) -> Result<CardDeleted, RpcError> {
    crate::off_main::blocking(move || {
        let state = tauri::Manager::state::<SessionState>(&app);
        card_delete_now(state, project_id, card_id, force)
    })
    .await
}

/// [`card_delete`], on the calling thread.
pub(crate) fn card_delete_now(
    state: State<SessionState>,
    project_id: String,
    card_id: String,
    force: bool,
) -> Result<CardDeleted, RpcError> {
    let store = store()?;
    of_project(&store, &project_id, &card_id)?;
    let card = store
        .card(&card_id)?
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "no such card"))?;
    let running = store
        .runs(&card_id)?
        .iter()
        .filter(|run| run.state == "running")
        .count();

    let leaves = card_tab_leaves(&project_id, &card_id);
    let agent = live_agent_in(&project_id, &leaves)?;

    let unsaved = match card.worktree_path.as_deref().map(std::path::Path::new) {
        Some(path) if path.is_dir() => devpit_git::unsaved_in(path)
            .map_err(|err| RpcError::internal(err.to_string()))?
            .len(),
        _ => 0,
    };

    if let Some(refused) = delete_refusal(running, agent.as_deref(), unsaved, force) {
        return Ok(CardDeleted {
            deleted: false,
            refused: Some(refused),
        });
    }
    if !leaves.is_empty() {
        close_card_tab(&state, &project_id, &card_id)?;
    }
    let deleted = store.delete_card(&card_id)?;
    if deleted {
        crate::card_activity::card_ended(&card_id);
    }
    Ok(CardDeleted {
        deleted,
        refused: None,
    })
}

/// A card id from the window is only acted on inside the project it names.
fn of_project(store: &Store, project_id: &str, card_id: &str) -> Result<(), RpcError> {
    if store.project_id_of_card(card_id)?.as_deref() == Some(project_id) {
        Ok(())
    } else {
        Err(RpcError::new(ErrorCode::NotFound, "no such card"))
    }
}

/// How many archived cards `board.archived` answers with.
const ARCHIVED_AT_MOST: i64 = 200;

/// `board.archived` — the cards off the board, newest first.
#[tauri::command]
#[specta::specta]
pub async fn board_archived(project_id: String) -> Result<ArchivedCards, RpcError> {
    crate::off_main::blocking(move || board_archived_now(project_id)).await
}

/// [`board_archived`], on the calling thread.
pub(crate) fn board_archived_now(project_id: String) -> Result<ArchivedCards, RpcError> {
    let cards = store()?
        .archived_cards(&project_id, ARCHIVED_AT_MOST)?
        .into_iter()
        .map(|row| ArchivedCard {
            id: row.id,
            title: row.title,
            column_name: row.column_name,
            archived_at: row.archived_at as f64,
        })
        .collect();
    Ok(ArchivedCards { cards })
}

/// `card.restore` — an archived card back on the board.
#[tauri::command]
#[specta::specta]
pub async fn card_restore(project_id: String, card_id: String) -> Result<Board, RpcError> {
    crate::off_main::blocking(move || card_restore_now(project_id, card_id)).await
}

/// [`card_restore`], on the calling thread.
pub(crate) fn card_restore_now(project_id: String, card_id: String) -> Result<Board, RpcError> {
    let store = store()?;
    of_project(&store, &project_id, &card_id)?;
    if !store.restore_card(&card_id)? {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "that card is not archived",
        ));
    }
    board_get_now(project_id)
}

#[cfg(test)]
#[path = "front_tests.rs"]
mod tests;
