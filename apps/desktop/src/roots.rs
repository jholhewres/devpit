//! Which directory a path belongs to.
//!
//! A project has a root, and a line of work may have a worktree of its own.
//! Every command that touches a path starts here, because "the project" and
//! "the worktree this card runs in" are two different directories and reading
//! the wrong one shows somebody else's tree.

use std::path::PathBuf;

use devpit_core::Store;
use devpit_rpc::{ErrorCode, RpcError};

fn store() -> Result<Store, RpcError> {
    Ok(Store::open_default()?)
}

/// The directory to resolve a path against.
///
/// A worktree id with no worktree behind it falls back to the project root
/// rather than failing. A branch gets merged and pruned while its card is
/// still on the board, and that card's files should still open.
pub(crate) fn root_of(project_id: &str, worktree_id: Option<&str>) -> Result<PathBuf, RpcError> {
    let store = store()?;
    let row = store
        .project(project_id)?
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "that project is not registered"))?;
    let root = PathBuf::from(&row.root_path);
    if let Some(id) = worktree_id {
        if let Some(path) = devpit_git::worktree_path(&root, id)
            .map_err(|err| RpcError::internal(err.to_string()))?
        {
            return Ok(path);
        }
    }
    Ok(root)
}
