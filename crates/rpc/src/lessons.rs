//! What the project agent proposed for its memory, waiting for the person's
//! approve, edit or discard, and the decisions that name their cards.

use serde::{Deserialize, Serialize};
use specta::Type;

/// One proposal: a lesson or a decision a closed card left, or the update or
/// removal of a stale entry a re-check found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LessonProposal {
    pub project_id: String,
    pub project: String,
    /// The day it was proposed, as YYYY-MM-DD.
    pub day: String,
    /// The card it came from, as `#12`, or the stale entry's own source.
    pub source: String,
    /// The memory file it goes to: `lessons.md`, `decisions.md` or `facts.md`.
    pub file: String,
    /// What it says; empty when it removes the stale entry it replaces.
    pub text: String,
    /// The stale entry it updates or removes.
    pub replaces: Option<String>,
}

/// A decision of a project's memory, with the cards that made it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDecision {
    pub project_id: String,
    pub day: String,
    /// The cards it names, as `#12`.
    pub cards: Vec<String>,
    /// Its whole source: the cards and the files it rests on.
    pub source: String,
    pub text: String,
}

/// A list, so tomorrow's field has somewhere to go.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LessonProposals {
    pub proposals: Vec<LessonProposal>,
    pub decisions: Vec<MemoryDecision>,
}
