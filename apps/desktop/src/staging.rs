//! Staging and committing, from the screen.
//!
//! Nothing here happens on its own. Staging is a click, committing is a
//! click, and the message is written by the person or by an agent they asked.

use devpit_rpc::{Commit, ErrorCode, ProjectChanges, RpcError};

use crate::projects::project_changes;
use crate::roots::root_of;

fn git_error(err: devpit_git::GitError) -> RpcError {
    RpcError::new(ErrorCode::Conflict, err.to_string())
}

/// `changes.stage` — puts these paths in the index, and answers with the list
/// as it now stands.
///
/// Answering with the whole list rather than nothing: the screen would
/// otherwise have to predict what staging did to every other row.
#[tauri::command]
#[specta::specta]
pub fn changes_stage(
    project_id: String,
    worktree_id: Option<String>,
    paths: Vec<String>,
) -> Result<ProjectChanges, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_git::stage(&root, &paths).map_err(git_error)?;
    project_changes(project_id, worktree_id)
}

/// `changes.unstage` — takes them back out. The file on disk is not touched.
#[tauri::command]
#[specta::specta]
pub fn changes_unstage(
    project_id: String,
    worktree_id: Option<String>,
    paths: Vec<String>,
) -> Result<ProjectChanges, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_git::unstage(&root, &paths).map_err(git_error)?;
    project_changes(project_id, worktree_id)
}

/// `changes.commit` — commits what is staged.
#[tauri::command]
#[specta::specta]
pub fn changes_commit(
    project_id: String,
    worktree_id: Option<String>,
    message: String,
) -> Result<Commit, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_git::commit(&root, &message).map_err(git_error)
}
