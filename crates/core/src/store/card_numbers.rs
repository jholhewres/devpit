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

    /// A card's number; `None` for one an older devpit made.
    pub fn card_number(&self, card_id: &str) -> Result<Option<i64>, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT number FROM card WHERE id = ?1", [card_id], |row| {
                row.get(0)
            })
            .optional()?
            .flatten())
    }
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
