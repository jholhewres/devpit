//! What a run actually ran, and what it left behind to prove it.
//!
//! Two questions a green row could not answer before: *green on which code*,
//! and *green because of what*. The first is the snapshot — the command, the
//! directory, the revision, whether it had a checkout of its own. The second
//! is the evidence, a versioned payload rather than a table of checks and a
//! table of cases and a table of artefacts. That shape was considered and
//! turned down: normalising means deciding retention and indexing before
//! anybody has used this once, and a payload can become tables later while
//! tables cannot become a payload.
//!
//! Every row written before this existed has all of it NULL, and reads back as
//! [`Ran::unknown`]. Nothing here ever infers a snapshot from an output.

use rusqlite::OptionalExtension;

use crate::store::{Store, StoreError};

/// The most evidence one run may keep.
///
/// Checked before the payload is read rather than after: a store edited
/// outside the app could hold a gigabyte on one row, and `SELECT evidence`
/// would have allocated it before anything could object.
pub const MOST_EVIDENCE: usize = 1024 * 1024;

/// The circumstances a run happened in.
///
/// Every field optional because every one of them is `unknown` for a row from
/// before migration 017 — and `unknown` is what the screen says, never a guess
/// dressed as a fact.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ran {
    pub command: Option<String>,
    /// The directory the run worked in.
    pub in_directory: Option<String>,
    /// The **names** of the environment devpit declared for it, never the
    /// values: a snapshot of a step's environment is a snapshot of whatever
    /// secret was in it.
    pub declared_env: Vec<String>,
    pub base_revision: Option<String>,
    pub head_revision: Option<String>,
    /// A digest of everything uncommitted when the run started. Together with
    /// `head_revision` this is what says, later, whether a result is still
    /// about the code in front of somebody.
    pub saw_changes: Option<String>,
    /// `None` for a run from before this was recorded.
    pub in_a_worktree: Option<bool>,
}

impl Ran {
    /// What is known about a run nothing recorded: nothing.
    pub fn unknown() -> Self {
        Self::default()
    }

    /// Whether anything at all was recorded. A screen showing a run with none
    /// of this says so rather than showing blanks.
    pub fn is_known(&self) -> bool {
        self != &Self::unknown()
    }
}

/// The evidence a run left, and which shape it is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    /// The payload's shape. A reader that does not know this number says so
    /// instead of reading the payload as though it were the shape it knows.
    pub version: i64,
    pub payload: String,
}

/// Why a run's evidence could not be kept or read.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EvidenceError {
    #[error("that evidence is {bytes} bytes, past the {MOST_EVIDENCE} a run may keep")]
    TooMuch { bytes: usize },
}

impl Store {
    /// Records what a run ran, once it is known.
    ///
    /// Separate from `start_run` because the command and the directory are
    /// settled after the row exists — the checkout is made, the branch is
    /// read — and a row that waits for them is a run the window cannot show
    /// as running.
    pub fn record_what_ran(&self, run_id: &str, ran: &Ran) -> Result<(), StoreError> {
        let declared = if ran.declared_env.is_empty() {
            None
        } else {
            Some(ran.declared_env.join("\n"))
        };
        self.conn.execute(
            "UPDATE run SET ran_command = ?2, ran_in = ?3, declared_env = ?4, \
             base_revision = ?5, head_revision = ?6, in_a_worktree = ?7, saw_changes = ?8 \
             WHERE id = ?1",
            rusqlite::params![
                run_id,
                ran.command,
                ran.in_directory,
                declared,
                ran.base_revision,
                ran.head_revision,
                ran.in_a_worktree.map(i64::from),
                ran.saw_changes,
            ],
        )?;
        Ok(())
    }

    /// What a run ran, or [`Ran::unknown`] for a row that never said.
    pub fn what_ran(&self, run_id: &str) -> Result<Option<Ran>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT ran_command, ran_in, declared_env, base_revision, head_revision, \
                 in_a_worktree, saw_changes FROM run WHERE id = ?1",
                [run_id],
                |row| {
                    let declared: Option<String> = row.get(2)?;
                    Ok(Ran {
                        command: row.get(0)?,
                        in_directory: row.get(1)?,
                        declared_env: declared
                            .unwrap_or_default()
                            .lines()
                            .filter(|name| !name.is_empty())
                            .map(str::to_owned)
                            .collect(),
                        base_revision: row.get(3)?,
                        head_revision: row.get(4)?,
                        in_a_worktree: row.get::<_, Option<i64>>(5)?.map(|kept| kept != 0),
                        saw_changes: row.get(6)?,
                    })
                },
            )
            .optional()?)
    }

    /// Keeps a run's evidence, refusing more than [`MOST_EVIDENCE`].
    ///
    /// The refusal comes before the write and names the size, so a step that
    /// produces too much is a step somebody can shorten rather than a payload
    /// silently truncated into something that parses and lies.
    pub fn record_evidence(&self, run_id: &str, evidence: &Evidence) -> Result<(), EvidenceError> {
        if evidence.payload.len() > MOST_EVIDENCE {
            return Err(EvidenceError::TooMuch {
                bytes: evidence.payload.len(),
            });
        }
        let _ = self.conn.execute(
            "UPDATE run SET evidence = ?2, evidence_version = ?3 WHERE id = ?1",
            rusqlite::params![run_id, evidence.payload, evidence.version],
        );
        Ok(())
    }

    /// A run's evidence, or `None` when it left none.
    ///
    /// The size is asked of SQLite first and the payload only fetched if it
    /// fits: `length()` reads the row's header, and a row too big to hold is
    /// refused before a byte of it is allocated here.
    pub fn evidence_of(&self, run_id: &str) -> Result<Option<Evidence>, StoreError> {
        let measured: Option<(Option<i64>, Option<i64>)> = self
            .conn
            .query_row(
                "SELECT length(evidence), evidence_version FROM run WHERE id = ?1",
                [run_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((Some(bytes), version)) = measured else {
            return Ok(None);
        };
        if bytes as usize > MOST_EVIDENCE {
            return Ok(None);
        }
        let payload: String =
            self.conn
                .query_row("SELECT evidence FROM run WHERE id = ?1", [run_id], |row| {
                    row.get(0)
                })?;
        Ok(Some(Evidence {
            version: version.unwrap_or_default(),
            payload,
        }))
    }
}

#[cfg(test)]
#[path = "evidence_tests.rs"]
mod tests;
