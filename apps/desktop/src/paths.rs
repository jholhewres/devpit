//! Making, moving and removing a path inside a project.
//!
//! Beside `files.rs` rather than in it: that one reads and writes the
//! *contents* of a file that exists, and these three change which paths exist
//! at all. Every one of them goes through `devpit_core::paths`, which resolves
//! the parent through symlinks and checks the final segment by name — this
//! process runs terminals, and a relative path from a screen is not a path to
//! trust.

use devpit_core::tree::TreeError;
use devpit_rpc::{ErrorCode, RpcError};

use crate::filetree::project_tree;
use crate::roots::root_of;
use devpit_rpc::ProjectTree;

/// Which of these is the person's to fix, and which is ours.
///
/// A name already taken and a path that climbs out are both answerable from
/// the screen — rename it, or pick somewhere else. Anything else is the
/// filesystem refusing, and the message is all we know.
fn refused(err: TreeError) -> RpcError {
    match err {
        TreeError::AlreadyExists { .. } => RpcError::new(ErrorCode::Conflict, err.to_string()),
        TreeError::Outside { .. } => RpcError::new(ErrorCode::Forbidden, err.to_string()),
        other => RpcError::internal(other.to_string()),
    }
}

/// `path.create` — a new empty file, or a new folder.
///
/// Answers with the tree the panel should now draw, the same way
/// `changes.discard` answers with the changes: the screen re-reads rather
/// than predicting what its own click did, so it cannot drift from the disk.
#[tauri::command]
#[specta::specta]
pub fn path_create(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
    folder: bool,
) -> Result<ProjectTree, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_core::paths::create(&root, &path, folder).map_err(refused)?;
    project_tree(project_id, worktree_id, String::new())
}

/// `path.move` — renames or moves, which are the same operation.
#[tauri::command]
#[specta::specta]
pub fn path_move(
    project_id: String,
    worktree_id: Option<String>,
    from: String,
    to: String,
) -> Result<ProjectTree, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_core::paths::move_to(&root, &from, &to).map_err(refused)?;
    project_tree(project_id, worktree_id, String::new())
}

/// `path.delete` — removes a file, or a folder and everything under it.
///
/// The screen confirms first and names what git cannot bring back; by the
/// time this runs, that decision has been made.
#[tauri::command]
#[specta::specta]
pub fn path_delete(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
) -> Result<ProjectTree, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_core::paths::remove(&root, &path).map_err(refused)?;
    project_tree(project_id, worktree_id, String::new())
}
