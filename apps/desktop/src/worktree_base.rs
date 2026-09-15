//! Where new worktrees are created, as a setting.
//!
//! It was a constant, and the constant is still the default. What this adds is
//! the other two answers people actually want: a folder inside the project, so
//! the checkouts travel with it, and one folder outside for every project, so
//! a laptop with one fast disk can put them all on it.
//!
//! The rules live in `crates/git`, next to the code that hands the path to
//! `git worktree add`. This file only carries the answer across.

use std::path::PathBuf;

use devpit_core::{preference, Store};
use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::projects::{locate, store};

/// The chosen base, and what it means for the project in front of you.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeBase {
    /// Exactly what was typed, so the field shows it back unchanged.
    pub typed: String,
    /// Where the next worktree of this project would land. A sentence about
    /// relative and absolute is not an answer; a path is.
    pub example: String,
    /// True while nothing has been chosen and the workspace default is in use.
    pub is_default: bool,
}

fn answer(store: &Store, project_id: Option<&str>) -> Result<WorktreeBase, RpcError> {
    let typed = store
        .preference(preference::WORKTREE_BASE)?
        .unwrap_or_default();
    let home = Store::root()?;

    // Without a project there is nothing to resolve a relative base against,
    // so the example says what it can: the shared default.
    let root = match project_id {
        Some(id) => locate(store, id).map(|(_, root)| root).ok(),
        None => None,
    };
    let root = root.unwrap_or_else(|| devpit_core::home::projects_dir(&home));

    Ok(WorktreeBase {
        example: devpit_git::worktree_at(&typed, &home, &root, "<project>", "<card>")
            .display()
            .to_string(),
        is_default: devpit_git::base_chosen(&typed).is_none(),
        typed,
    })
}

/// `worktree.base_read` — where new worktrees go, and an example of it.
#[tauri::command]
#[specta::specta]
pub fn worktree_base_read(project_id: Option<String>) -> Result<WorktreeBase, RpcError> {
    answer(&store()?, project_id.as_deref())
}

/// `worktree.base_write` — chooses it, or clears it back to the default.
///
/// Refused rather than corrected when the path cannot hold a worktree: a
/// field that silently rewrites what was typed is a field nobody trusts twice.
#[tauri::command]
#[specta::specta]
pub fn worktree_base_write(
    project_id: Option<String>,
    typed: String,
) -> Result<WorktreeBase, RpcError> {
    let store = store()?;

    // Against the project in front of you, because that is what a relative
    // base is relative to. With no project open, only the shape can be
    // checked, which is what `allowed` does with a root that does not exist.
    let root: PathBuf = match project_id.as_deref() {
        Some(id) => locate(&store, id)?.1,
        None => Store::root()?,
    };
    devpit_git::base_allowed(&root, &typed)
        .map_err(|refused| RpcError::new(ErrorCode::Invalid, refused.to_string()))?;

    store.set_preference(preference::WORKTREE_BASE, typed.trim())?;
    answer(&store, project_id.as_deref())
}
