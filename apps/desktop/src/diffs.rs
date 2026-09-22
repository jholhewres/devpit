//! What is uncommitted in one file, and what one commit changed.
//!
//! A different question from a card's front, which asks what changed on that
//! line of work. The Changes panel lines a file up against HEAD — the file on
//! disk on one side, `file.at_head` on the other — in the checkout as it
//! stands.

use devpit_rpc::{ErrorCode, RpcError};

use crate::roots::root_of;

/// `file.at_head` — a file as `HEAD` holds it, for the left side of a diff
/// whose right side is the file on disk. `None` when `HEAD` has no such file.
#[tauri::command]
#[specta::specta]
pub fn file_at_head(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
) -> Result<Option<String>, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_core::tree::resolve(&root, &path)
        .map_err(|err| RpcError::new(ErrorCode::Forbidden, err.to_string()))?;
    devpit_git::at_head(&root, &path).map_err(|err| RpcError::internal(err.to_string()))
}

/// `commit.diff` — the patch one commit introduced.
#[tauri::command]
#[specta::specta]
pub fn commit_diff(
    project_id: String,
    worktree_id: Option<String>,
    sha: String,
) -> Result<String, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_git::show(&root, &sha).map_err(|err| RpcError::new(ErrorCode::Conflict, err.to_string()))
}
