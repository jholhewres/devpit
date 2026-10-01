//! A card's date as a reminder: when it goes off, and when it was dealt with.
//!
//! Not a table of its own. A reminder is a card with a date — what somebody
//! asked to be told about at a time is work on a board, and a date already
//! lives on the card. What this adds is the two moments around it: reminded
//! and handled, both cleared when the date changes.

use rusqlite::OptionalExtension;

use crate::store::{Store, StoreError};

/// A card with a date, as the reminders read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderRow {
    pub card_id: String,
    pub project_id: String,
    pub title: String,
    /// Seconds since the epoch.
    pub due_at: i64,
    /// Whether `due_at` is a moment somebody chose rather than a day.
    pub timed: bool,
    pub reminded_at: Option<i64>,
}

const COLUMNS: &str = "id, project_id, title, due_at, due_time, reminded_at";

fn row(found: &rusqlite::Row<'_>) -> rusqlite::Result<ReminderRow> {
    Ok(ReminderRow {
        card_id: found.get(0)?,
        project_id: found.get(1)?,
        title: found.get(2)?,
        due_at: found.get(3)?,
        timed: found.get(4)?,
        reminded_at: found.get(5)?,
    })
}

impl Store {
    /// When the next reminder nobody has had yet goes off.
    pub fn next_reminder(&self) -> Result<Option<i64>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT MIN(due_at) FROM card \
                 WHERE due_at IS NOT NULL AND archived_at IS NULL AND reminded_at IS NULL",
                [],
                |found| found.get::<_, Option<i64>>(0),
            )
            .optional()?
            .flatten())
    }

    /// The reminders that have come due by `at` and have not gone off.
    pub fn reminders_due(&self, at: i64) -> Result<Vec<ReminderRow>, StoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM card \
             WHERE due_at IS NOT NULL AND archived_at IS NULL AND reminded_at IS NULL \
             AND due_at <= ?1 ORDER BY due_at"
        ))?;
        let rows = stmt.query_map([at], row)?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Says the reminder for a card's date as it stands went off at `at`.
    pub fn mark_reminded(&self, card_id: &str, at: i64) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE card SET reminded_at = ?2 WHERE id = ?1",
            rusqlite::params![card_id, at],
        )?;
        Ok(())
    }

    /// The reminders that went off and nobody has dealt with, oldest first.
    pub fn reminders_pending(&self) -> Result<Vec<ReminderRow>, StoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM card \
             WHERE due_at IS NOT NULL AND archived_at IS NULL \
             AND reminded_at IS NOT NULL AND handled_at IS NULL ORDER BY due_at"
        ))?;
        let rows = stmt.query_map([], row)?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Every date not yet dealt with — gone off or still to come — in one
    /// project or all of them, soonest first.
    pub fn reminders_open(&self, project_id: Option<&str>) -> Result<Vec<ReminderRow>, StoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM card \
             WHERE due_at IS NOT NULL AND archived_at IS NULL AND handled_at IS NULL \
             AND (?1 IS NULL OR project_id = ?1) ORDER BY due_at"
        ))?;
        let rows = stmt
            .query_map([project_id], row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// The person dealt with a card's reminder. Its date stays: it is still
    /// the card's date, and the board still shows it.
    pub fn handle_reminder(&self, card_id: &str, at: i64) -> Result<bool, StoreError> {
        let changed = self.conn.execute(
            "UPDATE card SET handled_at = ?2, reminded_at = COALESCE(reminded_at, ?2) \
             WHERE id = ?1 AND archived_at IS NULL AND due_at IS NOT NULL",
            rusqlite::params![card_id, at],
        )?;
        Ok(changed > 0)
    }

    /// Marks read what the bell holds about a card's reminder: dealt with on
    /// the banner, it is not news in the bell either.
    pub fn read_reminder_notices(
        &self,
        card_id: &str,
        kind: &str,
        at: i64,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE notice SET read_at = ?3 WHERE card_id = ?1 AND kind = ?2 AND read_at IS NULL",
            rusqlite::params![card_id, kind, at],
        )?;
        Ok(())
    }

    /// One card's reminder, whatever state it is in.
    pub fn reminder(&self, card_id: &str) -> Result<Option<ReminderRow>, StoreError> {
        Ok(self
            .conn
            .query_row(
                &format!(
                    "SELECT {COLUMNS} FROM card WHERE id = ?1 AND due_at IS NOT NULL \
                     AND archived_at IS NULL"
                ),
                [card_id],
                row,
            )
            .optional()?)
    }
}

#[cfg(test)]
#[path = "reminders_tests.rs"]
mod tests;
