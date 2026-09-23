//! Staging and committing, from the screen.
//!
//! Nothing here happens on its own. Staging is a click, committing is a
//! click, and the message is written by the person or by an agent they asked.

use devpit_rpc::{Commit, ErrorCode, ProjectChanges, RpcError};

use crate::projects::project_changes_now;
use crate::roots::root_of;

fn git_error(err: devpit_git::GitError) -> RpcError {
    RpcError::new(ErrorCode::Conflict, err.to_string())
}

/// Refuses any path that resolves outside the project root.
///
/// Discard writes to disk — it restores a file or deletes it — so it goes
/// through the same check every read here already does, rather than trusting
/// git to catch an escape on its own. `resolve_new`, not `resolve`: the
/// most common target is a deleted file, which is not there to canonicalise.
fn resolved(root: &std::path::Path, paths: &[String]) -> Result<(), RpcError> {
    for path in paths {
        devpit_core::paths::resolve_new(root, path)
            .map_err(|err| RpcError::forbidden(err.to_string()))?;
    }
    Ok(())
}

/// `changes.stage` — puts these paths in the index, and answers with the list
/// as it now stands.
///
/// Answering with the whole list rather than nothing: the screen would
/// otherwise have to predict what staging did to every other row.
#[tauri::command]
#[specta::specta]
pub async fn changes_stage(
    project_id: String,
    worktree_id: Option<String>,
    paths: Vec<String>,
) -> Result<ProjectChanges, RpcError> {
    crate::off_main::blocking(move || changes_stage_now(project_id, worktree_id, paths)).await
}

/// [`changes_stage`], on the calling thread.
pub(crate) fn changes_stage_now(
    project_id: String,
    worktree_id: Option<String>,
    paths: Vec<String>,
) -> Result<ProjectChanges, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_git::stage(&root, &paths).map_err(git_error)?;
    project_changes_now(project_id, worktree_id)
}

/// `changes.unstage` — takes them back out. The file on disk is not touched.
#[tauri::command]
#[specta::specta]
pub async fn changes_unstage(
    project_id: String,
    worktree_id: Option<String>,
    paths: Vec<String>,
) -> Result<ProjectChanges, RpcError> {
    crate::off_main::blocking(move || changes_unstage_now(project_id, worktree_id, paths)).await
}

/// [`changes_unstage`], on the calling thread.
pub(crate) fn changes_unstage_now(
    project_id: String,
    worktree_id: Option<String>,
    paths: Vec<String>,
) -> Result<ProjectChanges, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_git::unstage(&root, &paths).map_err(git_error)?;
    project_changes_now(project_id, worktree_id)
}

/// `changes.commit` — commits what is staged.
#[tauri::command]
#[specta::specta]
pub async fn changes_commit(
    project_id: String,
    worktree_id: Option<String>,
    message: String,
) -> Result<Commit, RpcError> {
    crate::off_main::blocking(move || changes_commit_now(project_id, worktree_id, message)).await
}

/// [`changes_commit`], on the calling thread.
pub(crate) fn changes_commit_now(
    project_id: String,
    worktree_id: Option<String>,
    message: String,
) -> Result<Commit, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_git::commit(&root, &message).map_err(git_error)
}

/// `changes.discard` — throws away uncommitted work in these paths.
///
/// A tracked change goes back to the index or HEAD; a path git has never
/// recorded — untracked, or added but never committed — has no earlier
/// version to go back to and is deleted outright. The screen confirms first,
/// because that second case cannot be undone from here.
#[tauri::command]
#[specta::specta]
pub async fn changes_discard(
    project_id: String,
    worktree_id: Option<String>,
    paths: Vec<String>,
) -> Result<ProjectChanges, RpcError> {
    crate::off_main::blocking(move || changes_discard_now(project_id, worktree_id, paths)).await
}

/// [`changes_discard`], on the calling thread.
pub(crate) fn changes_discard_now(
    project_id: String,
    worktree_id: Option<String>,
    paths: Vec<String>,
) -> Result<ProjectChanges, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    resolved(&root, &paths)?;
    devpit_git::discard(&root, &paths).map_err(git_error)?;
    project_changes_now(project_id, worktree_id)
}

#[cfg(test)]
#[path = "staging_tests.rs"]
mod tests;
