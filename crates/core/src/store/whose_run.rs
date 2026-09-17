//! Who asked for a run, and what carried it out.
//!
//! Two questions, not one. Claude can ask for a test that the local runner
//! executes and that Codex then reviews — three parties and one run — and a
//! single "who" column would have collapsed all of it into whichever was
//! written last.
//!
//! Both are written when the run **starts**. That is the whole discipline
//! here: an event arriving after the fact must not reattribute a run to
//! whatever happens to be active by the time it lands, and the only way to be
//! sure of that is to have nothing that writes these later.
//!
//! Every run from before migration 019 has both NULL, which reads back as
//! [`Asked::Unknown`] and [`Carried::Unknown`] — never as a guess.

use rusqlite::OptionalExtension;

use crate::store::{Store, StoreError};

/// The surface a run was asked for from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    /// A card dragged into a lane that runs something.
    Board,
    /// The card's own play, where it stands.
    Card,
    /// The checkpoint, which never moves the card.
    Checkpoint,
    /// The lane before it, passing the card on.
    Chain,
    /// Nothing recorded it. Every run from before migration 019.
    Unknown,
}

impl Asked {
    fn word(&self) -> Option<&'static str> {
        match self {
            Self::Board => Some("board"),
            Self::Card => Some("card"),
            Self::Checkpoint => Some("checkpoint"),
            Self::Chain => Some("chain"),
            Self::Unknown => None,
        }
    }

    fn from_word(word: &str) -> Self {
        match word {
            "board" => Self::Board,
            "card" => Self::Card,
            "checkpoint" => Self::Checkpoint,
            "chain" => Self::Chain,
            // A word this build does not know is not a word to invent a
            // meaning for.
            _ => Self::Unknown,
        }
    }
}

/// What actually did the work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Carried {
    /// A command devpit started on this machine.
    Process,
    /// An agent session, under the profile named.
    Agent {
        profile: Option<String>,
    },
    Unknown,
}

/// A run's origin and its executor, together because they are read together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhoseRun {
    pub asked: Asked,
    /// The id of the surface that asked — a tab, a pane, a session.
    ///
    /// A **reference**, never a name: read back, a terminal that has closed is
    /// unavailable, and finding another pane that happens to share its name
    /// would point somebody at work that is not theirs.
    pub asked_from: Option<String>,
    pub carried: Carried,
}

impl WhoseRun {
    pub fn unknown() -> Self {
        Self {
            asked: Asked::Unknown,
            asked_from: None,
            carried: Carried::Unknown,
        }
    }

    pub fn is_known(&self) -> bool {
        self != &Self::unknown()
    }
}

impl Store {
    /// Records who asked and what carried it out, at the moment the run opens.
    ///
    /// There is deliberately no second function that sets these later.
    pub fn record_whose_run(&self, run_id: &str, whose: &WhoseRun) -> Result<(), StoreError> {
        let (carried, profile) = match &whose.carried {
            Carried::Process => (Some("process"), None),
            Carried::Agent { profile } => (Some("agent"), profile.clone()),
            Carried::Unknown => (None, None),
        };
        self.conn.execute(
            "UPDATE run SET asked_by = ?2, asked_from = ?3, carried_by = ?4, \
             carried_profile = ?5 WHERE id = ?1",
            rusqlite::params![
                run_id,
                whose.asked.word(),
                whose.asked_from,
                carried,
                profile
            ],
        )?;
        Ok(())
    }

    /// Whose a run was, or [`WhoseRun::unknown`] for a row that never said.
    pub fn whose_run(&self, run_id: &str) -> Result<Option<WhoseRun>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT asked_by, asked_from, carried_by, carried_profile FROM run \
                 WHERE id = ?1",
                [run_id],
                |row| {
                    let asked: Option<String> = row.get(0)?;
                    let carried: Option<String> = row.get(2)?;
                    let profile: Option<String> = row.get(3)?;
                    Ok(WhoseRun {
                        asked: asked
                            .as_deref()
                            .map(Asked::from_word)
                            .unwrap_or(Asked::Unknown),
                        asked_from: row.get(1)?,
                        carried: match carried.as_deref() {
                            Some("process") => Carried::Process,
                            Some("agent") => Carried::Agent { profile },
                            _ => Carried::Unknown,
                        },
                    })
                },
            )
            .optional()?)
    }
}

#[cfg(test)]
#[path = "whose_run_tests.rs"]
mod tests;
