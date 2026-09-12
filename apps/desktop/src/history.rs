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
pub fn project_history(
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
