//! What ties a card to the others: the epic it was planned under, the cards
//! it waits on, an epic's progress — and its acceptance criteria, read from
//! its body and ticked there.
//!
//! Read once per board: the edges of a project are one query, and a board of
//! thirty cards must not ask thirty times.

use std::collections::{HashMap, HashSet};

use devpit_core::store::column_roles::role_of;
use devpit_core::{CardEdges, Store};
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
        let named = store
            .cards(project_id)?
            .into_iter()
            .map(|row| {
                let card = CardRef {
                    done: done.contains(&row.column_id),
                    id: row.id.clone(),
                    title: row.title,
                };
                (row.id, card)
            })
            .collect();
        let waiting = store
            .plans_waiting(project_id)?
            .into_iter()
            .map(|(epic, cards)| (epic, u32::try_from(cards).unwrap_or_default()))
            .collect();
        Ok(Ties {
            named,
            edges: store.card_edges(project_id)?,
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
    }
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
