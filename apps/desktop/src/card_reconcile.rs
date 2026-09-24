//! What the process table says about a card's panes when no hook has.
//!
//! Hooks are an agent's own word about itself. After a restart the app has
//! heard nothing, and a card pane with an agent in front would show nothing at
//! all. The process table fills that in — never over a newer hook: every change
//! is stamped with an order read before tmux and `ps` were asked.

use std::collections::HashMap;
use std::sync::Mutex;

use devpit_core::Store;
use devpit_rpc::{CardHappening, PaneRunning, SessionKind};

use crate::card_activity::{forget_before, hear, registry, Activities, Doing, Key, Place};
use crate::sessions::{card_of_tab, decode};

/// Which card a pane belongs to, and in which tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CardPane {
    pub card_id: String,
    pub tab_id: String,
}

/// Every leaf of a project's card tabs, by leaf, in one scan of the layouts.
///
/// Only cards still on that project's board: an archived card's tab is not a
/// card anybody can look at.
pub(crate) fn card_leaves(store: &Store, project_id: &str) -> HashMap<String, CardPane> {
    let Ok(layouts) = store.card_tab_layouts() else {
        return HashMap::new();
    };
    let mut leaves = HashMap::new();
    // Scoped to the project by the card, not the layout: a tab filed under one
    // project for another project's card is not that project's pane.
    for layout in layouts {
        let Some(card) = card_of_tab(&layout.tab_id) else {
            continue;
        };
        if store.live_card_project(card).ok().flatten().as_deref() != Some(project_id) {
            continue;
        }
        let Ok(decoded) = decode(&layout.project_id, &layout.tree, &layout.focused_id) else {
            continue;
        };
        for (leaf, _) in decoded.tree.leaves() {
            leaves.insert(
                leaf.to_owned(),
                CardPane {
                    card_id: card.to_owned(),
                    tab_id: layout.tab_id.clone(),
                },
            );
        }
    }
    leaves
}

/// What the process table changes on a card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Change {
    /// An agent is in front of a card pane nobody has heard from: it is open.
    Open { key: Key, place: Place },
    /// A pane the card knew is one tmux no longer lists.
    Gone(Key),
}

/// The rule, apart from tmux, `ps` and the registry.
///
/// Never by age: a pane quiet for an hour is as open as one quiet for a second,
/// and a timer would be a guess about an agent that can say for itself.
pub(crate) fn reconciled(
    leaves: &HashMap<String, CardPane>,
    fronts: &[PaneRunning],
    known: &[Key],
) -> Vec<Change> {
    let mut changes = Vec::new();
    for front in fronts.iter().filter(|front| front.agent.is_some()) {
        let Some(pane) = leaves.get(&front.pane_id) else {
            continue;
        };
        let key = Key {
            card_id: pane.card_id.clone(),
            kind: SessionKind::Pane,
            reference: front.pane_id.clone(),
        };
        if !known.contains(&key) {
            let place = Place {
                tab_id: Some(pane.tab_id.clone()),
                leaf_id: Some(front.pane_id.clone()),
            };
            changes.push(Change::Open { key, place });
        }
    }
    for key in known
        .iter()
        .filter(|key| key.kind == SessionKind::Pane && leaves.contains_key(&key.reference))
    {
        if !fronts.iter().any(|front| front.pane_id == key.reference) {
            changes.push(Change::Gone(key.clone()));
        }
    }
    changes
}

/// Reconciles against the registry, stamping every change `seq`.
///
/// `seq` was read before tmux and `ps` were asked, so a hook heard since then
/// is newer and wins: a pane it spoke for is not taken off its card. An `open`
/// is stamped below every hook instead, so even a hook accepted before `seq`
/// and applied after it still says more. Answers the cards that changed.
pub(crate) fn reconcile_in(
    activities: &Mutex<Activities>,
    leaves: &HashMap<String, CardPane>,
    fronts: &[PaneRunning],
    seq: u64,
) -> Vec<CardHappening> {
    if leaves.is_empty() {
        return Vec::new();
    }
    let Ok(mut activities) = activities.lock() else {
        return Vec::new();
    };
    let known = activities.pane_keys();
    reconciled(leaves, fronts, &known)
        .into_iter()
        .filter_map(|change| match change {
            // The weakest word, at the bottom of the order: a hook already on
            // its way when tmux was asked still lands over it.
            Change::Open { key, place } => hear(&mut activities, key, 0, Doing::Open, place),
            Change::Gone(key) => forget_before(&mut activities, &key, seq),
        })
        .collect()
}

/// The process table's word on a project's card panes, told to the window.
pub(crate) fn reconcile(
    app: &tauri::AppHandle,
    project_id: &str,
    fronts: &[PaneRunning],
    seq: u64,
) {
    let Ok(store) = crate::projects::store() else {
        return;
    };
    let leaves = card_leaves(&store, project_id);
    for happening in reconcile_in(registry(), &leaves, fronts, seq) {
        let _ = tauri::Emitter::emit(app, "card:happening", happening);
    }
}

/// The project to rebuild first: the one the person was last in.
///
/// Pure so the choice can be read without a store. `projects()` already
/// answers most-recently-opened first, and this says out loud that the order
/// is the rule rather than an accident of the query.
pub(crate) fn last_opened(projects: &[devpit_core::ProjectRow]) -> Option<String> {
    projects.first().map(|row| row.id.clone())
}

/// Rebuilds what the registry knew, for the project the window will open on.
///
/// The registry is in memory, so a restart starts it empty and every card says
/// nothing until its agent speaks again. This asks tmux and the process table
/// once, on its own thread — `setup` returns without waiting, because the same
/// two questions were measured at about forty milliseconds each and this runs
/// before the window has painted.
///
/// One project, not all of them: the others are rebuilt by the poll that
/// already runs when they are opened.
pub(crate) fn rebuild_on_start(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        // A store that would not open was kept where the error was made.
        let Ok(store) = crate::projects::store() else {
            return;
        };
        let projects = match store.projects() {
            Ok(projects) => projects,
            Err(err) => return devpit_core::reports::background("card reconcile", &err),
        };
        let Some(project_id) = last_opened(&projects) else {
            return;
        };
        // Stamped before tmux and `ps` are asked, so a hook heard in between
        // is newer and wins.
        let seq = crate::card_activity::next_seq();
        let fronts = crate::shell_launch::running_in(&project_id).unwrap_or_default();
        reconcile(&app, &project_id, &fronts, seq);
    });
}

#[cfg(test)]
#[path = "card_reconcile_tests.rs"]
mod tests;
