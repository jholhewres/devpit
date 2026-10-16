//! What ties a card to the others: the epic it was planned under, the cards
//! it waits on, an epic's progress — and its acceptance criteria, read from
//! its body and ticked there.
//!
//! Read once per board: the edges of a project are one query, and a board of
//! thirty cards must not ask thirty times.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use devpit_core::store::column_roles::role_of;
use devpit_core::{CardEdges, Store};
use devpit_devflow::config::UnblockOn;
use devpit_rpc::{Card, CardRef, Criterion, ErrorCode, Progress, RpcError};

/// The role of the lane where finished work sits.
const DONE: &str = "done";

/// A project's cards as other cards name them, and the edges between them.
pub(crate) struct Ties {
    named: HashMap<String, CardRef>,
    edges: CardEdges,
    waiting: HashMap<String, u32>,
}

impl Ties {
    pub(crate) fn read(store: &Store, project_id: &str) -> Result<Ties, RpcError> {
        let done: HashSet<String> = store
            .columns(project_id)?
            .into_iter()
            .filter(|column| role_of(column.role.as_deref(), &column.name) == Some(DONE))
            .map(|column| column.id)
            .collect();
        let numbers = store.card_numbers(project_id)?;
        let rows = store.cards(project_id)?;
        let edges = store.card_edges(project_id)?;
        let lane_done: HashSet<&str> = rows
            .iter()
            .filter(|row| done.contains(&row.column_id))
            .map(|row| row.id.as_str())
            .collect();
        let on_board: HashSet<&str> = rows.iter().map(|row| row.id.as_str()).collect();
        // Only a card something waits on, or whose work another took, is worth its gate.
        let asked: HashSet<&str> = edges
            .blockers
            .iter()
            .map(|(_, by)| by.as_str())
            .chain(edges.closed.iter().filter_map(|(_, into)| into.as_deref()))
            .filter(|id| !lane_done.contains(id))
            .collect();
        let passed = gates_passed(store, project_id, &asked);
        let closed: HashMap<String, Option<String>> = edges.closed.iter().cloned().collect();
        let free =
            |id: &str| lane_done.contains(id) || passed.contains(id) || !on_board.contains(id);
        let named = rows
            .iter()
            .map(|row| {
                let card = CardRef {
                    done: lane_done.contains(row.id.as_str()),
                    released: !lane_done.contains(row.id.as_str())
                        && releases(&row.id, &free, &closed),
                    number: numbers.get(&row.id).copied(),
                    id: row.id.clone(),
                    title: row.title.clone(),
                };
                (row.id.clone(), card)
            })
            .collect();
        let waiting = store
            .plans_waiting(project_id)?
            .into_iter()
            .map(|(epic, cards)| (epic, u32::try_from(cards).unwrap_or_default()))
            .collect();
        Ok(Ties {
            named,
            edges,
            waiting,
        })
    }

    pub(crate) fn parent(&self, id: &str) -> Option<CardRef> {
        self.edges
            .parents
            .iter()
            .find(|(card, _)| card == id)
            .and_then(|(_, parent)| self.named.get(parent).cloned())
    }

    /// An epic's cards, in the order the board keeps them.
    pub(crate) fn children(&self, id: &str) -> Vec<CardRef> {
        self.edges
            .parents
            .iter()
            .filter(|(_, parent)| parent == id)
            .filter_map(|(card, _)| self.named.get(card).cloned())
            .collect()
    }

    fn blockers(&self, id: &str) -> Vec<CardRef> {
        self.edges
            .blockers
            .iter()
            .filter(|(card, _)| card == id)
            .filter_map(|(_, by)| self.named.get(by).cloned())
            .collect()
    }

    /// The card with what ties it to the others.
    pub(crate) fn fill(&self, card: &mut Card) {
        card.parent_id = self.parent(&card.id).map(|parent| parent.id);
        card.blocked_by = self.blockers(&card.id);
        card.progress = progress(&self.children(&card.id));
        card.plan_waiting = self.waiting.get(&card.id).copied().unwrap_or_default();
        let closed = self.edges.closed.iter().find(|(id, _)| *id == card.id);
        card.cut = matches!(closed, Some((_, None)));
        card.superseded_by = closed
            .and_then(|(_, into)| into.as_ref())
            .and_then(|into| self.named.get(into).cloned());
    }
}

/// Whether the cards waiting on `id` may start: it is `free` (done, or past
/// its gate where the plan says so), cut, or its work went into a card that
/// would release them.
pub(crate) fn releases(
    id: &str,
    free: &dyn Fn(&str) -> bool,
    closed: &HashMap<String, Option<String>>,
) -> bool {
    let mut at = id;
    // A merge never lands on a closed card, so the chain ends; bounded anyway.
    for _ in 0..=closed.len() {
        if free(at) {
            return true;
        }
        match closed.get(at) {
            None => return false,
            Some(None) => return true,
            Some(Some(into)) => at = into,
        }
    }
    false
}

/// The cards among `asked` whose newest gate passed, where devflow.yaml says
/// `stages.plan.unblock_on: gate`; none otherwise.
fn gates_passed(store: &Store, project_id: &str, asked: &HashSet<&str>) -> HashSet<String> {
    if asked.is_empty() {
        return HashSet::new();
    }
    let Some(row) = store.project(project_id).ok().flatten() else {
        return HashSet::new();
    };
    let root = Path::new(&row.root_path);
    let devpit_devflow::State::Valid(_, config) = crate::devflow::read(root) else {
        return HashSet::new();
    };
    if config.unblock_on() != UnblockOn::Gate {
        return HashSet::new();
    }
    let Ok(runs) = crate::devflow_evidence::runs_of(project_id, root) else {
        return HashSet::new();
    };
    asked
        .iter()
        .filter(|card| crate::devflow_gate::gate_passed(&runs, card))
        .map(|card| (*card).to_owned())
        .collect()
}

/// How far an epic's cards are, or `None` for a card with none.
pub(crate) fn progress(children: &[CardRef]) -> Option<Progress> {
    (!children.is_empty()).then(|| Progress {
        done: children.iter().filter(|child| child.done).count() as u32,
        total: children.len() as u32,
    })
}

/// A card answered alone, with its ties.
pub(crate) fn tied(store: &Store, project_id: &str, mut card: Card) -> Result<Card, RpcError> {
    Ties::read(store, project_id)?.fill(&mut card);
    Ok(card)
}

/// The criteria of a body, as a card carries them.
pub(crate) fn criteria_of(body: &str) -> Vec<Criterion> {
    devpit_core::criteria::criteria(body)
        .into_iter()
        .map(|one| Criterion {
            text: one.text,
            done: one.done,
        })
        .collect()
}

/// `card.criterion_tick` — the person ticks, or unticks, one of a card's
/// acceptance criteria. It is an edit of the body, where the criterion lives.
#[tauri::command]
#[specta::specta]
pub async fn card_criterion_tick(
    project_id: String,
    card_id: String,
    at: u32,
    done: bool,
) -> Result<Card, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::board::store()?;
        let row = store
            .card(&card_id)?
            .filter(|_| {
                store.project_id_of_card(&card_id).ok().flatten().as_deref() == Some(&project_id)
            })
            .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "no such card"))?;
        let body = devpit_core::criteria::ticked(&row.body, at as usize, done)
            .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "the card has no such criterion"))?;
        crate::board::card_update_now(project_id, card_id, row.title, body)
    })
    .await
}

#[cfg(test)]
#[path = "card_ties_tests.rs"]
mod tests;
