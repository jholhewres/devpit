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

use crate::filetree::project_tree_now;
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
        other => crate::filetree::tree_error(other),
    }
}

/// `path.create` — a new empty file, or a new folder.
///
/// Answers with the tree the panel should now draw, the same way
/// `changes.discard` answers with the changes: the screen re-reads rather
/// than predicting what its own click did, so it cannot drift from the disk.
#[tauri::command]
#[specta::specta]
pub async fn path_create(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
    folder: bool,
) -> Result<ProjectTree, RpcError> {
    crate::off_main::blocking(move || path_create_now(project_id, worktree_id, path, folder)).await
}

/// [`path_create`], on the calling thread.
pub(crate) fn path_create_now(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
    folder: bool,
) -> Result<ProjectTree, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_core::paths::create(&root, &path, folder).map_err(refused)?;
    project_tree_now(project_id, worktree_id, String::new())
}

/// `path.move` — renames or moves, which are the same operation.
#[tauri::command]
#[specta::specta]
pub async fn path_move(
    project_id: String,
    worktree_id: Option<String>,
    from: String,
    to: String,
) -> Result<ProjectTree, RpcError> {
    crate::off_main::blocking(move || path_move_now(project_id, worktree_id, from, to)).await
}

/// [`path_move`], on the calling thread.
pub(crate) fn path_move_now(
    project_id: String,
    worktree_id: Option<String>,
    from: String,
    to: String,
) -> Result<ProjectTree, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_core::paths::move_to(&root, &from, &to).map_err(refused)?;
    project_tree_now(project_id, worktree_id, String::new())
}

/// `path.delete` — removes a file, or a folder and everything under it.
///
/// The screen confirms first and names what git cannot bring back; by the
/// time this runs, that decision has been made.
#[tauri::command]
#[specta::specta]
pub async fn path_delete(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
) -> Result<ProjectTree, RpcError> {
    crate::off_main::blocking(move || path_delete_now(project_id, worktree_id, path)).await
}

/// [`path_delete`], on the calling thread.
pub(crate) fn path_delete_now(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
) -> Result<ProjectTree, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_core::paths::remove(&root, &path).map_err(refused)?;
    project_tree_now(project_id, worktree_id, String::new())
}

/// A file dropped from outside is copied whole; past this it is not one to
/// carry into a checkout by dragging.
const MOST_IMPORTED: u64 = 512 * 1024 * 1024;

/// `path.import` — files dropped from outside, copied into `folder` under
/// their own names. A name already there is refused, never written over.
#[tauri::command]
#[specta::specta]
pub async fn path_import(
    project_id: String,
    worktree_id: Option<String>,
    folder: String,
    sources: Vec<String>,
) -> Result<ProjectTree, RpcError> {
    crate::off_main::blocking(move || {
        let root = root_of(&project_id, worktree_id.as_deref())?;
        for source in &sources {
            import(&root, &folder, std::path::Path::new(source))?;
        }
        project_tree_now(project_id, worktree_id, String::new())
    })
    .await
}

/// One file into `folder` of `root`, held to the project like every write.
pub(crate) fn import(
    root: &std::path::Path,
    folder: &str,
    from: &std::path::Path,
) -> Result<(), RpcError> {
    let meta = from
        .metadata()
        .map_err(|err| RpcError::new(ErrorCode::NotFound, format!("{}: {err}", from.display())))?;
    if !meta.is_file() {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            format!("{} is not a file", from.display()),
        ));
    }
    if meta.len() > MOST_IMPORTED {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            format!("{} is too large to drop in", from.display()),
        ));
    }
    let name = from
        .file_name()
        .and_then(|one| one.to_str())
        .ok_or_else(|| RpcError::new(ErrorCode::Invalid, "a file with no name"))?;
    let relative = if folder.trim_matches('/').is_empty() {
        name.to_owned()
    } else {
        format!("{}/{name}", folder.trim_matches('/'))
    };
    let target = devpit_core::paths::resolve_new(root, &relative).map_err(refused)?;
    if target.symlink_metadata().is_ok() {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            format!("{relative} is already there"),
        ));
    }
    std::fs::copy(from, &target).map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::import;

    #[test]
    fn a_dropped_file_lands_in_the_folder_and_nowhere_else() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("repo");
        std::fs::create_dir_all(root.join("docs")).expect("mkdir");
        std::fs::create_dir_all(root.join(".git")).expect("mkdir");
        let picked = dir.path().join("shot.png");
        std::fs::write(&picked, "png").expect("write");

        import(&root, "docs", &picked).expect("imported");
        assert!(root.join("docs/shot.png").is_file());
        assert!(
            import(&root, "docs", &picked).is_err(),
            "wrote over a file already there"
        );
        assert!(
            import(&root, "..", &picked).is_err(),
            "climbed out of the project"
        );
        assert!(
            import(&root, ".git/hooks", &picked).is_err(),
            "wrote into .git"
        );
    }
}
