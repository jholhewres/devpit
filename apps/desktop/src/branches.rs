//! Which branch this checkout is on, and moving it.

use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::roots::root_of;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    pub name: String,
    pub current: bool,
    /// The subject of its last commit, so a name nobody remembers still says
    /// what it is.
    pub subject: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Branches {
    pub branches: Vec<Branch>,
}

fn git_error(err: devpit_git::GitError) -> RpcError {
    RpcError::new(ErrorCode::Conflict, err.to_string())
}

fn read(root: &std::path::Path) -> Result<Branches, RpcError> {
    Ok(Branches {
        branches: devpit_git::branches(root)
            .map_err(git_error)?
            .into_iter()
            .map(|branch| Branch {
                name: branch.name,
                current: branch.current,
                subject: branch.subject,
            })
            .collect(),
    })
}

/// `branch.list` — the local branches, the current one first.
#[tauri::command]
#[specta::specta]
pub async fn branch_list(
    project_id: String,
    worktree_id: Option<String>,
) -> Result<Branches, RpcError> {
    crate::off_main::blocking(move || branch_list_now(project_id, worktree_id)).await
}

/// [`branch_list`], on the calling thread.
pub(crate) fn branch_list_now(
    project_id: String,
    worktree_id: Option<String>,
) -> Result<Branches, RpcError> {
    read(&root_of(&project_id, worktree_id.as_deref())?)
}

/// `branch.switch` — moves the checkout, or says why git would not.
///
/// git's own refusal is passed through: it names the files that would be
/// lost, and a summary here would name fewer.
#[tauri::command]
#[specta::specta]
pub async fn branch_switch(
    project_id: String,
    worktree_id: Option<String>,
    name: String,
) -> Result<Branches, RpcError> {
    crate::off_main::blocking(move || branch_switch_now(project_id, worktree_id, name)).await
}

/// [`branch_switch`], on the calling thread.
pub(crate) fn branch_switch_now(
    project_id: String,
    worktree_id: Option<String>,
    name: String,
) -> Result<Branches, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    devpit_git::switch(&root, &name).map_err(git_error)?;
    read(&root)
}
