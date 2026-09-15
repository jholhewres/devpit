//! What a card carries besides a title: a deadline, a conversation, the files
//! somebody pinned to it — and what the bell has to show.
//!
//! Apart from `board.rs` because that file owns the shape of the board —
//! columns, positions, the drag — and these are about one card at a time.
//! They also arrive together on purpose: each is a table whose rows hang off a
//! card and go with it, and the cascade is the only rule they share.

use rusqlite::OptionalExtension;

use crate::store::{Store, StoreError};

pub struct CommentRow {
    pub id: String,
    /// `you`, or an agent id. Never a display name — those change, and a
    /// screen would have to trust one it was handed.
    pub author: String,
    pub body: String,
    pub created_at: i64,
    pub edited_at: Option<i64>,
}

pub struct AttachmentRow {
    pub id: String,
    /// Absolute, and local only — like `project.root_path`, and for the same
    /// reason: it says nothing on another machine.
    pub path: String,
    pub label: String,
    pub created_at: i64,
    /// The plugin whose file this is, when that plugin pinned it.
    pub plugin_id: Option<String>,
}

pub struct NoticeRow {
    pub id: String,
    pub project_id: Option<String>,
    pub kind: String,
    pub title: String,
    pub detail: Option<String>,
    pub card_id: Option<String>,
    pub created_at: i64,
    pub read_at: Option<i64>,
}

/// A ceiling on what one read can allocate.
///
/// Comments are typed by a person and written by an agent, and the second one
/// has no natural end. Applied on the way in, so the row on disk is already
/// within it and no read has to trust its own table.
pub const LONGEST_COMMENT: usize = 16 * 1024;
pub const LONGEST_LABEL: usize = 200;
/// How many notices the bell keeps. Older ones are dropped on write rather
/// than on read: a list nobody trims is a list that is one day a megabyte.
pub const NOTICES_KEPT: i64 = 200;

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}

impl Store {
    /// When this card is due, or `None` to clear it.
    pub fn set_card_due(&self, card_id: &str, due_at: Option<i64>) -> Result<bool, StoreError> {
        let changed = self.conn.execute(
            "UPDATE card SET due_at = ?2, updated_at = ?3 WHERE id = ?1 AND archived_at IS NULL",
            rusqlite::params![card_id, due_at, now()],
        )?;
        Ok(changed > 0)
    }

    // ── The conversation ────────────────────────────────────────────────

    pub fn comments(&self, card_id: &str) -> Result<Vec<CommentRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, author, body, created_at, edited_at FROM card_comment \
             WHERE card_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt
            .query_map([card_id], |row| {
                Ok(CommentRow {
                    id: row.get(0)?,
                    author: row.get(1)?,
                    body: row.get(2)?,
                    created_at: row.get(3)?,
                    edited_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn add_comment(
        &self,
        card_id: &str,
        author: &str,
        body: &str,
    ) -> Result<String, StoreError> {
        let id = format!("cmt_{}", ulid::Ulid::generate());
        self.conn.execute(
            "INSERT INTO card_comment (id, card_id, author, body, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, card_id, author, body, now()],
        )?;
        Ok(id)
    }

    /// Edits one, recording that it was edited.
    ///
    /// Recorded rather than silent: a conversation where a line can change
    /// under you with no mark is a conversation you cannot rely on.
    pub fn edit_comment(&self, comment_id: &str, body: &str) -> Result<bool, StoreError> {
        let changed = self.conn.execute(
            "UPDATE card_comment SET body = ?2, edited_at = ?3 WHERE id = ?1",
            rusqlite::params![comment_id, body, now()],
        )?;
        Ok(changed > 0)
    }

    pub fn delete_comment(&self, comment_id: &str) -> Result<bool, StoreError> {
        let changed = self
            .conn
            .execute("DELETE FROM card_comment WHERE id = ?1", [comment_id])?;
        Ok(changed > 0)
    }

    // ── The files pinned to it ──────────────────────────────────────────

    pub fn attachments(&self, card_id: &str) -> Result<Vec<AttachmentRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, label, created_at, plugin_id FROM card_attachment \
             WHERE card_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt
            .query_map([card_id], |row| {
                Ok(AttachmentRow {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    label: row.get(2)?,
                    created_at: row.get(3)?,
                    plugin_id: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Pins a file, naming the plugin that pinned it if one did. The same path
    /// twice is one attachment, not two.
    pub fn attach(
        &self,
        card_id: &str,
        path: &str,
        label: &str,
        plugin_id: Option<&str>,
    ) -> Result<String, StoreError> {
        if let Some(had) = self
            .conn
            .query_row(
                "SELECT id FROM card_attachment WHERE card_id = ?1 AND path = ?2",
                rusqlite::params![card_id, path],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            // A file pinned by hand and then by its plugin opens in the plugin.
            if plugin_id.is_some() {
                self.conn.execute(
                    "UPDATE card_attachment SET plugin_id = ?2 WHERE id = ?1",
                    rusqlite::params![had, plugin_id],
                )?;
            }
            return Ok(had);
        }

        let id = format!("att_{}", ulid::Ulid::generate());
        self.conn.execute(
            "INSERT INTO card_attachment (id, card_id, path, label, created_at, plugin_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, card_id, path, label, now(), plugin_id],
        )?;
        Ok(id)
    }

    /// Unpins one. The file on disk is not touched — this is the pin.
    pub fn detach(&self, attachment_id: &str) -> Result<bool, StoreError> {
        let changed = self
            .conn
            .execute("DELETE FROM card_attachment WHERE id = ?1", [attachment_id])?;
        Ok(changed > 0)
    }

    // ── What the bell shows ─────────────────────────────────────────────

    /// Newest first, with the id as the tiebreak.
    ///
    /// `created_at` is seconds, and a burst — five runs finishing at once —
    /// lands entirely within one of them. Ordering on the timestamp alone
    /// left those five in whatever order SQLite felt like, which is the same
    /// list drawn differently on every read. The id is a ULID, ordered by the
    /// millisecond it was minted in, so it breaks the tie the right way.
    pub fn notices(&self, limit: i64) -> Result<Vec<NoticeRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, kind, title, detail, card_id, created_at, read_at \
             FROM notice ORDER BY created_at DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map([limit], |row| {
                Ok(NoticeRow {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    kind: row.get(2)?,
                    title: row.get(3)?,
                    detail: row.get(4)?,
                    card_id: row.get(5)?,
                    created_at: row.get(6)?,
                    read_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn add_notice(
        &self,
        project_id: Option<&str>,
        kind: &str,
        title: &str,
        detail: Option<&str>,
        card_id: Option<&str>,
    ) -> Result<String, StoreError> {
        Self::add_notice_on(&self.conn, project_id, kind, title, detail, card_id)
    }

    /// `add_notice` for work holding a connection but no store: a migration,
    /// inside its own transaction.
    pub(crate) fn add_notice_on(
        conn: &rusqlite::Connection,
        project_id: Option<&str>,
        kind: &str,
        title: &str,
        detail: Option<&str>,
        card_id: Option<&str>,
    ) -> Result<String, StoreError> {
        let id = format!("ntc_{}", ulid::Ulid::generate());
        conn.execute(
            "INSERT INTO notice (id, project_id, kind, title, detail, card_id, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, project_id, kind, title, detail, card_id, now()],
        )?;

        // Trimmed on write, not on read: the read is what a person waits for.
        // Same tiebreak as the read, or the trim would keep a different 200.
        conn.execute(
            "DELETE FROM notice WHERE id NOT IN \
             (SELECT id FROM notice ORDER BY created_at DESC, id DESC LIMIT ?1)",
            [NOTICES_KEPT],
        )?;
        Ok(id)
    }

    /// Marks one read. Reading is per notice — nothing here expires on its own.
    pub fn read_notice(&self, notice_id: &str) -> Result<bool, StoreError> {
        let changed = self.conn.execute(
            "UPDATE notice SET read_at = ?2 WHERE id = ?1 AND read_at IS NULL",
            rusqlite::params![notice_id, now()],
        )?;
        Ok(changed > 0)
    }

    pub fn read_all_notices(&self) -> Result<u32, StoreError> {
        let changed = self.conn.execute(
            "UPDATE notice SET read_at = ?1 WHERE read_at IS NULL",
            [now()],
        )?;
        Ok(changed as u32)
    }

    pub fn unread_notices(&self) -> Result<u32, StoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM notice WHERE read_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(count as u32)
    }

    /// Cards past their deadline, for whatever is going to say so.
    pub fn overdue_cards(&self, at: i64) -> Result<Vec<(String, String, String)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, title FROM card \
             WHERE archived_at IS NULL AND due_at IS NOT NULL AND due_at <= ?1",
        )?;
        let rows = stmt
            .query_map([at], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
#[path = "cards_tests.rs"]
mod tests;

/// A card off the board, as the Archived list shows it.
pub struct ArchivedRow {
    pub id: String,
    pub title: String,
    /// `None` only for a lane that went while the card was archived.
    pub column_name: Option<String>,
    pub archived_at: i64,
}

impl Store {
    /// A project's archived cards, most recently archived first.
    pub fn archived_cards(
        &self,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<ArchivedRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.title, col.name, c.archived_at FROM card c \
             LEFT JOIN board_column col ON col.id = c.column_id \
             WHERE c.project_id = ?1 AND c.archived_at IS NOT NULL \
             ORDER BY c.archived_at DESC, c.id DESC LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![project_id, limit], |row| {
                Ok(ArchivedRow {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    column_name: row.get(2)?,
                    archived_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Back on the board at the end of its lane, or of the first lane when its
    /// own is gone. Answers false for a card that is not archived.
    pub fn restore_card(&self, card_id: &str) -> Result<bool, StoreError> {
        let Some((project, column)) = self
            .conn
            .query_row(
                "SELECT project_id, column_id FROM card WHERE id = ?1 AND archived_at IS NOT NULL",
                [card_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
        else {
            return Ok(false);
        };
        let lane: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM board_column WHERE id = ?1 \
                 UNION ALL SELECT * FROM \
                 (SELECT id FROM board_column WHERE project_id = ?2 ORDER BY position LIMIT 1) \
                 LIMIT 1",
                rusqlite::params![column, project],
                |row| row.get(0),
            )
            .optional()?;
        let Some(lane) = lane else {
            return Ok(false);
        };
        let position: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM card \
             WHERE column_id = ?1 AND archived_at IS NULL",
            [&lane],
            |row| row.get(0),
        )?;
        self.conn.execute(
            "UPDATE card SET archived_at = NULL, column_id = ?2, position = ?3, updated_at = ?4 \
             WHERE id = ?1",
            rusqlite::params![card_id, lane, position, now()],
        )?;
        Ok(true)
    }
}
