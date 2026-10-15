//! A card's number in its project, `#42`: what a person says and types.
//!
//! `card_<ULID>` stays the card's id everywhere; a number only ever leads to
//! it. Numbers are given in the insert itself (`INSERT`), so two writers cannot
//! hand out the same one, and are never reused.

use rusqlite::OptionalExtension;

use crate::store::{Store, StoreError};

/// A new card, taking the next number of its project: id, project, column,
/// title, body, position and the time, as `?1` to `?7`.
pub(crate) const INSERT: &str = "INSERT INTO card \
     (id, project_id, column_id, title, body, position, created_at, updated_at, number) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, \
     (SELECT COALESCE(MAX(number), 0) + 1 FROM card WHERE project_id = ?2))";

impl Store {
    /// The card numbered `number` in the project, archived or not.
    pub fn card_numbered(
        &self,
        project_id: &str,
        number: i64,
    ) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT id FROM card WHERE project_id = ?1 AND number = ?2",
                rusqlite::params![project_id, number],
                |row| row.get(0),
            )
            .optional()?)
    }
}

/// A number for each card an older devpit made on this file, after the
/// highest its project has, in the order they were made.
const NUMBER_THE_REST: &str = "UPDATE card SET number = numbered.top + numbered.n \
     FROM (SELECT c.id, \
             ROW_NUMBER() OVER (PARTITION BY c.project_id ORDER BY c.created_at, c.id) AS n, \
             (SELECT COALESCE(MAX(m.number), 0) FROM card m WHERE m.project_id = c.project_id) AS top \
           FROM card c WHERE c.number IS NULL) AS numbered \
     WHERE card.id = numbered.id";

impl Store {
    /// Numbers the cards an older devpit made without one; how many it did.
    /// Asked first, so a start with nothing to number writes nothing.
    pub fn number_unnumbered(&self) -> Result<usize, StoreError> {
        let any: bool = self.conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM card WHERE number IS NULL)",
            [],
            |row| row.get(0),
        )?;
        if !any {
            return Ok(0);
        }
        let tx = self.writing()?;
        let numbered = tx.execute(NUMBER_THE_REST, [])?;
        tx.commit()?;
        Ok(numbered)
    }

    /// Each card's number in the project, by id: what a card shows of another.
    pub fn card_numbers(
        &self,
        project_id: &str,
    ) -> Result<std::collections::HashMap<String, u32>, StoreError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, number FROM card WHERE project_id = ?1 AND number IS NOT NULL",
        )?;
        let rows = stmt
            .query_map([project_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }
}

/// What a card's tile counts, read for a whole board in one query rather than
/// four per card.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CardTally {
    pub comments: u32,
    pub pinned: u32,
    pub cost_usd: f64,
    pub number: Option<i64>,
}

const TALLY: &str = "SELECT c.id, \
     (SELECT COUNT(*) FROM card_comment WHERE card_id = c.id), \
     (SELECT COUNT(*) FROM card_attachment WHERE card_id = c.id), \
     (SELECT COALESCE(SUM(cost_usd), 0.0) FROM run WHERE card_id = c.id), \
     c.number FROM card c";

impl Store {
    /// Every card of the project, archived ones included, by id.
    pub fn card_tallies(
        &self,
        project_id: &str,
    ) -> Result<std::collections::HashMap<String, CardTally>, StoreError> {
        let mut stmt = self
            .conn
            .prepare_cached(&format!("{TALLY} WHERE c.project_id = ?1"))?;
        let rows = stmt
            .query_map([project_id], tally_row)?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// One card's, or nothing for a card that is not there.
    pub fn card_tally(&self, card_id: &str) -> Result<CardTally, StoreError> {
        Ok(self
            .conn
            .query_row(&format!("{TALLY} WHERE c.id = ?1"), [card_id], tally_row)
            .optional()?
            .map(|(_, tally)| tally)
            .unwrap_or_default())
    }
}

fn tally_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(String, CardTally)> {
    Ok((
        row.get(0)?,
        CardTally {
            comments: row.get(1)?,
            pinned: row.get(2)?,
            cost_usd: row.get(3)?,
            number: row.get(4)?,
        },
    ))
}

/// `#42`, `42` or `devpit#42`: the project named before the `#`, if any, and
/// the number. `None` for anything else, a `card_<ULID>` included.
pub fn card_number_of(text: &str) -> Option<(Option<&str>, i64)> {
    let text = text.trim();
    let (project, digits) = match text.rsplit_once('#') {
        Some((project, digits)) => (Some(project.trim()).filter(|p| !p.is_empty()), digits),
        None => (None, text),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let number = digits.parse().ok().filter(|n: &i64| *n > 0)?;
    Some((project, number))
}

#[cfg(test)]
#[path = "card_numbers_tests.rs"]
mod tests;
