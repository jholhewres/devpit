//! The worktree commands: what exists, what it costs in disk, and removing
//! one without losing work.
//!
//! Nothing here runs on its own. Leaving the last column *offers* to remove a
//! worktree; the person answers.

use std::path::{Path, PathBuf};

use devpit_core::Store;
use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

/// One checkout belonging to a card, or left over from one.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CardWorktree {
    /// The card it belongs to, which is the folder's name.
    pub card_id: String,
    /// Present when a card by that id still exists.
    pub card_title: Option<String>,
    pub folder: String,
    pub branch: Option<String>,
    /// Bytes on disk. A feature that quietly eats forty gigabytes is a feature
    /// people uninstall.
    pub disk_bytes: f64,
    pub uncommitted_files: u32,
    pub uncommitted_lines: u32,
    /// True when the folder is there and the card is not.
    pub orphan: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Worktrees {
    pub worktrees: Vec<CardWorktree>,
    /// The whole set, so the screen can say it in one number.
    pub disk_bytes: f64,
}

/// What removing a worktree would cost, when it refuses.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Removed {
    pub uncommitted_files: u32,
    pub uncommitted_lines: u32,
    /// The branch that stayed behind. The folder is disposable; the commits
    /// in it are not.
    pub branch_kept: Option<String>,
}

fn store() -> Result<Store, RpcError> {
    Ok(Store::open_default()?)
}

pub(crate) fn home() -> Result<PathBuf, RpcError> {
    Store::root().map_err(|err| RpcError::new(ErrorCode::Internal, err.to_string()))
}

fn branch_in(path: &Path) -> Option<String> {
    devpit_git::status(path).ok().map(|read| read.branch)
}

/// `worktree.list` — every checkout this project's cards have, with the disk
/// each one takes and the work each one is holding.
#[tauri::command]
#[specta::specta]
pub async fn worktree_list(project_id: String) -> Result<Worktrees, RpcError> {
    crate::off_main::blocking(move || worktree_list_now(project_id)).await
}

/// [`worktree_list`], on the calling thread.
pub(crate) fn worktree_list_now(project_id: String) -> Result<Worktrees, RpcError> {
    let store = store()?;
    let home = home()?;
    let dir = home.join("worktrees").join(&project_id);

    let mut found = Vec::new();
    let entries = std::fs::read_dir(&dir).into_iter().flatten().flatten();
    for entry in entries {
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        let card_id = entry.file_name().to_string_lossy().into_owned();
        let card = store.card(&card_id).ok().flatten();
        let loss = devpit_git::uncommitted(&path).unwrap_or_default();
        found.push(CardWorktree {
            folder: card_id.clone(),
            card_title: card.as_ref().map(|card| card.title.clone()),
            orphan: card.is_none(),
            branch: branch_in(&path),
            disk_bytes: devpit_git::disk_usage(&path) as f64,
            uncommitted_files: loss.files as u32,
            uncommitted_lines: loss.lines as u32,
            card_id,
        });
    }
    found.sort_by(|a, b| a.folder.cmp(&b.folder));

    Ok(Worktrees {
        disk_bytes: found.iter().map(|one| one.disk_bytes).sum(),
        worktrees: found,
    })
}

/// `worktree.remove` — the folder goes, the branch stays.
///
/// Refuses a checkout with uncommitted work unless `even_dirty`, and the
/// refusal names what would be lost rather than saying "it is dirty".
#[tauri::command]
#[specta::specta]
pub async fn worktree_remove(
    project_id: String,
    card_id: String,
    even_dirty: bool,
) -> Result<Removed, RpcError> {
    crate::off_main::blocking(move || worktree_remove_now(project_id, card_id, even_dirty)).await
}

/// [`worktree_remove`], on the calling thread.
pub(crate) fn worktree_remove_now(
    project_id: String,
    card_id: String,
    even_dirty: bool,
) -> Result<Removed, RpcError> {
    let store = store()?;
    let home = home()?;
    // What the card recorded when the worktree was made, not where the rule
    // would put it now. The two were the same while the base was a constant;
    // the moment it became a setting, changing it would have sent this looking
    // in the new place for a folder sitting in the old one — gone from the
    // screen and still on the disk.
    let path = store
        .card(&card_id)?
        .and_then(|card| card.worktree_path)
        .map(PathBuf::from)
        .unwrap_or_else(|| devpit_git::worktree_home(&home, &project_id, &card_id));
    let main = store
        .project(&project_id)?
        .map(|row| PathBuf::from(row.root_path))
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "that project is not registered"))?;

    let branch = branch_in(&path);
    let loss = devpit_git::remove(&main, &path, even_dirty)
        .map_err(|err| RpcError::new(ErrorCode::Conflict, err.to_string()))?;

    // The card keeps its base_ref: the branch still exists, and forgetting
    // where it started would make the next diff meaningless.
    let _ = store.set_card_front(&card_id, None, None);

    Ok(Removed {
        uncommitted_files: loss.files as u32,
        uncommitted_lines: loss.lines as u32,
        branch_kept: branch,
    })
}
