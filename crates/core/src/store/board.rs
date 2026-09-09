//! Reading and writing the board: columns, cards, steps and runs.
//!
//! The board belongs to the project. A column is data — created, renamed,
//! reordered and deleted by the person using it — so nothing here matches on
//! a column name, and the default board below is a seed, not a contract.

use rusqlite::OptionalExtension;

use crate::store::{Store, StoreError};

pub struct ColumnRow {
    pub id: String,
    pub name: String,
    pub position: i64,
    pub step_id: Option<String>,
}

pub struct CardRow {
    pub id: String,
    pub column_id: String,
    pub title: String,
    pub body: String,
    pub position: i64,
    pub worktree_path: Option<String>,
    pub base_ref: Option<String>,
}

pub struct StepRow {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub config: String,
    pub irreversible: bool,
}

pub struct RunRow {
    pub id: String,
    pub card_id: String,
    pub step_id: String,
    pub state: String,
    pub output: Option<String>,
    pub exit_code: Option<i64>,
    pub cost_usd: Option<f64>,
    pub duration_ms: Option<i64>,
    pub started_at: i64,
}

/// The board a new project starts with.
///
/// A seed, not a contract: these names are as editable as any the person
/// types later, and a test asserts no query matches on them.
pub const DEFAULT_COLUMNS: [&str; 6] = ["inbox", "refine", "review", "doing", "check", "ship"];

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}

impl Store {
    /// The project's columns, left to right.
    pub fn columns(&self, project_id: &str) -> Result<Vec<ColumnRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, position, step_id FROM board_column \
             WHERE project_id = ?1 ORDER BY position",
        )?;
        let rows = stmt
            .query_map([project_id], |row| {
                Ok(ColumnRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    position: row.get(2)?,
                    step_id: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Creates the default board, once. Later calls find columns and stop.
    pub fn ensure_board(&self, project_id: &str) -> Result<(), StoreError> {
        if !self.columns(project_id)?.is_empty() {
            return Ok(());
        }
        for (position, name) in DEFAULT_COLUMNS.iter().enumerate() {
            self.create_column(project_id, name, position as i64)?;
        }
        Ok(())
    }

    pub fn create_column(
        &self,
        project_id: &str,
        name: &str,
        position: i64,
    ) -> Result<String, StoreError> {
        let id = format!("col_{}", ulid::Ulid::generate());
        self.conn.execute(
            "INSERT INTO board_column (id, project_id, name, position, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, project_id, name, position, now()],
        )?;
        Ok(id)
    }

    pub fn rename_column(&self, column_id: &str, name: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE board_column SET name = ?2 WHERE id = ?1",
            rusqlite::params![column_id, name],
        )?;
        Ok(())
    }

    /// Writes a whole left-to-right order at once.
    ///
    /// One statement per column would trip the uniqueness of
    /// (project_id, position) halfway through a reorder, so the positions go
    /// negative first and come back in the order given.
    pub fn reorder_columns(&self, project_id: &str, ids: &[String]) -> Result<(), StoreError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE board_column SET position = -position - 1 WHERE project_id = ?1",
            [project_id],
        )?;
        for (position, id) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE board_column SET position = ?2 WHERE id = ?1 AND project_id = ?3",
                rusqlite::params![id, position as i64, project_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Deleting a column that still holds cards fails, by design: the
    /// interface has to ask where they go.
    pub fn delete_column(&self, column_id: &str) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM board_column WHERE id = ?1", [column_id])?;
        Ok(())
    }

    pub fn set_column_step(
        &self,
        column_id: &str,
        step_id: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE board_column SET step_id = ?2 WHERE id = ?1",
            rusqlite::params![column_id, step_id],
        )?;
        Ok(())
    }

    /// Every card of a project, by column and then by position.
    pub fn cards(&self, project_id: &str) -> Result<Vec<CardRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, column_id, title, body, position, worktree_path, base_ref \
             FROM card WHERE project_id = ?1 AND archived_at IS NULL \
             ORDER BY column_id, position",
        )?;
        let rows = stmt
            .query_map([project_id], |row| {
                Ok(CardRow {
                    id: row.get(0)?,
                    column_id: row.get(1)?,
                    title: row.get(2)?,
                    body: row.get(3)?,
                    position: row.get(4)?,
                    worktree_path: row.get(5)?,
                    base_ref: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn card(&self, card_id: &str) -> Result<Option<CardRow>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, column_id, title, body, position, worktree_path, base_ref \
                 FROM card WHERE id = ?1",
                [card_id],
                |row| {
                    Ok(CardRow {
                        id: row.get(0)?,
                        column_id: row.get(1)?,
                        title: row.get(2)?,
                        body: row.get(3)?,
                        position: row.get(4)?,
                        worktree_path: row.get(5)?,
                        base_ref: row.get(6)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn create_card(
        &self,
        project_id: &str,
        column_id: &str,
        title: &str,
        body: &str,
    ) -> Result<String, StoreError> {
        let id = format!("card_{}", ulid::Ulid::generate());
        let position: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM card WHERE column_id = ?1",
            [column_id],
            |row| row.get(0),
        )?;
        let at = now();
        self.conn.execute(
            "INSERT INTO card \
             (id, project_id, column_id, title, body, position, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            rusqlite::params![id, project_id, column_id, title, body, position, at],
        )?;
        Ok(id)
    }

    pub fn update_card(&self, card_id: &str, title: &str, body: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE card SET title = ?2, body = ?3, updated_at = ?4 WHERE id = ?1",
            rusqlite::params![card_id, title, body, now()],
        )?;
        Ok(())
    }

    pub fn move_card(
        &self,
        card_id: &str,
        column_id: &str,
        position: i64,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE card SET column_id = ?2, position = ?3, updated_at = ?4 WHERE id = ?1",
            rusqlite::params![card_id, column_id, position, now()],
        )?;
        Ok(())
    }

    pub fn archive_card(&self, card_id: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE card SET archived_at = ?2 WHERE id = ?1",
            rusqlite::params![card_id, now()],
        )?;
        Ok(())
    }

    pub fn steps(&self, project_id: &str) -> Result<Vec<StepRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, name, config, irreversible FROM step \
             WHERE project_id = ?1 ORDER BY name",
        )?;
        let rows = stmt
            .query_map([project_id], |row| {
                Ok(StepRow {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    name: row.get(2)?,
                    config: row.get(3)?,
                    irreversible: row.get::<_, i64>(4)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn step(&self, step_id: &str) -> Result<Option<StepRow>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, kind, name, config, irreversible FROM step WHERE id = ?1",
                [step_id],
                |row| {
                    Ok(StepRow {
                        id: row.get(0)?,
                        kind: row.get(1)?,
                        name: row.get(2)?,
                        config: row.get(3)?,
                        irreversible: row.get::<_, i64>(4)? != 0,
                    })
                },
            )
            .optional()?)
    }

    pub fn create_step(
        &self,
        project_id: &str,
        kind: &str,
        name: &str,
        config: &str,
        irreversible: bool,
    ) -> Result<String, StoreError> {
        let id = format!("step_{}", ulid::Ulid::generate());
        self.conn.execute(
            "INSERT INTO step (id, project_id, kind, name, config, irreversible, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                id,
                project_id,
                kind,
                name,
                config,
                irreversible as i64,
                now()
            ],
        )?;
        Ok(id)
    }

    /// Opens a run in `running`. It is closed by `finish_run`.
    pub fn start_run(&self, card_id: &str, step_id: &str) -> Result<String, StoreError> {
        let id = format!("run_{}", ulid::Ulid::generate());
        self.conn.execute(
            "INSERT INTO run (id, card_id, step_id, state, started_at) \
             VALUES (?1, ?2, ?3, 'running', ?4)",
            rusqlite::params![id, card_id, step_id, now()],
        )?;
        Ok(id)
    }

    pub fn finish_run(
        &self,
        run_id: &str,
        state: &str,
        output: Option<&str>,
        cost_usd: Option<f64>,
        duration_ms: Option<i64>,
        exit_code: Option<i64>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE run SET state = ?2, output = ?3, cost_usd = ?4, duration_ms = ?5, \
             exit_code = ?6, ended_at = ?7 WHERE id = ?1",
            rusqlite::params![
                run_id,
                state,
                output,
                cost_usd,
                duration_ms,
                exit_code,
                now()
            ],
        )?;
        Ok(())
    }

    /// A card's runs, most recent first.
    pub fn runs(&self, card_id: &str) -> Result<Vec<RunRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, card_id, step_id, state, output, exit_code, cost_usd, duration_ms, \
             started_at FROM run WHERE card_id = ?1 ORDER BY started_at DESC",
        )?;
        let rows = stmt
            .query_map([card_id], |row| {
                Ok(RunRow {
                    id: row.get(0)?,
                    card_id: row.get(1)?,
                    step_id: row.get(2)?,
                    state: row.get(3)?,
                    output: row.get(4)?,
                    exit_code: row.get(5)?,
                    cost_usd: row.get(6)?,
                    duration_ms: row.get(7)?,
                    started_at: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// The checkout this card's work happens in, and where it began.
    ///
    /// Both or neither: a worktree with no base ref cannot answer "what
    /// changed here", which is the only question it exists to answer.
    pub fn set_card_front(
        &self,
        card_id: &str,
        worktree_path: Option<&str>,
        base_ref: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE card SET worktree_path = ?2, base_ref = ?3, updated_at = ?4 WHERE id = ?1",
            rusqlite::params![card_id, worktree_path, base_ref, now()],
        )?;
        Ok(())
    }

    /// How many cards a column holds. Where a returning card lands.
    pub fn cards_in_column(&self, column_id: &str) -> Result<i64, StoreError> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM card WHERE column_id = ?1 AND archived_at IS NULL",
            [column_id],
            |row| row.get(0),
        )?)
    }

    /// Appends a line to a card's body.
    ///
    /// A card sent back has to say why on the card itself. Putting the reason
    /// only in the run would make the board show a card that moved for no
    /// visible cause.
    pub fn note_on_card(&self, card_id: &str, line: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE card SET body = CASE WHEN body = '' THEN ?2 \
             ELSE body || char(10) || char(10) || ?2 END, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![card_id, line, now()],
        )?;
        Ok(())
    }

    /// The root path of the project a card belongs to.
    ///
    /// A session is started in the project's directory, and the card knows its
    /// project — asking the caller to carry the path would let the two drift.
    pub fn project_of_card(&self, card_id: &str) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT p.root_path FROM card c JOIN project p ON p.id = c.project_id \
                 WHERE c.id = ?1",
                [card_id],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// The id of the project a card belongs to.
    ///
    /// Separate from `project_of_card`, which answers with the path: the id is
    /// what names a folder in the devpit workspace, and the path is what git
    /// is run in.
    pub fn project_id_of_card(&self, card_id: &str) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT project_id FROM card WHERE id = ?1",
                [card_id],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// What a card has cost across every run of it.
    pub fn card_cost(&self, card_id: &str) -> Result<f64, StoreError> {
        Ok(self.conn.query_row(
            "SELECT COALESCE(SUM(cost_usd), 0.0) FROM run WHERE card_id = ?1",
            [card_id],
            |row| row.get(0),
        )?)
    }
}

/// The link between a card and the agent session working on it.
///
/// Only the link. Whether that session is idle or busy, and what it has spent,
/// are answered by the agent CLI and its transcript — a copy here would be a
/// second truth that drifts from the first.
pub struct SessionLink {
    pub card_id: String,
    pub short_id: String,
    pub session_id: String,
    pub transcript_path: Option<String>,
}

impl Store {
    pub fn link_session(
        &self,
        card_id: &str,
        short_id: &str,
        session_id: &str,
        transcript_path: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO session_link \
             (card_id, short_id, session_id, transcript_path, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(card_id) DO UPDATE SET \
               short_id = ?2, session_id = ?3, transcript_path = ?4",
            rusqlite::params![card_id, short_id, session_id, transcript_path, now()],
        )?;
        Ok(())
    }

    pub fn session_link(&self, card_id: &str) -> Result<Option<SessionLink>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT card_id, short_id, session_id, transcript_path \
                 FROM session_link WHERE card_id = ?1",
                [card_id],
                |row| {
                    Ok(SessionLink {
                        card_id: row.get(0)?,
                        short_id: row.get(1)?,
                        session_id: row.get(2)?,
                        transcript_path: row.get(3)?,
                    })
                },
            )
            .optional()?)
    }

    /// Every card of a project that has a session behind it.
    pub fn session_links(&self, project_id: &str) -> Result<Vec<SessionLink>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT l.card_id, l.short_id, l.session_id, l.transcript_path \
             FROM session_link l JOIN card c ON c.id = l.card_id \
             WHERE c.project_id = ?1",
        )?;
        let rows = stmt
            .query_map([project_id], |row| {
                Ok(SessionLink {
                    card_id: row.get(0)?,
                    short_id: row.get(1)?,
                    session_id: row.get(2)?,
                    transcript_path: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
#[path = "board_tests.rs"]
mod tests;
