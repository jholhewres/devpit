//! Which kinds of checkout the lists are about.
//!
//! Three at once live in one repository when a person works with agents:
//! devpit's own card worktrees, the ones Claude Code makes for itself, and
//! whatever they made by hand. Added together they were a number on the
//! project row that nobody could account for.
//!
//! Hiding one is a view, never a deletion. Nothing here touches git and
//! nothing here touches the disk — a hidden checkout is still there, still
//! listed by `git worktree list`, and unhiding it brings it straight back.

use std::path::PathBuf;

use devpit_core::{preference, Store};
use devpit_rpc::{RpcError, WorktreeOrigin};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::projects::{locate, store};

/// One kind of checkout, with the count that makes the choice meaningful.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    /// `devpit`, `claude`, `other` — what the preference stores.
    pub id: String,
    pub label: String,
    /// Where these sit, in the shape a person would recognise.
    pub hint: String,
    /// How many this project has right now. Hiding a kind it has none of is a
    /// switch that does nothing, and the count is what says so.
    pub count: u32,
    pub shown: bool,
}

const KINDS: &[(WorktreeOrigin, &str, &str)] = &[
    (WorktreeOrigin::Devpit, "devpit", "the worktree folder"),
    (WorktreeOrigin::Claude, "Claude Code", ".claude/worktrees"),
    (WorktreeOrigin::Other, "Other locations", "made by hand"),
];

/// Every folder devpit treats as its own.
///
/// Both the workspace default and whatever the setting names: changing the
/// setting must not reclassify the checkouts made under the old one as
/// somebody else's and drop them out of the list that created them.
pub(crate) fn ours(store: &Store, project_root: &std::path::Path) -> Vec<PathBuf> {
    let Ok(home) = Store::root() else {
        return Vec::new();
    };
    let mut folders = vec![home.join("worktrees")];

    if let Ok(Some(typed)) = store.preference(preference::WORKTREE_BASE) {
        if let Some(base) = devpit_git::base_chosen(&typed) {
            let path = PathBuf::from(base);
            folders.push(if path.is_absolute() {
                path
            } else {
                project_root.join(path)
            });
        }
    }
    folders
}

/// What was hidden, as stored.
pub(crate) fn hidden(store: &Store) -> String {
    store
        .preference(preference::WORKTREES_HIDDEN)
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// `worktree.sources` — the kinds this project has, and which are shown.
#[tauri::command]
#[specta::specta]
pub async fn worktree_sources(project_id: Option<String>) -> Result<Vec<Source>, RpcError> {
    crate::off_main::blocking(move || worktree_sources_now(project_id)).await
}

/// [`worktree_sources`], on the calling thread.
pub(crate) fn worktree_sources_now(project_id: Option<String>) -> Result<Vec<Source>, RpcError> {
    let store = store()?;
    let stored = hidden(&store);

    // Counted before anything is hidden, so a switch that would empty the
    // list still says how much it would hide.
    let found = match project_id.as_deref() {
        Some(id) => {
            let root = locate(&store, id)?.1;
            devpit_git::worktrees(&root, &ours(&store, &root)).unwrap_or_default()
        }
        None => Vec::new(),
    };

    Ok(KINDS
        .iter()
        .map(|(origin, label, hint)| Source {
            id: devpit_git::word_of(*origin).to_owned(),
            label: (*label).to_owned(),
            hint: (*hint).to_owned(),
            count: found
                .iter()
                .filter(|worktree| worktree.origin == *origin)
                .count() as u32,
            shown: devpit_git::shown(&stored, *origin),
        })
        .collect())
}

/// `worktree.source_show` — shows or hides one kind.
#[tauri::command]
#[specta::specta]
pub async fn worktree_source_show(
    project_id: Option<String>,
    source_id: String,
    shown: bool,
) -> Result<Vec<Source>, RpcError> {
    crate::off_main::blocking(move || worktree_source_show_now(project_id, source_id, shown)).await
}

/// [`worktree_source_show`], on the calling thread.
pub(crate) fn worktree_source_show_now(
    project_id: Option<String>,
    source_id: String,
    shown: bool,
) -> Result<Vec<Source>, RpcError> {
    let store = store()?;
    let mut set = devpit_git::hidden_in(&hidden(&store));

    // Matched against what this build knows rather than stored as it arrived:
    // the preference keys a filter, and a word nothing recognises would sit
    // there forever hiding nothing and explaining nothing.
    let Some(word) = [
        WorktreeOrigin::Devpit,
        WorktreeOrigin::Claude,
        WorktreeOrigin::Other,
    ]
    .into_iter()
    .map(devpit_git::word_of)
    .find(|known| *known == source_id) else {
        return Err(RpcError::new(
            devpit_rpc::ErrorCode::Invalid,
            "no such kind of worktree",
        ));
    };

    if shown {
        set.remove(word);
    } else {
        set.insert(word);
    }
    store.set_preference(preference::WORKTREES_HIDDEN, &devpit_git::hidden_as(&set))?;

    worktree_sources_now(project_id)
}
