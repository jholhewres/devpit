//! A Devflow plan previewed for an epic: kept until the person creates its
//! cards, on the board or through an agent after their yes, or drops it.
//!
//! The cards are a payload `devpit-devflow` shapes; the store keeps the text
//! and makes creating them one transaction, so a yes counted twice cannot
//! make the cards twice.

use rusqlite::OptionalExtension;

use crate::store::board::now;
use crate::store::card_edges::wait_on;
use crate::store::{Store, StoreError};

/// The most a kept plan may weigh. Checked on the way in and before a read
/// allocates it, as the evidence of a run is.
pub const MOST_PLAN: usize = 256 * 1024;

pub struct PlanRow {
    pub epic_id: String,
    /// The planned cards, as `devpit-devflow` wrote them.
    pub cards: String,
    /// The person changed it on the board.
    pub edited: bool,
    pub planned_at: i64,
    /// What was made of it, once it was.
    pub created: Option<String>,
}

/// One card of a plan as it is created: the cards it waits on are indexes
/// into the same plan.
pub struct NewCard<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub waits_on: &'a [usize],
}

impl Store {
    /// Keeps the plan for an epic. A new one replaces what was there, created
    /// or not: it is a new preview.
    pub fn keep_plan(&self, epic_id: &str, cards: &str, edited: bool) -> Result<bool, StoreError> {
        if cards.len() > MOST_PLAN {
            return Ok(false);
        }
        self.conn.execute(
            "INSERT INTO devflow_plan (epic_id, cards, edited, planned_at) VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(epic_id) DO UPDATE SET cards = ?2, edited = ?3, planned_at = ?4, \
             created = NULL, applied_at = NULL",
            rusqlite::params![epic_id, cards, edited, now()],
        )?;
        Ok(true)
    }

    pub fn plan(&self, epic_id: &str) -> Result<Option<PlanRow>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT epic_id, cards, edited, planned_at, created FROM devflow_plan \
                 WHERE epic_id = ?1 AND length(cards) <= ?2",
                rusqlite::params![epic_id, MOST_PLAN as i64],
                |row| {
                    Ok(PlanRow {
                        epic_id: row.get(0)?,
                        cards: row.get(1)?,
                        edited: row.get(2)?,
                        planned_at: row.get(3)?,
                        created: row.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    /// The epics of a project with a plan waiting, and how many cards each
    /// would make: what the board's tiles say.
    pub fn plans_waiting(&self, project_id: &str) -> Result<Vec<(String, i64)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT p.epic_id, json_array_length(p.cards) FROM devflow_plan p \
             JOIN card c ON c.id = p.epic_id \
             WHERE c.project_id = ?1 AND p.applied_at IS NULL AND json_valid(p.cards)",
        )?;
        let rows = stmt
            .query_map([project_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn drop_plan(&self, epic_id: &str) -> Result<bool, StoreError> {
        Ok(self
            .conn
            .execute("DELETE FROM devflow_plan WHERE epic_id = ?1", [epic_id])?
            > 0)
    }

    /// Creates the epic's waiting plan as cards at the end of a lane, under
    /// the epic, with the edges between them, and keeps what `created` says of
    /// them — all of it or none. `None` when the plan is no longer waiting:
    /// another yes got there first.
    pub fn create_plan(
        &self,
        epic_id: &str,
        column_id: &str,
        cards: &[NewCard<'_>],
        created: impl FnOnce(&[String]) -> String,
    ) -> Result<Option<Vec<String>>, StoreError> {
        let tx = self.writing()?;
        let waiting: Option<Option<i64>> = tx
            .query_row(
                "SELECT applied_at FROM devflow_plan WHERE epic_id = ?1",
                [epic_id],
                |row| row.get(0),
            )
            .optional()?;
        if !matches!(waiting, Some(None)) {
            return Ok(None);
        }
        let project_id: String = tx.query_row(
            "SELECT project_id FROM card WHERE id = ?1",
            [epic_id],
            |row| row.get(0),
        )?;
        let mut position: i64 = tx.query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM card WHERE column_id = ?1",
            [column_id],
            |row| row.get(0),
        )?;
        let at = now();
        let mut ids = Vec::with_capacity(cards.len());
        for card in cards {
            let id = format!("card_{}", ulid::Ulid::generate());
            tx.execute(
                super::card_numbers::INSERT,
                rusqlite::params![id, project_id, column_id, card.title, card.body, position, at],
            )?;
            tx.execute(
                "INSERT INTO card_parent (card_id, parent_id) VALUES (?1, ?2)",
                rusqlite::params![id, epic_id],
            )?;
            position += 1;
            ids.push(id);
        }
        for (card, id) in cards.iter().zip(&ids) {
            for on in card.waits_on {
                let on = ids.get(*on).ok_or(StoreError::Circle)?;
                if !wait_on(&tx, id, on, at)? {
                    return Err(StoreError::Circle);
                }
            }
        }
        tx.execute(
            "UPDATE devflow_plan SET created = ?2, applied_at = ?3 WHERE epic_id = ?1",
            rusqlite::params![epic_id, created(&ids), at],
        )?;
        tx.commit()?;
        Ok(Some(ids))
    }
}

#[cfg(test)]
#[path = "plans_tests.rs"]
mod tests;
