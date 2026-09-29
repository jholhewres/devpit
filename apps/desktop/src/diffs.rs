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
pub async fn file_at_head(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
) -> Result<Option<String>, RpcError> {
    crate::off_main::blocking(move || file_at_head_now(project_id, worktree_id, path)).await
}

/// [`file_at_head`], on the calling thread.
pub(crate) fn file_at_head_now(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
) -> Result<Option<String>, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    if !names_inside(&path) {
        return Err(RpcError::new(
            ErrorCode::Forbidden,
            format!("{path} is not a path inside the checkout"),
        ));
    }
    devpit_git::at_head(&root, &path).map_err(|err| RpcError::internal(err.to_string()))
}

/// Whether `path` names something under the checkout, judged by its words.
///
/// Not resolved on disk: a deleted file is exactly the one whose `HEAD` side is
/// asked for, and git reads it from the commit, never through a symlink.
fn names_inside(path: &str) -> bool {
    use std::path::Component;
    !path.is_empty()
        && std::path::Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

/// `commit.diff` — the patch one commit introduced.
#[tauri::command]
#[specta::specta]
pub async fn commit_diff(
    project_id: String,
    worktree_id: Option<String>,
    sha: String,
) -> Result<String, RpcError> {
    crate::off_main::blocking(move || commit_diff_now(project_id, worktree_id, sha)).await
}

/// [`commit_diff`], on the calling thread.
pub(crate) fn commit_diff_now(
    project_id: String,
    worktree_id: Option<String>,
    sha: String,
) -> Result<String, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_git::show(&root, &sha).map_err(|err| RpcError::new(ErrorCode::Conflict, err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::names_inside;

    #[test]
    fn a_deleted_file_is_asked_for_by_name_and_nothing_climbs_out() {
        assert!(names_inside("src/gone.rs"));
        assert!(!names_inside("../outside.rs"));
        assert!(!names_inside("src/../../outside.rs"));
        assert!(!names_inside("/etc/passwd"));
        assert!(!names_inside(""));
    }
}
