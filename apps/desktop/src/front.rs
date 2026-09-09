//! What a line of work changed, and putting it away safely.

use devpit_core::Store;
use devpit_rpc::{Board, ErrorCode, Front, RpcError};

use crate::board::board_get;

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
