//! `project.history` — the commits of a checkout, one page at a time.
//!
//! Apart from `projects.rs` so that file has room left: this command carries
//! its own paging logic, and every project-shaped command accreting into one
//! file is how that file stops being readable.

use devpit_rpc::{ProjectHistory, RpcError};

use crate::projects::{checkout, locate, store};

/// How many commits one page carries.
///
/// A number, not "all": `git log` on a large repository walks the whole
/// graph, and the surface shows a handful at a time.
const HISTORY: u32 = 8;

/// `project.history` — the last few commits of a checkout, or the next page
/// of older ones.
///
/// `skip` counts from the newest commit, not from a sha: the caller already
/// knows how many it has drawn, and a count survives a rebase that would
/// invalidate a remembered sha.
#[tauri::command]
#[specta::specta]
pub async fn project_history(
    project_id: String,
    worktree_id: Option<String>,
    skip: Option<u32>,
) -> Result<ProjectHistory, RpcError> {
    crate::off_main::blocking(move || project_history_now(project_id, worktree_id, skip)).await
}

/// [`project_history`], on the calling thread.
pub(crate) fn project_history_now(
    project_id: String,
    worktree_id: Option<String>,
    skip: Option<u32>,
) -> Result<ProjectHistory, RpcError> {
    let store = store()?;
    let (_, root) = locate(&store, &project_id)?;
    let root = checkout(&root, worktree_id.as_deref());

    // One extra commit, asked for and never shown: whether it came back is
    // the answer to "is there more", without a second request to count them.
    let mut commits = devpit_git::history(&root, HISTORY + 1, skip.unwrap_or(0))
        .map_err(|err| RpcError::internal(err.to_string()))?;
    let has_more = commits.len() > HISTORY as usize;
    commits.truncate(HISTORY as usize);

    Ok(ProjectHistory { commits, has_more })
}

/// A commit named in full, and its page on the remote's forge if it has one.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CommitRef {
    pub full: String,
    pub url: Option<String>,
}

/// `project.commit_link` — what a commit's menu copies and opens.
#[tauri::command]
#[specta::specta]
pub async fn project_commit_link(
    project_id: String,
    worktree_id: Option<String>,
    sha: String,
) -> Result<CommitRef, RpcError> {
    crate::off_main::blocking(move || project_commit_link_now(project_id, worktree_id, sha)).await
}

/// [`project_commit_link`], on the calling thread.
pub(crate) fn project_commit_link_now(
    project_id: String,
    worktree_id: Option<String>,
    sha: String,
) -> Result<CommitRef, RpcError> {
    // A commit id, and nothing git would read as an option or a range.
    if sha.is_empty() || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(RpcError::new(
            devpit_rpc::ErrorCode::Invalid,
            "that is not a commit id",
        ));
    }
    let store = store()?;
    let (_, root) = locate(&store, &project_id)?;
    let root = checkout(&root, worktree_id.as_deref());
    let link =
        devpit_git::commit_link(&root, &sha).map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(CommitRef {
        full: link.full,
        url: link.url,
    })
}
