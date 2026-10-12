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
        })
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
