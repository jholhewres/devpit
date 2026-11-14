//! Putting away a card's checkout once its work has landed.
//!
//! On unless turned off: once every commit of a card's branch is in the
//! default branch, its worktree, its build cache and its local branch go. A
//! checkout with uncommitted work, or a session still running in it, is kept
//! and listed. Nothing that is not devpit's own is removed: only a worktree
//! git lists for the project and the card recorded, and a cache folder named
//! for the card.

use std::path::{Path, PathBuf};

use devpit_core::{preference, Store};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SweptWhy {
    Landed,
    Uncommitted,
    Running,
    Failed,
}

/// One card the cleanup removed or kept, and why.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Swept {
    pub card_id: String,
    pub title: String,
    pub why: SweptWhy,
    pub bytes: f64,
    /// What git said, when it refused.
    pub detail: Option<String>,
}

/// What one cleanup did.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CleanupRun {
    /// Seconds since the epoch.
    pub at: f64,
    pub removed: Vec<Swept>,
    pub kept: Vec<Swept>,
    pub branches: u32,
    pub freed_bytes: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rules {
    pub delete_landed: bool,
}

/// Where devpit's own folders are, apart from its home.
pub(crate) struct Places {
    /// The build caches: a card's checkout builds into `wt-<card id>` here.
    pub cache: Option<PathBuf>,
}

pub(crate) fn rules_of(store: &Store) -> Rules {
    Rules {
        delete_landed: store
            .preference_flag(preference::WORKTREES_DELETE_MERGED)
            .ok()
            .flatten()
            .unwrap_or(true),
    }
}

/// `~/.cache/devpit-target`.
pub(crate) fn build_caches() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|home| PathBuf::from(home).join(".cache").join("devpit-target"))
}

/// The card's own cache, for an id that cannot climb out of `caches`.
fn card_cache(caches: &Path, card_id: &str) -> Option<PathBuf> {
    card_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        .then(|| caches.join(format!("wt-{card_id}")))
}

/// Removes a real folder, never a link to one elsewhere, and says how much it
/// held.
fn drop_dir(path: &Path) -> Option<u64> {
    if !std::fs::symlink_metadata(path).ok()?.is_dir() {
        return None;
    }
    let bytes = devpit_git::disk_usage(path);
    std::fs::remove_dir_all(path).ok()?;
    Some(bytes)
}

/// Every project's landed cards, put away. `live` says whether a session
/// still runs in a card's checkout.
pub(crate) fn sweep(
    store: &Store,
    places: &Places,
    rules: &Rules,
    now: i64,
    live: &dyn Fn(&str, &Path) -> bool,
) -> CleanupRun {
    let mut run = CleanupRun {
        at: now as f64,
        ..CleanupRun::default()
    };
    if !rules.delete_landed {
        return run;
    }
    for project in store.projects().unwrap_or_default() {
        let root = PathBuf::from(&project.root_path);
        for card in store.cards(&project.id).unwrap_or_default() {
            sweep_card(store, places, &root, &card, live, &mut run);
        }
    }
    run
}

fn sweep_card(
    store: &Store,
    places: &Places,
    root: &Path,
    card: &devpit_core::store::CardRow,
    live: &dyn Fn(&str, &Path) -> bool,
    run: &mut CleanupRun,
) {
    let Some(began) = card.base_ref.as_deref() else {
        return;
    };
    let checkout = card
        .worktree_path
        .as_deref()
        .map(PathBuf::from)
        .filter(|path| path.is_dir() && devpit_git::landing::linked_checkout(root, path));
    let recorded = store.card_branch(&card.id).ok().flatten();
    let branch = match &checkout {
        Some(path) => devpit_git::branch_at(path)
            .ok()
            .filter(|name| !name.is_empty()),
        None => devpit_git::branch_left(root, &card.id, recorded.as_deref()),
    };
    let Some(branch) = branch else {
        return;
    };
    if !devpit_git::landing::landed(root, &branch, began) {
        return;
    }
    let row = |why, bytes: u64, detail| Swept {
        card_id: card.id.clone(),
        title: card.title.clone(),
        why,
        bytes: bytes as f64,
        detail,
    };

    let mut freed = 0;
    if let Some(path) = &checkout {
        if live(&card.id, path) {
            run.kept.push(row(SweptWhy::Running, 0, None));
            return;
        }
        match devpit_git::uncommitted(path) {
            Ok(loss) if loss.is_empty() => {}
            Ok(_) => {
                run.kept.push(row(SweptWhy::Uncommitted, 0, None));
                return;
            }
            Err(err) => {
                run.kept
                    .push(row(SweptWhy::Failed, 0, Some(err.to_string())));
                return;
            }
        }
        let bytes = devpit_git::disk_usage(path);
        if let Err(err) = devpit_git::remove(root, path, false) {
            run.kept
                .push(row(SweptWhy::Failed, 0, Some(err.to_string())));
            return;
        }
        freed += bytes;
        let _ = store.forget_card_checkout(&card.id);
    }
    let cache = places
        .cache
        .as_deref()
        .and_then(|caches| card_cache(caches, &card.id));
    freed += cache.as_deref().and_then(drop_dir).unwrap_or(0);
    if devpit_git::landing::drop_landed(root, &branch, &card.id, recorded.as_deref(), began).is_ok()
    {
        run.branches += 1;
    }
    if checkout.is_some() || freed > 0 {
        run.removed.push(row(SweptWhy::Landed, freed, None));
        run.freed_bytes += freed as f64;
    }
}

/// One line for the log.
pub(crate) fn said(run: &CleanupRun) -> String {
    format!(
        "worktree cleanup: removed {} merged worktree(s) and {} branch(es), kept {}; freed {}",
        run.removed.len(),
        run.branches,
        run.kept.len(),
        size(run.freed_bytes)
    )
}

/// `1.5 GB`, as the Settings screen writes it.
fn size(bytes: f64) -> String {
    let mut size = bytes;
    let mut unit = 0;
    let units = ["B", "KB", "MB", "GB", "TB"];
    while size >= 1024.0 && unit < units.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit < 2 {
        format!("{} {}", size.round(), units[unit])
    } else {
        format!("{size:.1} {}", units[unit])
    }
}

#[cfg(test)]
#[path = "worktree_sweep_tests.rs"]
mod tests;
