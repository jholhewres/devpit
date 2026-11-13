//! Lessons closed cards proposed, waiting for the person's approve, edit or
//! discard.

use serde::{Deserialize, Serialize};
use specta::Type;

/// One lesson a closed card proposed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LessonProposal {
    pub project_id: String,
    pub project: String,
    /// The day it was proposed, as YYYY-MM-DD.
    pub day: String,
    /// The card it came from, as `#12`.
    pub source: String,
    pub text: String,
}

/// A list, so tomorrow's field has somewhere to go.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LessonProposals {
    pub proposals: Vec<LessonProposal>,
}
