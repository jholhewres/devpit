//! What a line of work changed, and putting it away safely.

use devpit_core::Store;
use devpit_rpc::{Board, CardDeleted, DeleteRefusal, ErrorCode, Front, RpcError};
use tauri::State;

use crate::board::board_get;
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
pub fn card_diff(card_id: String) -> Result<Front, RpcError> {
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

/// `card.archive` — and it refuses while the front holds unsaved work.
///
/// `force` is the person saying they know. Nothing here decides on its own
/// that work nobody committed was not worth keeping.
#[tauri::command]
#[specta::specta]
pub fn card_archive(project_id: String, card_id: String, force: bool) -> Result<Board, RpcError> {
    let store = store()?;

    if !force {
        if let Some(path) = store.card(&card_id)?.and_then(|card| card.worktree_path) {
            let unsaved = devpit_git::unsaved_in(std::path::Path::new(&path))
                .map_err(|err| RpcError::internal(err.to_string()))?;
            if !unsaved.is_empty() {
                return Err(RpcError::new(
                    ErrorCode::Conflict,
                    format!(
                        "{} change{} in {path} that nothing has saved — archive anyway?",
                        unsaved.len(),
                        if unsaved.len() == 1 { "" } else { "s" }
                    ),
                ));
            }
        }
    }

    store.archive_card(&card_id)?;
    board_get(project_id)
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
pub fn card_delete(
    state: State<SessionState>,
    project_id: String,
    card_id: String,
    force: bool,
) -> Result<CardDeleted, RpcError> {
    let store = store()?;
    if store.project_id_of_card(&card_id)?.as_deref() != Some(project_id.as_str()) {
        return Err(RpcError::new(ErrorCode::NotFound, "no such card"));
    }
    let card = store
        .card(&card_id)?
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "no such card"))?;
    let running = store
        .runs(&card_id)?
        .iter()
        .filter(|run| run.state == "running")
        .count();

    let tab = tab_for_card(&card_id);
    let leaves: Vec<String> = layout_of(&project_id, &tab)
        .map(|layout| {
            layout
                .tree
                .leaves()
                .into_iter()
                .map(|(leaf, _)| leaf.to_owned())
                .collect()
        })
        .unwrap_or_default();
    // Only asked when the card has a terminal: it spawns tmux and ps.
    let agent = if leaves.is_empty() {
        None
    } else {
        crate::shell_launch::running_in(&project_id)?
            .into_iter()
            .find(|pane| pane.agent.is_some() && leaves.contains(&pane.pane_id))
            .map(|pane| pane.label)
    };

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
        crate::arranging::close_tab(&state, &project_id, &tab)?;
    }
    Ok(CardDeleted {
        deleted: store.delete_card(&card_id)?,
        refused: None,
    })
}

#[cfg(test)]
#[path = "front_tests.rs"]
mod tests;
