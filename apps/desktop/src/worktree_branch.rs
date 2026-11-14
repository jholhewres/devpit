//! Settings → Worktrees → Branch name: what a new card's branch is called.
//!
//! Only new checkouts use it. A card keeps the branch it was made on, so a
//! changed pattern never renames work in progress.

use devpit_core::{preference, Store};
use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::projects::store;

/// The chosen pattern, and the branch it would give a card.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeBranch {
    /// Exactly what was typed; empty while the default is in use.
    pub typed: String,
    pub default: String,
    /// The branch a card titled "Fix the login redirect", numbered 42, gets.
    pub example: String,
}

fn answer(store: &Store) -> Result<WorktreeBranch, RpcError> {
    let typed = store
        .preference(preference::WORKTREE_BRANCH)?
        .unwrap_or_default();
    Ok(WorktreeBranch {
        example: devpit_git::branch_for(
            &typed,
            "Fix the login redirect",
            Some(42),
            "card_01HZXY9K",
        ),
        default: devpit_git::BRANCH_PATTERN.to_owned(),
        typed,
    })
}

/// `worktree.branch_read`
#[tauri::command]
#[specta::specta]
pub async fn worktree_branch_read() -> Result<WorktreeBranch, RpcError> {
    crate::off_main::blocking(|| answer(&store()?)).await
}

/// `worktree.branch_write` — chooses it, or clears it back to the default.
/// Refused, never corrected, when it cannot name a branch.
#[tauri::command]
#[specta::specta]
pub async fn worktree_branch_write(typed: String) -> Result<WorktreeBranch, RpcError> {
    crate::off_main::blocking(move || {
        devpit_git::pattern_allowed(&typed)
            .map_err(|refused| RpcError::new(ErrorCode::Invalid, refused))?;
        let store = store()?;
        store.set_preference(preference::WORKTREE_BRANCH, typed.trim())?;
        answer(&store)
    })
    .await
}
