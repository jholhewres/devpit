//! What ties one card to another: the epic it was planned under, and the
//! cards it waits on.
//!
//! Apart from `board.rs`, which is the shape of the lanes, and `cards.rs`,
//! which is one card's own rows. An edge goes with either of its cards.

use rusqlite::{Connection, OptionalExtension};

use crate::store::{Store, StoreError};

/// The edges among a project's cards on the board: both ends unarchived, so
/// a card taken off the board neither blocks nor counts.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CardEdges {
    /// A card, and the epic it belongs to.
    pub parents: Vec<(String, String)>,
    /// A card, and a card it waits on.
    pub blockers: Vec<(String, String)>,
    /// A card closed without its own evidence, and the card its work went
    /// into: `None` for one cut from the plan.
    pub closed: Vec<(String, Option<String>)>,
}

impl Store {
    pub fn card_edges(&self, project_id: &str) -> Result<CardEdges, StoreError> {
        let pairs = |sql: &str| -> Result<Vec<(String, String)>, StoreError> {
            let mut stmt = self.conn.prepare(sql)?;
            let rows = stmt
                .query_map([project_id], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        };
        Ok(CardEdges {
            parents: pairs(
                "SELECT e.card_id, e.parent_id FROM card_parent e \
                 JOIN card c ON c.id = e.card_id JOIN card p ON p.id = e.parent_id \
                 WHERE c.project_id = ?1 AND c.archived_at IS NULL AND p.archived_at IS NULL \
                 ORDER BY c.position, c.id",
            )?,
            blockers: pairs(
                "SELECT b.card_id, b.blocked_by FROM card_blocker b \
                 JOIN card c ON c.id = b.card_id JOIN card w ON w.id = b.blocked_by \
                 WHERE c.project_id = ?1 AND c.archived_at IS NULL AND w.archived_at IS NULL \
                 ORDER BY c.position, w.position, b.card_id, b.blocked_by",
            )?,
            closed: self.closed_cards(project_id)?,
        })
    }

    fn closed_cards(&self, project_id: &str) -> Result<Vec<(String, Option<String>)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT s.card_id, s.by_card_id FROM card_superseded s \
             JOIN card c ON c.id = s.card_id \
             WHERE c.project_id = ?1 AND c.archived_at IS NULL ORDER BY s.card_id",
        )?;
        let rows = stmt
            .query_map([project_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Closes `card` without its own evidence: its work went `into` another
    /// card, or with `None` it was cut from the plan. A second close replaces
    /// the first. Refused, as false, into itself, into a card of another
    /// project, or into one closed itself: the work has to land somewhere.
    pub fn close_card(&self, card: &str, into: Option<&str>) -> Result<bool, StoreError> {
        if let Some(into) = into {
            let open_here: Option<i64> = self
                .conn
                .query_row(
                    "SELECT 1 FROM card a JOIN card b ON a.project_id = b.project_id \
                     WHERE a.id = ?1 AND b.id = ?2 \
                     AND NOT EXISTS (SELECT 1 FROM card_superseded WHERE card_id = ?2)",
                    [card, into],
                    |row| row.get(0),
                )
                .optional()?;
            if card == into || open_here.is_none() {
                return Ok(false);
            }
        }
        self.conn.execute(
            "INSERT INTO card_superseded (card_id, by_card_id, at) VALUES (?1, ?2, ?3) \
             ON CONFLICT(card_id) DO UPDATE SET by_card_id = ?2, at = ?3",
            rusqlite::params![card, into, crate::store::board::now()],
        )?;
        Ok(true)
    }

    /// A card at the end of `column_id`, under `parent` and waiting on
    /// `waits_on` — all of it or none. A wait that would circle or cross
    /// projects is refused as [`StoreError::Circle`].
    pub fn create_tied_card(
        &self,
        project_id: &str,
        column_id: &str,
        title: &str,
        body: &str,
        parent: Option<&str>,
        waits_on: &[String],
    ) -> Result<String, StoreError> {
        let tx = self.writing()?;
        let id = format!("card_{}", ulid::Ulid::generate());
        let position: i64 = tx.query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM card WHERE column_id = ?1",
            [column_id],
            |row| row.get(0),
        )?;
        let at = crate::store::board::now();
        tx.execute(
            super::card_numbers::INSERT,
            rusqlite::params![id, project_id, column_id, title, body, position, at],
        )?;
        if let Some(parent) = parent {
            tx.execute(
                "INSERT INTO card_parent (card_id, parent_id) \
                 SELECT ?1, id FROM card WHERE id = ?2 AND project_id = ?3",
                rusqlite::params![id, parent, project_id],
            )?;
        }
        for on in waits_on {
            if !wait_on(&tx, &id, on, at)? {
                return Err(StoreError::Circle);
            }
        }
        tx.commit()?;
        Ok(id)
    }

    /// The epic `card` belongs to, if any.
    pub fn card_parent(&self, card: &str) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT parent_id FROM card_parent WHERE card_id = ?1",
                [card],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// The cards `card` waits on, oldest edge first.
    pub fn card_blockers(&self, card: &str) -> Result<Vec<String>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT blocked_by FROM card_blocker WHERE card_id = ?1 ORDER BY created_at, blocked_by",
        )?;
        let rows = stmt
            .query_map([card], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(rows)
    }

    /// How `card` was closed, when it was: `Some(None)` cut, `Some(Some(id))`
    /// merged into `id`.
    pub fn card_closed(&self, card: &str) -> Result<Option<Option<String>>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT by_card_id FROM card_superseded WHERE card_id = ?1",
                [card],
                |row| row.get(0),
            )
            .optional()?)
    }
}

/// Makes `card` wait on `on`. Refused, as false, for a card waiting on
/// itself, on a card of another project, or on one that already waits on it:
/// a circle of cards never starts.
pub(super) fn wait_on(
    conn: &Connection,
    card: &str,
    on: &str,
    at: i64,
) -> Result<bool, StoreError> {
    let same_project: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM card a JOIN card b ON a.project_id = b.project_id \
             WHERE a.id = ?1 AND b.id = ?2",
            [card, on],
            |row| row.get(0),
        )
        .optional()?;
    if card == on || same_project.is_none() || waits_on(conn, on, card)? {
        return Ok(false);
    }
    conn.execute(
        "INSERT OR IGNORE INTO card_blocker (card_id, blocked_by, created_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![card, on, at],
    )?;
    Ok(true)
}

/// Whether `card` waits on `other`, directly or through cards between them.
fn waits_on(conn: &Connection, card: &str, other: &str) -> Result<bool, StoreError> {
    Ok(conn
        .query_row(
            "WITH RECURSIVE up(id) AS ( \
                 SELECT blocked_by FROM card_blocker WHERE card_id = ?1 \
                 UNION SELECT b.blocked_by FROM card_blocker b JOIN up ON b.card_id = up.id) \
             SELECT 1 FROM up WHERE id = ?2",
            [card, other],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .is_some())
}

#[cfg(test)]
#[path = "card_edges_tests.rs"]
mod tests;
