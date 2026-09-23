//! Adding a project by cloning it. Apart from `projects.rs`, which registers
//! what is already on disk.

use devpit_core::Store;
use devpit_rpc::{ErrorCode, Project, RpcError};

use crate::projects::{drawn, store};

/// `project.clone` — clones a remote and registers where it landed.
///
/// `into` is the parent folder, and it is optional: someone deciding *whether*
/// to add a project should not be stopped to answer *where*. Left out, it goes
/// to `~/.devpit/repos/`, and the path is shown before the clone runs.
#[tauri::command]
#[specta::specta]
pub async fn project_clone(url: String, into: Option<String>) -> Result<Project, RpcError> {
    crate::off_main::blocking(move || project_clone_now(url, into)).await
}

/// [`project_clone`], on the calling thread.
pub(crate) fn project_clone_now(url: String, into: Option<String>) -> Result<Project, RpcError> {
    // Somewhere of their choosing when they chose one. The app's own folder is
    // the answer to "I do not want to decide", not a place to be put.
    let parent = match into.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        Some(chosen) => std::path::PathBuf::from(chosen),
        None => Store::root()?.join("repos"),
    };

    let into = devpit_git::clone(url.trim(), &parent).map_err(|err| match err {
        devpit_git::GitError::Missing => RpcError::new(ErrorCode::Unsupported, err.to_string()),
        // A clone that failed because the folder is taken is a conflict the
        // person can act on, not an internal error.
        devpit_git::GitError::Failed { ref stderr, .. } if stderr.contains("already exists") => {
            RpcError::new(ErrorCode::Conflict, err.to_string())
        }
        other => RpcError::internal(other.to_string()),
    })?;

    let store = store()?;
    let id = store.add_project(&into, Some(url.trim()))?;
    let row = store
        .project(&id)?
        .ok_or_else(|| RpcError::internal("the project vanished between write and read"))?;

    Ok(drawn(&store, row))
}
