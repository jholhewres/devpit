//! The engine's work items, kept as cards on one project's board: a card is
//! an item, the card it was planned under its parent, the cards it waits on
//! its `depends_on`, and the lane's role its state.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use devflow_engine::brief;
use devflow_engine::work::{Closed, Comment, Kind, New, State, WorkItem, WorkItems};
use devpit_core::store::column_roles::role_of;
use devpit_core::{CardRow, Store};

/// One project's board, as the engine asks for its work items.
pub(crate) struct Board<'a> {
    pub store: &'a Store,
    pub project_id: &'a str,
}

/// The engine's state for a lane of this role; a lane with none waits.
pub(crate) fn state_of(role: Option<&str>) -> State {
    match role {
        Some("doing") | Some("check") => State::Doing,
        Some("done") => State::Done,
        _ => State::Open,
    }
}

/// The lane role an item in `state` goes to, first choice first.
fn roles_for(state: State) -> &'static [&'static str] {
    match state {
        State::Open => &["backlog"],
        State::Doing => &["doing"],
        State::Done => &["done"],
    }
}

/// When a card was made, from its ULID: cards keep no other creation time.
fn created_of(id: &str) -> String {
    let millis = id
        .strip_prefix("card_")
        .and_then(|ulid| ulid::Ulid::from_string(ulid).ok())
        .map_or(0, |ulid| ulid.timestamp_ms());
    devflow_engine::runs::stamp(UNIX_EPOCH + Duration::from_millis(millis))
}

fn err(why: impl std::fmt::Display) -> String {
    why.to_string()
}

impl Board<'_> {
    /// The card, when it is on this board.
    fn own(&self, id: &str) -> Result<Option<CardRow>, String> {
        if self.store.project_id_of_card(id).map_err(err)?.as_deref() != Some(self.project_id) {
            return Ok(None);
        }
        self.store.card(id).map_err(err)
    }

    fn roles(&self) -> Result<Vec<(String, Option<&'static str>)>, String> {
        self.store.ensure_board(self.project_id).map_err(err)?;
        Ok(self
            .store
            .columns(self.project_id)
            .map_err(err)?
            .into_iter()
            .map(|column| {
                let role = role_of(column.role.as_deref(), &column.name);
                (column.id, role)
            })
            .collect())
    }

    /// The lane for `state`: the one with its role, else the first lane.
    fn lane_for(&self, state: State) -> Result<String, String> {
        let lanes = self.roles()?;
        roles_for(state)
            .iter()
            .find_map(|wanted| lanes.iter().find(|(_, role)| *role == Some(*wanted)))
            .or(lanes.first())
            .map(|(id, _)| id.clone())
            .ok_or_else(|| "this board has no columns".to_owned())
    }

    fn item_of(&self, card: CardRow) -> Result<WorkItem, String> {
        let role = self
            .roles()?
            .into_iter()
            .find(|(id, _)| *id == card.column_id)
            .and_then(|(_, role)| role);
        let closed = match self.store.card_closed(&card.id).map_err(err)? {
            Some(None) => Some(Closed::Cut),
            Some(Some(into)) => Some(Closed::Into(into)),
            None => None,
        };
        let epic = card.title.starts_with(&brief::epic_title(""));
        Ok(WorkItem {
            parent: self.store.card_parent(&card.id).map_err(err)?,
            depends_on: self.store.card_blockers(&card.id).map_err(err)?,
            created: created_of(&card.id),
            kind: if epic { Kind::Epic } else { Kind::Item },
            state: state_of(role),
            closed,
            id: card.id,
            title: card.title,
            body: card.body,
        })
    }

    fn must(&self, id: &str) -> Result<CardRow, String> {
        self.own(id)?
            .ok_or_else(|| format!("no card `{id}` on this project's board"))
    }
}

impl WorkItems for Board<'_> {
    fn get(&self, id: &str) -> Result<Option<WorkItem>, String> {
        self.own(id)?.map(|card| self.item_of(card)).transpose()
    }

    fn list(&self) -> Result<Vec<WorkItem>, String> {
        let mut cards = self.store.cards(self.project_id).map_err(err)?;
        // A card's number is the order it was made in; two ULIDs of the same
        // millisecond are not.
        let numbers = self.store.card_numbers(self.project_id).map_err(err)?;
        cards.sort_by_cached_key(|card| {
            let number = numbers.get(&card.id).copied().unwrap_or(u32::MAX);
            (number, card.id.clone())
        });
        cards.into_iter().map(|card| self.item_of(card)).collect()
    }

    fn create(&self, new: &New, _at: SystemTime) -> Result<WorkItem, String> {
        for id in new.parent.iter().chain(&new.depends_on) {
            self.must(id)?;
        }
        let lane = self.lane_for(State::Open)?;
        let id = self
            .store
            .create_tied_card(
                self.project_id,
                &lane,
                &new.title,
                &new.body,
                new.parent.as_deref(),
                &new.depends_on,
            )
            .map_err(err)?;
        self.item_of(self.must(&id)?)
    }

    fn update_body(&self, id: &str, body: &str) -> Result<WorkItem, String> {
        let card = self.must(id)?;
        self.store.update_card(id, &card.title, body).map_err(err)?;
        self.item_of(self.must(id)?)
    }

    fn retitle(&self, id: &str, title: &str) -> Result<WorkItem, String> {
        let card = self.must(id)?;
        self.store.update_card(id, title, &card.body).map_err(err)?;
        self.item_of(self.must(id)?)
    }

    fn set_state(&self, id: &str, state: State) -> Result<WorkItem, String> {
        self.must(id)?;
        let lane = self.lane_for(state)?;
        // `i64::MAX` is the end of the lane.
        self.store.move_card(id, &lane, i64::MAX).map_err(err)?;
        self.item_of(self.must(id)?)
    }

    fn comment(&self, id: &str, by: &str, text: &str, _at: SystemTime) -> Result<(), String> {
        self.must(id)?;
        let text = crate::cards::sayable(text).map_err(|err| err.message)?;
        self.store.add_comment(id, by, text).map_err(err)?;
        Ok(())
    }

    fn comments(&self, id: &str) -> Result<Vec<Comment>, String> {
        self.must(id)?;
        Ok(self
            .store
            .comments(id)
            .map_err(err)?
            .into_iter()
            .map(|row| Comment {
                by: row.author,
                at: devflow_engine::runs::stamp(
                    UNIX_EPOCH + Duration::from_secs(row.created_at.max(0) as u64),
                ),
                text: row.body,
            })
            .collect())
    }

    fn close(&self, id: &str, closed: Closed) -> Result<WorkItem, String> {
        self.must(id)?;
        let into = match &closed {
            Closed::Cut => None,
            Closed::Into(into) => Some(into.as_str()),
        };
        if !self.store.close_card(id, into).map_err(err)? {
            return Err(format!(
                "`{id}` cannot close into that card: it is itself, closed, or not on this board"
            ));
        }
        self.item_of(self.must(id)?)
    }
}

#[cfg(test)]
#[path = "devflow_work_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "devflow_flow_tests.rs"]
mod flow_tests;
